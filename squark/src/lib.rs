extern crate rand;
extern crate serde_json;
extern crate uuid;

use std::fmt::Debug;
use std::collections::{HashMap, HashSet};
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
            (&mut Node::Element(ref mut a), &Node::Element(ref b)) => {
                match Element::diff(a, b, i) {
                    Some(diff) => Some(diff),
                    None => None,
                }
            }
            (&mut Node::Text(ref mut text_a), &Node::Text(ref text_b)) => {
                if text_a == text_b {
                    return None;
                }
                Some(Diff::ReplaceChild(i.clone(), b.clone()))
            }
            (&mut Node::Null, &Node::Null) => None,
            (&mut Node::Null, _) => Some(Diff::AddChild(i.clone(), b.clone())),
            _ => Some(Diff::ReplaceChild(i.clone(), b.clone())),
        }
    }

    fn get_key(&self) -> Option<String> {
        match self {
            &Node::Element(ref el) => el.get_key(),
            _ => None,
        }
    }
}

fn get_nodelist_key_set(nodelist: &Vec<Node>) -> HashSet<String> {
    HashSet::from_iter(nodelist.iter().filter_map(|c| c.get_key()))
}

fn diff_children(a: &mut Vec<Node>, b: &Vec<Node>, i: &mut usize) -> Vec<Diff> {
    let mut result = vec![];
    let b_key_set = get_nodelist_key_set(b);
    let survived = a.drain(..)
        .filter(|c| match c.get_key() {
            Some(k) => {
                let is_survived = b_key_set.contains(&k);
                if !is_survived {
                    result.push(Diff::RemoveChild(i.clone()));
                    return false;
                }
                *i += 1;
                true
            }
            None => {
                *i += 1;
                true
            }
        })
        .collect();
    *a = survived;

    let mut i = 0;
    a.reverse();
    for new_child in b.iter() {
        match a.pop() {
            None => {
                result.push(Diff::AddChild(i, new_child.clone()));
                i += 1;
            }
            Some(mut old_child) => {
                if let Some(diff) = Node::diff(&mut old_child, new_child, &mut i) {
                    result.push(diff);
                }
            }
        }
        i += 1;
    }

    for _ in a.iter() {
        result.push(Diff::RemoveChild(i));
    }

    result
}

#[derive(Clone, Debug)]
pub struct Element {
    pub name: String,
    pub attributes: Attributes,
    pub handlers: Handlers,
    pub children: Vec<Node>,
}

impl Element {
    fn new(
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

    fn diff(a: &mut Element, b: &Element, i: &usize) -> Option<Diff> {
        if let (Some(a_key), Some(b_key)) = (a.get_key(), b.get_key()) {
            if a_key != b_key {
                return Some(Diff::ReplaceChild(i.clone(), Node::Element(b.clone())));
            }
        }

        if a.name != b.name {
            return Some(Diff::ReplaceChild(i.clone(), Node::Element(b.clone())));
        }

        let mut result = vec![];

        result.append(&mut diff_attributes(&mut a.attributes, &b.attributes));
        result.append(&mut diff_handlers(&mut a.handlers, &b.handlers));
        result.append(&mut diff_children(&mut a.children, &b.children, &mut 0));

        if result.is_empty() {
            return None;
        }
        Some(Diff::PatchChild(i.clone(), result))
    }

    fn get_key(&self) -> Option<String> {
        self.attributes
            .iter()
            .find(|&&(ref k, _)| k == "key")
            .and_then(|&(_, ref v)| match v {
                &AttributeValue::String(ref s) => Some(s.clone()),
                &AttributeValue::Bool(_) => None,
            })
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

pub enum Child<A> {
    View(View<A>),
    ViewList(Vec<View<A>>),
}

impl<A, T> From<T> for Child<A>
where
    T: Into<View<A>> + Sized,
{
    fn from(v: T) -> Child<A> {
        Child::View(v.into())
    }
}

impl<A> FromIterator<View<A>> for Child<A> {
    fn from_iter<I>(iter: I) -> Child<A>
    where
        I: IntoIterator<Item = View<A>>,
    {
        Child::ViewList(iter.into_iter().collect())
    }
}

impl<A> View<A> {
    pub fn new(
        name: String,
        attributes: Attributes,
        handlers: Vec<(String, (u64, String, HandlerFunction<A>))>,
        children: Vec<Child<A>>,
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

        let mut children_vec = vec![];
        for child in children.into_iter() {
            match child {
                Child::View(v) => {
                    handler_map.extend(v.handler_map);
                    children_vec.push(v.node);
                }
                Child::ViewList(child_vec) => for v in child_vec {
                    handler_map.extend(v.handler_map);
                    children_vec.push(v.node);
                },
            }
        }

        View {
            node: Node::Element(Element::new(name, attributes, handlers, children_vec)),
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

    fn pop_handler(&self, id: &str) -> Option<Box<Fn(HandlerArg)>> {
        let handler = self.get_env().pop_handler(id)?;

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
