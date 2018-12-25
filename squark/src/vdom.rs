use rustc_hash::{FxHashMap, FxHashSet};
use std::iter::FromIterator;

pub use serde_json::Value as HandlerArg;

type Attribute = (String, AttributeValue);

fn diff_attributes(a: &mut Vec<Attribute>, b: &[Attribute]) -> Vec<Diff> {
    let mut result = vec![];

    let mut old_map = FxHashMap::<String, AttributeValue>::from_iter(a.drain(..));
    for &(ref new_key, ref new_val) in b {
        match old_map.remove(new_key) {
            Some(old_val) => {
                if &old_val != new_val {
                    result.push(Diff::SetAttribute(new_key.to_owned(), new_val.to_owned()))
                }
            }
            None => result.push(Diff::SetAttribute(new_key.to_owned(), new_val.to_owned())),
        }
    }

    for (old_key, _) in old_map.drain() {
        result.push(Diff::RemoveAttribute(old_key));
    }

    result
}

pub(crate) type HandlerFunction<A> = Box<Fn(HandlerArg) -> Option<A>>;
type Handler = (String, String);

fn diff_handlers(a: &mut Vec<Handler>, b: &[Handler]) -> Vec<Diff> {
    let mut result = vec![];

    let mut old_map = FxHashMap::<String, String>::from_iter(a.drain(..));
    for &(ref new_key, ref new_id) in b {
        old_map.remove(new_key);
        result.push(Diff::SetHandler(new_key.to_owned(), new_id.to_owned()));
    }

    for (old_key, old_id) in old_map.drain() {
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
    pub(crate) fn diff(a: &mut Node, b: &Node, i: &mut usize) -> Option<Diff> {
        match (a, b) {
            (&mut Node::Element(ref mut a), &Node::Element(ref b)) => {
                match Element::diff(a, b, *i) {
                    Some(diff) => Some(diff),
                    None => None,
                }
            }
            (&mut Node::Text(ref mut text_a), &Node::Text(ref text_b)) => {
                if text_a == text_b {
                    return None;
                }
                Some(Diff::ReplaceChild(*i, b.to_owned()))
            }
            (&mut Node::Null, &Node::Null) => None,
            (&mut Node::Null, _) => Some(Diff::AddChild(*i, b.to_owned())),
            (_, &Node::Null) => Some(Diff::RemoveChild(*i)),
            _ => Some(Diff::ReplaceChild(*i, b.to_owned())),
        }
    }

    fn get_key(&self) -> Option<String> {
        match self {
            Node::Element(ref el) => el.get_key(),
            _ => None,
        }
    }
}

fn get_nodelist_key_set(nodelist: &[Node]) -> FxHashSet<String> {
    FxHashSet::from_iter(nodelist.iter().filter_map(|c| c.get_key()))
}

fn diff_children(a: &mut Vec<Node>, b: &[Node], i: &mut usize) -> Vec<Diff> {
    let mut result = vec![];
    let b_key_set = get_nodelist_key_set(b);
    let survived = a
        .drain(..)
        .filter(|c| match c.get_key() {
            Some(k) => {
                let is_survived = b_key_set.contains(&k);
                if !is_survived {
                    result.push(Diff::RemoveChild(*i));
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
                result.push(Diff::AddChild(i, new_child.to_owned()));
                i += 1;
            }
            Some(mut old_child) => {
                if let Some(diff) = Node::diff(&mut old_child, new_child, &mut i) {
                    result.push(diff.to_owned());
                    if let Diff::RemoveChild(_) = diff {
                        continue;
                    }
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
    name: String,
    attributes: Vec<Attribute>,
    handlers: Vec<Handler>,
    children: Vec<Node>,
}

impl Element {
    fn new(
        name: String,
        attributes: Vec<Attribute>,
        handlers: Vec<Handler>,
        children: Vec<Node>,
    ) -> Element {
        Element {
            name,
            attributes,
            handlers,
            children,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn attributes(&self) -> &[Attribute] {
        &self.attributes
    }

    pub fn handlers(&self) -> &[Handler] {
        &self.handlers
    }

    pub fn children(&self) -> &[Node] {
        &self.children
    }

    fn diff(a: &mut Element, b: &Element, i: usize) -> Option<Diff> {
        if let (Some(a_key), Some(b_key)) = (a.get_key(), b.get_key()) {
            if a_key != b_key {
                return Some(Diff::ReplaceChild(i, Node::Element(b.to_owned())));
            }
        }

        if a.name != b.name {
            return Some(Diff::ReplaceChild(i, Node::Element(b.to_owned())));
        }

        let mut result = vec![];

        result.append(&mut diff_attributes(&mut a.attributes, &b.attributes));
        result.append(&mut diff_handlers(&mut a.handlers, &b.handlers));
        result.append(&mut diff_children(&mut a.children, &b.children, &mut 0));

        if result.is_empty() {
            return None;
        }
        Some(Diff::PatchChild(i, result))
    }

    fn get_key(&self) -> Option<String> {
        self.attributes
            .iter()
            .find(|&&(ref k, _)| k == "key")
            .and_then(|&(_, ref v)| match v {
                AttributeValue::String(ref s) => Some(s.to_owned()),
                AttributeValue::Bool(_) => None,
            })
    }
}

#[derive(Debug, Clone)]
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
        AttributeValue::String(s.to_owned())
    }
}

impl From<bool> for AttributeValue {
    fn from(b: bool) -> AttributeValue {
        AttributeValue::Bool(b)
    }
}

pub(crate) type HandlerMap<A> = FxHashMap<String, HandlerFunction<A>>;

pub struct View<A> {
    pub(crate) node: Node,
    pub(crate) handler_map: HandlerMap<A>,
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
        attributes: Vec<Attribute>,
        handlers: Vec<(String, (String, HandlerFunction<A>))>,
        children: Vec<Child<A>>,
    ) -> View<A> {
        let mut handler_map = FxHashMap::default();
        let handlers = handlers
            .into_iter()
            .map(|(kind, (id, f))| {
                let handler = (kind, id.to_owned());
                handler_map.insert(id, f);
                handler
            })
            .collect();

        let mut children_vec = vec![];
        for child in children {
            match child {
                Child::View(v) => {
                    handler_map.extend(v.handler_map);
                    children_vec.push(v.node);
                }
                Child::ViewList(child_vec) => {
                    for v in child_vec {
                        handler_map.extend(v.handler_map);
                        children_vec.push(v.node);
                    }
                }
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
            handler_map: FxHashMap::default(),
        }
    }

    pub fn null() -> View<A> {
        View {
            node: Node::Null,
            handler_map: FxHashMap::default(),
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
        View::text(s.to_owned())
    }
}

impl<A, T> From<Option<T>> for View<A>
where
    T: Into<View<A>>,
{
    fn from(option: Option<T>) -> View<A> {
        option.map_or_else(View::null, |v| v.into())
    }
}
