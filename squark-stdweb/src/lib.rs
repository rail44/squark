#[macro_use]
extern crate serde_json;
extern crate squark;
#[macro_use]
extern crate stdweb;

use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use std::collections::Bound::{Excluded, Included};

use squark::{App, Diff, Element, HandlerArg, HandlerMap, Node, AttributeValue};
use stdweb::traits::*;
use stdweb::unstable::TryFrom;
use stdweb::web;
use stdweb::web::{document, window, EventListenerHandle};
use stdweb::web::html_element::InputElement;
use stdweb::web::event::{BlurEvent, ChangeEvent, ClickEvent, ConcreteEvent, DoubleClickEvent,
                         InputEvent, KeyDownEvent};

type Position = Vec<usize>;
type AttachedMadp = BTreeMap<Position, HashMap<String, EventListenerHandle>>;

trait ToHandlerArg {
    fn to_handler_arg(&self) -> HandlerArg;
}

impl ToHandlerArg for ClickEvent {
    fn to_handler_arg(&self) -> HandlerArg {
        json!{null}
    }
}

impl ToHandlerArg for BlurEvent {
    fn to_handler_arg(&self) -> HandlerArg {
        json!{null}
    }
}

impl ToHandlerArg for DoubleClickEvent {
    fn to_handler_arg(&self) -> HandlerArg {
        json!{null}
    }
}

impl ToHandlerArg for InputEvent {
    fn to_handler_arg(&self) -> HandlerArg {
        json!{self.target().map(|t| InputElement::try_from(t).unwrap().raw_value())}
    }
}

impl ToHandlerArg for KeyDownEvent {
    fn to_handler_arg(&self) -> HandlerArg {
        json!{self.key()}
    }
}

impl ToHandlerArg for ChangeEvent {
    fn to_handler_arg(&self) -> HandlerArg {
        json!{null}
    }
}

#[derive(Clone)]
pub struct Runtime<A: App> {
    state: Rc<RefCell<A::State>>,
    node: Rc<RefCell<Node>>,
    handler_map: Rc<RefCell<HandlerMap<A::Action>>>,
    attached_map: Rc<RefCell<AttachedMadp>>,
    scheduled: Rc<Cell<bool>>,
    root: web::Element,
}

fn insert_at<N: INode>(parent: &web::Element, i: usize, node: N) {
    match parent.child_nodes().into_iter().nth(i) {
        Some(ref_node) => {
            parent.insert_before(&node, &ref_node).unwrap();
        }
        None => {
            parent.append_child(&node);
        }
    }
}

fn replace_at<N: INode>(parent: &web::Element, i: usize, node: N) {
    let current = parent.child_nodes().into_iter().nth(i).unwrap();
    parent.replace_child(&node, &current).unwrap();
}

fn set_attribute(el: &web::Element, name: &str, value: &AttributeValue) {
    match value {
        &AttributeValue::Bool(ref b) => {
            js! { @{el.clone()}[@{name}] = @{b} };
            el.set_attribute(name, b.to_string().as_str()).unwrap();
        },
        &AttributeValue::String(ref s) => {
            js! { @{el.clone()}[@{name}] = @{s} };
            el.set_attribute(name, s).unwrap();
        },
    }
}

impl<A: App> Runtime<A> {
    pub fn new(root: web::Element, state: A::State) -> Runtime<A> {
        Runtime {
            state: Rc::new(RefCell::new(state)),
            node: Rc::new(RefCell::new(Node::Null)),
            handler_map: Rc::new(RefCell::new(HashMap::new())),
            attached_map: Rc::new(RefCell::new(BTreeMap::new())),
            scheduled: Rc::new(Cell::new(false)),
            root,
        }
    }

    pub fn start(&self) {
        let (mut old, new) = {
            let mut node = self.node.borrow_mut();
            let old = node.clone();
            let (new, handler_map) = A::view(self.state.borrow().clone());
            *node = new.clone();
            *self.handler_map.borrow_mut() = handler_map;
            (old, new)
        };
        if let Some(diff) = Node::diff(&mut old, &new, &mut 0) {
            let root = self.root.clone();
            self.handle_diff(&root, diff, &mut vec![]);
        }
    }

    fn schedule_render(&self) {
        if self.scheduled.get() {
            return;
        }
        let this = self.clone();
        window().request_animation_frame(move |_| {
            this.scheduled.set(false);
            this.start();
        });
        self.scheduled.set(false);
    }

    fn handle_diff(&self, el: &web::Element, diff: Diff, pos: &mut Position) {
        match diff {
            Diff::AddChild(i, node) => self.add_child(el, i, node, pos),
            Diff::PatchChild(i, diffs) => {
                let child =
                    web::Element::try_from(el.child_nodes().iter().nth(i).unwrap()).unwrap();
                pos.push(i);
                for diff in diffs {
                    self.handle_diff(&child, diff, pos);
                }
                pos.pop();
            }
            Diff::ReplaceChild(i, node) => self.replace_child(el, i, node, pos),
            Diff::SetAttribute(name, value) => set_attribute(el, &name, &value),
            Diff::RemoveAttribute(name) => {
                js! { @{el}[@{name.clone()}] = undefined };
                el.remove_attribute(&name);
            }
            Diff::RemoveChild(i) => self.remove_child(el, i, pos),
            Diff::SetHandler(name, id) => self.set_handler(el, &name, &id, pos),
            Diff::RemoveHandler(name, _) => {
                let attached = self.attached_map
                    .borrow_mut()
                    .get_mut(pos)
                    .and_then(|m| m.remove(&name));
                if let Some(attached) = attached {
                    attached.remove();
                }
            }
        }
    }

    fn create_element(&self, el: Element, pos: &mut Position) -> web::Element {
        let web_el = document().create_element(el.name.as_str()).unwrap();

        for &(ref name, ref value) in el.attributes.iter() {
            set_attribute(&web_el, name, value);
        }

        for &(ref name, (_, ref id)) in el.handlers.iter() {
            self.set_handler(&web_el, name, &id, pos);
        }

        let mut i = 0;
        for child in el.children {
            pos.push(i.clone());
            match child {
                Node::Element(el) => {
                    let child = self.create_element(el, pos);
                    web_el.append_child(&child);
                }
                Node::Text(s) => {
                    let child = document().create_text_node(s.as_str());
                    web_el.append_child(&child);
                }
                _ => (),
            };
            pos.pop();
            i += 1;
        }

        web_el
    }

    fn add_child(&self, parent: &web::Element, i: usize, node: Node, pos: &mut Position) {
        pos.push(i);
        match node {
            Node::Element(el) => {
                let child = self.create_element(el, pos);
                insert_at(parent, i, child);
            }
            Node::Text(s) => {
                let child = document().create_text_node(s.as_str());
                insert_at(parent, i, child);
            }
            _ => (),
        };
        pos.pop();
    }

    fn replace_child(&self, parent: &web::Element, i: usize, node: Node, pos: &mut Position) {
        pos.push(i);
        self.remove_attached(pos);
        match node {
            Node::Element(el) => {
                let child = self.create_element(el, pos);
                replace_at(parent, i, child);
            }
            Node::Text(s) => {
                let child = document().create_text_node(s.as_str());
                replace_at(parent, i, child);
            }
            _ => (),
        };
        pos.pop();
    }

    fn remove_attached(&self, pos: &Position) {
        let mut max = pos.clone();
        let i = max.pop().unwrap() + 1;
        max.push(i);
        let range = (Included(pos.clone()), Excluded(max));
        let mut map = self.attached_map.borrow_mut();
        let vec: Vec<Position> = map.range(range).map(|(k, _)| k.clone()).collect();
        for k in vec {
            map.remove(&k);
        }
    }

    fn remove_child(&self, parent: &web::Element, i: usize, pos: &mut Position) {
        pos.push(i);
        self.remove_attached(pos);
        let current = parent.child_nodes().into_iter().nth(i).unwrap();
        parent.remove_child(&current).unwrap();
        pos.pop();
    }

    fn set_handler(&self, el: &web::Element, name: &str, id: &str, pos: &Position) {
        let handle = match name {
            "click" => self._set_handler::<ClickEvent>(&el, id),
            "dblclick" => self._set_handler::<DoubleClickEvent>(&el, id),
            "blur" => self._set_handler::<BlurEvent>(&el, id),
            "change" => self._set_handler::<ChangeEvent>(&el, id),
            "input" => self._set_handler::<InputEvent>(&el, id),
            "keydown" => self._set_handler::<KeyDownEvent>(&el, id),
            "render" => {
                let handler = self.handler_map.borrow_mut().remove(id).unwrap();
                let this = self.clone();
                window().request_animation_frame(move |_| {
                    let is_updated = {
                        let mut state = this.state.borrow_mut();
                        match handler(json!{null}) {
                            Some(action) => {
                                *state = A::reducer(state.clone(), action);
                                true
                            }
                            None => false,
                        }
                    };
                    if is_updated {
                        this.schedule_render();
                    }
                    this.scheduled.set(false);
                    this.start();
                });
                return;
            }

            _ => return,
        };

        let mut map = self.attached_map.borrow_mut();
        let attached = map.get_mut(pos).and_then(|m| m.remove(name));
        if let Some(attached) = attached {
            attached.remove();
        }
        map.entry(pos.clone())
            .or_insert(HashMap::new())
            .insert(name.to_string(), handle);
    }

    fn _set_handler<E: ConcreteEvent + ToHandlerArg>(
        &self,
        el: &web::Element,
        id: &str,
    ) -> EventListenerHandle {
        let handler = self.handler_map.borrow_mut().remove(id).unwrap();
        let el = el.clone();
        let this = self.clone();
        el.clone().add_event_listener(move |e: E| {
            e.stop_propagation();
            let is_updated = {
                let mut state = this.state.borrow_mut();
                match handler(e.to_handler_arg()) {
                    Some(action) => {
                        *state = A::reducer(state.clone(), action);
                        true
                    }
                    None => false,
                }
            };
            if is_updated {
                this.schedule_render();
                e.prevent_default();
            }
        })
    }
}
