extern crate rand;
extern crate serde_json;
extern crate uuid;

use std::fmt::Debug;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use rand::{OsRng, RngCore};
use uuid::Uuid;
pub use serde_json::Value as HandlerArg;

pub type Attributes = Vec<(String, String)>;

fn diff_attributes(a: &mut Attributes, b: &Attributes) -> Vec<Diff> {
    let mut result = vec![];

    let mut old_map = HashMap::<String, String>::from_iter(a.drain(..));
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

pub type HandlerFunction<A> = Box<Fn(HandlerArg) -> Option<A>>;
pub type Handler = (String, (u64, String));
pub type Handlers = Vec<Handler>;

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
    pub fn diff(a: &mut Node, b: &Node, i: &mut usize) -> Option<Diff> {
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
    SetAttribute(String, String),
    RemoveAttribute(String),
    AddChild(usize, Node),
    ReplaceChild(usize, Node),
    RemoveChild(usize),
    PatchChild(usize, Vec<Diff>),
    SetHandler(String, String),
    RemoveHandler(String, String),
}

pub type HandlerMap<A> = HashMap<String, HandlerFunction<A>>;

pub type View<A> = (Node, HandlerMap<A>);

pub trait App: 'static + Clone {
    type State: Clone + Debug + 'static;
    type Action: Clone + Debug + 'static;

    fn reducer(state: Self::State, action: Self::Action) -> Self::State;

    fn view(state: Self::State) -> View<Self::Action>;
}

pub fn h<A>(
    name: String,
    attributes: Attributes,
    handlers: Vec<(Handler, HandlerFunction<A>)>,
    children: Vec<View<A>>,
) -> View<A> {
    let mut map = HashMap::new();
    let handlers = handlers
        .into_iter()
        .map(|(handler, f)| {
            map.insert((handler.1).1.clone(), f);
            handler
        })
        .collect();

    let children = children
        .into_iter()
        .map(|(node, child_map)| {
            map.extend(child_map);
            node
        })
        .collect();
    (
        Node::Element(Element::new(name, attributes, handlers, children)),
        map,
    )
}

pub fn text<A>(s: String) -> View<A> {
    (Node::Text(s), HashMap::new())
}

pub fn null<A>() -> View<A> {
    (Node::Null, HashMap::new())
}

pub fn handler<A, H, F>(kind: String, hash: H, f: F) -> (Handler, HandlerFunction<A>)
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

    ((kind, (hasher.finish(), id)), Box::new(f))
}
