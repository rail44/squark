extern crate rand;
extern crate serde_json;
extern crate uuid;

use std::fmt::Debug;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use rand::{OsRng, RngCore};
use uuid::Uuid;

pub use serde_json::Value as HandlerArg;

type Attributes = Vec<(String, AttributeValue)>;

fn diff_attributes(a: &mut Attributes, b: &Attributes) -> Vec<Diff> {
    let mut result = vec![];

    let mut old_map = HashMap::<String, AttributeValue>::from_iter(a.drain(..));
    for &(ref new_key, ref new_val) in b {
        match old_map.remove(new_key) {
            Some(old_val) => {
                if &old_val != new_val {
                    result.push(Diff::SetAttribute(new_key.clone(), new_val.clone()))
                }
            }
            None => result.push(Diff::SetAttribute(new_key.clone(), new_val.clone())),
        }
    }

    for (old_key, _) in old_map.drain() {
        result.push(Diff::RemoveAttribute(old_key));
    }

    result
}

type HandlerFunction<A> = Box<Fn(HandlerArg) -> Option<A>>;
type Handler = (String, (u64, String));
type Handlers = Vec<Handler>;

fn diff_handlers(a: &mut Handlers, b: &Handlers) -> Vec<Diff> {
    let mut result = vec![];

    let mut old_map = HashMap::<String, (u64, String)>::from_iter(a.drain(..));
    for &(ref new_key, (ref new_hash, ref new_id)) in b {
        match old_map.remove(new_key) {
            Some((old_hash, _)) => {
                if &old_hash != new_hash {
                    result.push(Diff::SetHandler(new_key.clone(), new_id.clone()))
                }
            }
            None => result.push(Diff::SetHandler(new_key.clone(), new_id.clone())),
        }
    }

    for (old_key, (_, old_id)) in old_map.drain() {
        result.push(Diff::RemoveHandler(old_key, old_id));
    }

    result
}

#[derive(Clone, Debug)]
pub enum Node {
    Text(String),
    Element(Element),
    Null,
}

impl Node {
    fn diff(a: &mut Node, b: &Node, i: &mut usize) -> Option<Diff> {
        match (a, b) {
            (&mut Node::Element(ref mut a), &Node::Element(ref b)) => Element::diff(a, b, &i),
            (&mut Node::Text(ref mut text_a), &Node::Text(ref text_b)) => {
                if text_a == text_b {
                    return None;
                }
                Some(Diff::ReplaceChild(i.clone(), b.clone()))
            }
            (&mut Node::Null, &Node::Null) => None,
            (&mut Node::Null, _) => Some(Diff::AddChild(i.clone(), b.clone())),
            (_, &Node::Null) => {
                let j = i.clone();
                *i -= 1;
                Some(Diff::RemoveChild(j))
            }
            _ => Some(Diff::ReplaceChild(i.clone(), b.clone())),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Element {
    pub name: String,
    pub attributes: Attributes,
    pub handlers: Handlers,
    pub children: Vec<Node>,
}

impl Element {
    pub fn new(
        name: String,
        attributes: Attributes,
        handlers: Handlers,
        children: Vec<Node>,
    ) -> Element {
        Element {
            name,
            attributes,
            handlers,
            children,
        }
    }

    pub fn diff(a: &mut Element, b: &Element, j: &usize) -> Option<Diff> {
        if a.name != b.name {
            return Some(Diff::ReplaceChild(j.clone(), Node::Element(b.clone())));
        }

        let mut result = vec![];

        result.append(&mut diff_attributes(&mut a.attributes, &b.attributes));
        result.append(&mut diff_handlers(&mut a.handlers, &b.handlers));

        let mut i = 0;
        a.children.reverse();
        for new_child in &b.children {
            match a.children.pop() {
                None => {
                    result.push(Diff::AddChild(i, new_child.clone()));
                    i += 1;
                }
                Some(mut old_child) => {
                    if let Some(d) = Node::diff(&mut old_child, new_child, &mut i) {
                        result.push(d);
                    }
                }
            }
            i += 1;
        }

        for _ in &a.children {
            result.push(Diff::RemoveChild(i));
        }

        if result.is_empty() {
            return None;
        }
        Some(Diff::PatchChild(j.clone(), result))
    }
}

#[derive(Debug)]
pub enum Diff {
    SetAttribute(String, AttributeValue),
    RemoveAttribute(String),
    AddChild(usize, Node),
    ReplaceChild(usize, Node),
    RemoveChild(usize),
    PatchChild(usize, Vec<Diff>),
    SetHandler(String, String),
    RemoveHandler(String, String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttributeValue {
    String(String),
    Bool(bool),
}

impl From<String> for AttributeValue {
    fn from(s: String) -> AttributeValue {
        AttributeValue::String(s)
    }
}

impl<'a> From<&'a str> for AttributeValue {
    fn from(s: &'a str) -> AttributeValue {
        AttributeValue::String(s.to_string())
    }
}

impl From<bool> for AttributeValue {
    fn from(b: bool) -> AttributeValue {
        AttributeValue::Bool(b)
    }
}

type HandlerMap<A> = HashMap<String, HandlerFunction<A>>;

pub struct View<A> {
    node: Node,
    handler_map: HandlerMap<A>,
}

impl<A> View<A> {
    pub fn new(
        name: String,
        attributes: Attributes,
        handlers: Vec<(String, (u64, String, HandlerFunction<A>))>,
        children: Vec<View<A>>,
    ) -> View<A> {
        let mut handler_map = HashMap::new();
        let handlers = handlers
            .into_iter()
            .map(|(kind, (hash, id, f))| {
                let handler = (kind, (hash, id.clone()));
                handler_map.insert(id, f);
                handler
            })
            .collect();

        let children = children
            .into_iter()
            .map(|child| {
                handler_map.extend(child.handler_map);
                child.node
            })
            .collect();

        View {
            node: Node::Element(Element::new(name, attributes, handlers, children)),
            handler_map,
        }
    }

    pub fn text(s: String) -> View<A> {
        View {
            node: Node::Text(s),
            handler_map: HashMap::new(),
        }
    }

    pub fn null() -> View<A> {
        View {
            node: Node::Null,
            handler_map: HashMap::new(),
        }
    }
}

impl<A> From<()> for View<A> {
    fn from(_: ()) -> View<A> {
        View::null()
    }
}

impl<A> From<String> for View<A> {
    fn from(s: String) -> View<A> {
        View::text(s)
    }
}

impl<'a, A> From<&'a str> for View<A> {
    fn from(s: &'a str) -> View<A> {
        View::text(s.to_string())
    }
}

impl<A, T> From<Option<T>> for View<A>
where
    T: Into<View<A>>,
{
    fn from(option: Option<T>) -> View<A> {
        option.map_or_else(|| View::null(), |v| v.into())
    }
}

pub trait App: 'static + Clone {
    type State: Clone + Debug + PartialEq + 'static;
    type Action: Clone + Debug + 'static;

    fn reducer(state: Self::State, action: Self::Action) -> Self::State;

    fn view(state: Self::State) -> View<Self::Action>;
}

pub fn handler<A, H, F>(hash: H, f: F) -> (u64, String, HandlerFunction<A>)
where
    H: Hash,
    F: Fn(HandlerArg) -> Option<A> + 'static,
{
    let mut hasher = DefaultHasher::new();
    let mut rng = OsRng::new().unwrap();
    hash.hash(&mut hasher);
    let mut bytes = [0; 16];
    rng.fill_bytes(&mut bytes);

    let id = Uuid::from_random_bytes(bytes).to_string();

    (hasher.finish(), id, Box::new(f))
}

#[derive(Clone)]
pub struct Env<A: App> {
    state: Rc<RefCell<A::State>>,
    node: Rc<RefCell<Node>>,
    handler_map: Rc<RefCell<HandlerMap<A::Action>>>,
    scheduled: Rc<Cell<bool>>,
}

impl<A: App> Env<A> {
    pub fn new(state: A::State) -> Env<A> {
        Env {
            state: Rc::new(RefCell::new(state)),
            node: Rc::new(RefCell::new(Node::Null)),
            handler_map: Rc::new(RefCell::new(HashMap::new())),
            scheduled: Rc::new(Cell::new(false)),
        }
    }

    fn get_state(&self) -> A::State {
        self.state.borrow().clone()
    }

    fn set_state(&self, state: A::State) {
        *self.state.borrow_mut() = state;
    }

    fn get_node(&self) -> Node {
        self.node.borrow().clone()
    }

    fn set_node(&self, node: Node) {
        *self.node.borrow_mut() = node;
    }

    fn pop_handler(&self, id: &str) -> Option<HandlerFunction<A::Action>> {
        self.handler_map.borrow_mut().remove(id)
    }
}

pub trait Runtime<A: App>: Clone + 'static {
    fn get_env<'a>(&'a self) -> &'a Env<A>;

    fn handle_diff(&self, diff: Diff);

    fn schedule_render(&self);

    fn debug<T: Debug + 'static>(v: T);

    fn run(&self) {
        let env = self.get_env();
        env.scheduled.set(false);
        let mut old_node = env.get_node();
        let view = A::view(env.get_state());
        *env.handler_map.borrow_mut() = view.handler_map;
        if let Some(diff) = Node::diff(&mut old_node, &view.node, &mut 0) {
            env.set_node(view.node);
            self.handle_diff(diff);
        }
    }

    fn get_handler(&self, id: &str) -> Option<Box<Fn(HandlerArg)>> {
        let handler = match self.get_env().pop_handler(id) {
            Some(h) => h,
            None => return None,
        };
        let this = self.clone();
        let f = move |arg: HandlerArg| {
            let action = match handler(arg) {
                Some(a) => a,
                None => return,
            };

            let env = this.get_env();

            let old_state = env.get_state();
            let new_state = A::reducer(old_state.clone(), action);
            if old_state == new_state {
                return;
            }
            env.set_state(new_state);
            if env.scheduled.get() {
                return;
            }
            env.scheduled.set(true);
            this.schedule_render();
        };
        Some(Box::new(f))
    }
}
