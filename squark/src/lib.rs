use rand::prelude::*;
use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use rustc_hash::FxHashMap;
use std::rc::Rc;
use uuid::Uuid;
use futures::Future;
use serde::Serialize;

mod vdom;

pub use crate::vdom::{Node, Element, Diff, View, HandlerArg, AttributeValue, Child};
use crate::vdom::{HandlerFunction, HandlerMap};

thread_local! {
    static RNG: RefCell<SmallRng> = RefCell::new(SmallRng::from_entropy());
}

pub trait App: 'static + Clone + Default {
    type State: Clone + Debug + PartialEq + 'static;
    type Action: Clone + Debug + 'static;

    fn reducer(&self, state: Self::State, action: Self::Action) -> (Self::State, Vec<Task<Self::Action>>);

    fn view(&self, state: Self::State) -> View<Self::Action>;
}

pub fn handler<A, F>(f: F) -> (String, HandlerFunction<A>)
where
    F: Fn(HandlerArg) -> Option<A> + 'static,
{
    (uuid(), Box::new(f))
}

#[derive(Clone)]
pub struct Env<A: App> {
    app: A,
    state: Rc<RefCell<A::State>>,
    node: Rc<RefCell<Node>>,
    handler_map: Rc<RefCell<HandlerMap<A::Action>>>,
    scheduled: Rc<Cell<bool>>,
}

impl<A: App> Env<A> {
    pub fn new(state: A::State) -> Env<A> {
        Env {
            app: A::default(),
            state: Rc::new(RefCell::new(state)),
            node: Rc::new(RefCell::new(Node::Null)),
            handler_map: Rc::new(RefCell::new(FxHashMap::default())),
            scheduled: Rc::new(Cell::new(false)),
        }
    }

    fn get_state(&self) -> A::State {
        self.state.borrow().to_owned()
    }

    fn set_state(&self, state: A::State) {
        *self.state.borrow_mut() = state;
    }

    fn get_node(&self) -> Node {
        self.node.borrow().to_owned()
    }

    fn set_node(&self, node: Node) {
        *self.node.borrow_mut() = node;
    }

    fn pop_handler(&self, id: &str) -> Option<HandlerFunction<A::Action>> {
        self.handler_map.borrow_mut().remove(id)
    }
}

pub type Task<A> = Box<Future<Item = A, Error = ()>>;

pub trait Runtime<A: App>: Clone + 'static {
    fn get_env<'a>(&'a self) -> &'a Env<A>;

    fn handle_diff(&self, diff: Diff);

    fn handle_future<T: Serialize + 'static, E: Serialize + 'static>(&self, future: Box<Future<Item = T, Error = E>>);

    fn schedule_render(&self);

    fn run(&self) {
        let env = self.get_env();
        env.scheduled.set(false);
        let mut old_node = env.get_node();
        let view = env.app.view(env.get_state());
        *env.handler_map.borrow_mut() = view.handler_map;
        if let Some(diff) = Node::diff(&mut old_node, &view.node, &mut 0) {
            env.set_node(view.node);
            self.handle_diff(diff);
        }
    }

    fn on_action(&self, action: A::Action) {
        let env = self.get_env();

        let old_state = env.get_state();
        let (new_state, tasks) = env.app.reducer(old_state.to_owned(), action);
        for task in tasks {
            self.emit_task(task);
        }
        self.set_state(new_state);
    }

    fn set_state(&self, new_state: A::State) {
        let env = self.get_env();
        let old_state = env.get_state();
        if old_state == new_state {
            return;
        }
        env.set_state(new_state);
        if env.scheduled.get() {
            return;
        }
        env.scheduled.set(true);
        self.schedule_render();
    }

    fn emit_task(&self, task: Box<Future<Item = A::Action, Error = ()>>) {
        let this = self.clone();
        self.handle_future(Box::new(task.map(move |a| {
            this.on_action(a);
        })));
    }

    fn pop_handler(&self, id: &str) -> Option<Box<Fn(HandlerArg)>> {
        let env = self.get_env();
        let handler = env.pop_handler(id)?;
        let this = self.to_owned();
        let f = move |arg: HandlerArg| {
            match handler(arg) {
                Some(a) => this.on_action(a),
                None => return,
            };
        };
        Some(Box::new(f))
    }
}

pub fn uuid() -> String {
    RNG.with(|rng| Uuid::from_random_bytes(rng.borrow_mut().gen()))
        .to_string()
}
