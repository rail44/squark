#[macro_use]
extern crate serde_json;
extern crate squark;
#[macro_use]
extern crate stdweb;

use std::cell::RefCell;
use std::collections::Bound::{Excluded, Included};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use squark::{App, AttributeValue, Diff, Element, Env, HandlerArg, Node, Runtime};
use stdweb::traits::*;
use stdweb::unstable::TryFrom;
use stdweb::web;
use stdweb::web::event::{
    BlurEvent, ChangeEvent, ClickEvent, ConcreteEvent, DoubleClickEvent, InputEvent, KeyDownEvent,
};
use stdweb::web::html_element::InputElement;
use stdweb::web::{document, window, EventListenerHandle};

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
        json!{self.target().map(|t| InputElement::try_from(t).expect("Failed to convert InputEvent to HandlerArg").raw_value())}
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
pub struct StdwebRuntime<A: App> {
    env: Env<A>,
    attached_map: Rc<RefCell<AttachedMadp>>,
    root: web::Element,
}

fn insert_at<N: INode>(parent: &web::Element, i: usize, node: N) {
    match parent.child_nodes().into_iter().nth(i) {
        Some(ref_node) => {
            parent
                .insert_before(&node, &ref_node)
                .expect("Failed to insert at given position");
        }
        None => {
            parent.append_child(&node);
        }
    }
}

fn replace_at<N: INode>(parent: &web::Element, i: usize, node: N) {
    let current = parent
        .child_nodes()
        .into_iter()
        .nth(i)
        .expect("Could not find node by given index");
    parent
        .replace_child(&node, &current)
        .expect("Failed to replace child");
}

fn set_attribute(el: &web::Element, name: &str, value: &AttributeValue) {
    match value {
        &AttributeValue::Bool(ref b) => {
            js! { @{el.clone()}[@{name}] = @{b} };
            el.set_attribute(name, &b.to_string())
                .expect("Failet to set bool attirbute");
        }
        &AttributeValue::String(ref s) => {
            js! { @{el.clone()}[@{name}] = @{s} };
            el.set_attribute(name, s)
                .expect("Failed to set string attribute");
        }
    }
}

impl<A: App> StdwebRuntime<A> {
    pub fn new(root: web::Element, state: A::State) -> StdwebRuntime<A> {
        StdwebRuntime {
            env: Env::new(state),
            attached_map: Rc::new(RefCell::new(BTreeMap::new())),
            root,
        }
    }

    fn handle_diff_inner(&self, el: &web::Element, diff: Diff, pos: &mut Position) {
        match diff {
            Diff::AddChild(i, node) => self.add_child(el, i, node, pos),
            Diff::PatchChild(i, diffs) => {
                let child = web::Element::try_from(
                    el.child_nodes()
                        .iter()
                        .nth(i)
                        .expect("Failed to find child for patching"),
                ).expect("Failed to convert Node to Element");
                pos.push(i);
                for diff in diffs {
                    self.handle_diff_inner(&child, diff, pos);
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
                let attached = self
                    .attached_map
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
        let web_el = document()
            .create_element(el.name.as_str())
            .expect("Failed to create element");

        for &(ref name, ref value) in el.attributes.iter() {
            set_attribute(&web_el, name, value);
        }

        for &(ref name, ref id) in el.handlers.iter() {
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
        let i = max
            .pop()
            .expect("Failed to pop index from posion to use max range") + 1;
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
        let current = parent
            .child_nodes()
            .into_iter()
            .nth(i)
            .expect("Could not find node for removing");
        parent
            .remove_child(&current)
            .expect("Failed to remove child");
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
                let handler = self
                    .pop_handler(&id)
                    .expect("Could not find handler by given id");
                window().request_animation_frame(move |_| {
                    handler(json!{null});
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
            .insert(name.to_owned(), handle);
    }

    fn _set_handler<E: ConcreteEvent + ToHandlerArg>(
        &self,
        el: &web::Element,
        id: &str,
    ) -> EventListenerHandle {
        let handler = self
            .pop_handler(id)
            .expect("Could not find handler by given id");
        el.clone().add_event_listener(move |e: E| {
            e.stop_propagation();
            let arg = e.to_handler_arg();
            handler(arg);
        })
    }
}

impl<A: App> Runtime<A> for StdwebRuntime<A> {
    fn get_env<'a>(&'a self) -> &'a Env<A> {
        &self.env
    }

    fn schedule_render(&self) {
        let this = self.clone();
        window().request_animation_frame(move |_| {
            this.run();
        });
    }

    fn handle_diff(&self, diff: Diff) {
        self.handle_diff_inner(&self.root, diff, &mut vec![]);
    }
}
