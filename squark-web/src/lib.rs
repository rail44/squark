use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use futures::Future;
use wasm_bindgen_futures::future_to_promise;
use squark::{
    uuid,
    App, AttributeValue, Diff, Element as SquarkElement, Env, HandlerArg, Node as SquarkNode,
    Runtime,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, Document, Element, EventTarget, HtmlElement, Node};
use serde::Serialize;
use serde_json::json;

trait ToHandlerArg: JsCast {
    fn to_handler_arg(self) -> HandlerArg;
}

impl ToHandlerArg for web_sys::Event {
    fn to_handler_arg(self) -> HandlerArg {
        json!{null}
    }
}

impl ToHandlerArg for web_sys::InputEvent {
    fn to_handler_arg(self) -> HandlerArg {
        let ev: web_sys::Event = self.into();
        let target = ev.target().unwrap();
        let js_val: &JsValue = target.as_ref();
        if js_val.is_null() {
            return json!{""};
        }
        let input_el: &web_sys::HtmlInputElement = target.unchecked_ref();
        json!{input_el.value()}
    }
}

impl ToHandlerArg for web_sys::KeyboardEvent {
    fn to_handler_arg(self) -> HandlerArg {
        json!{self.key()}
    }
}

type AttachedMap = HashMap<String, HashMap<String, Closure<Fn(JsValue)>>>;

fn document() -> Document {
    window().unwrap().document().unwrap()
}

fn get_handler_id(el: &HtmlElement) -> Option<String> {
    let id = js_sys::Reflect::get(el.dataset().as_ref(), &"handlerId".into()).unwrap();

    if id.is_undefined() {
        return None;
    }
    Some(id.as_string().unwrap())
}

fn set_handler_id(el: &HtmlElement, id: &str) {
    js_sys::Reflect::set(el.dataset().as_ref(), &"handlerId".into(), &id.into()).unwrap();
}

#[derive(Clone)]
pub struct WebRuntime<A: App> {
    env: Env<A>,
    root: Rc<Element>,
    attached_map: Rc<RefCell<AttachedMap>>,
}

fn insert_at(parent: &Node, i: usize, node: &Node) {
    let ref_node = parent.child_nodes().item(i as u32);
    if ref_node.is_none() {
        parent.append_child(&node).unwrap();
        return;
    }
    parent.insert_before(&node, ref_node.as_ref()).unwrap();
}

fn set_attribute(el: &Element, name: &str, value: &AttributeValue) {
    match value {
        AttributeValue::Bool(b) => {
            js_sys::Reflect::set(el.as_ref(), &name.into(), &(*b).into()).unwrap();
            el.set_attribute(name, &b.to_string()).unwrap();
        }
        AttributeValue::String(s) => {
            js_sys::Reflect::set(el.as_ref(), &name.into(), &s.into()).unwrap();
            el.set_attribute(name, s).unwrap();
        }
    }
}

impl<A: App> WebRuntime<A> {
    pub fn new(root: Element, state: A::State) -> WebRuntime<A> {
        WebRuntime {
            env: Env::new(state),
            root: Rc::new(root),
            attached_map: Rc::new(RefCell::new(AttachedMap::new())),
        }
    }

    fn handle_diff_inner(&self, el: &Element, diff: Diff) {
        match diff {
            Diff::AddChild(i, node) => self.add_child(el, i, node),
            Diff::PatchChild(i, diffs) => {
                let as_node: &Node = el.as_ref();
                let child = as_node.child_nodes().item(i as u32).unwrap();
                for diff in diffs {
                    self.handle_diff_inner(child.unchecked_ref(), diff);
                }
            }
            Diff::ReplaceChild(i, node) => self.replace_child(el, i, node),
            Diff::SetAttribute(name, value) => set_attribute(el, &name, &value),
            Diff::RemoveAttribute(name) => {
                el.remove_attribute(&name).unwrap();
            }
            Diff::RemoveChild(i) => self.remove_child(el.as_ref(), i),
            Diff::SetHandler(name, id) => self.set_handler(el.unchecked_ref(), &name, &id),
            Diff::RemoveHandler(name, _) => {
                let attached = self
                    .attached_map
                    .borrow_mut()
                    .get_mut(&get_handler_id(el.unchecked_ref()).unwrap())
                    .and_then(|inner| inner.remove(&name))
                    .unwrap();
                let html_el: &EventTarget = el.unchecked_ref();
                html_el
                    .remove_event_listener_with_callback(&name, attached.as_ref().unchecked_ref())
                    .unwrap();
            }
        }
    }

    fn replace_at(&self, parent: &Node, i: usize, node: &Node) {
        let current = parent.child_nodes().item(i as u32).unwrap();
        self.remove_attached(&current);
        parent.replace_child(&node, &current).unwrap();
    }

    fn create_element(&self, el: &SquarkElement) -> Element {
        let web_el: Element = document().create_element(el.name()).unwrap();
        for (ref name, ref value) in el.attributes() {
            set_attribute(&web_el, name, value);
        }

        for (ref name, id) in el.handlers() {
            self.set_handler(web_el.unchecked_ref(), name, &id);
        }

        {
            let node: &Node = web_el.as_ref();
            for child in el.children() {
                match child {
                    SquarkNode::Element(el) => {
                        let child = self.create_element(el);
                        node.append_child(child.as_ref()).unwrap();
                    }
                    SquarkNode::Text(s) => {
                        let child = document().create_text_node(s.as_str());
                        node.append_child(child.as_ref()).unwrap();
                    }
                    _ => (),
                };
            }
        }

        web_el
    }

    fn add_child(&self, parent: &Element, i: usize, node: SquarkNode) {
        match node {
            SquarkNode::Element(el) => {
                let child = self.create_element(&el);
                insert_at(parent.as_ref(), i, child.as_ref());
            }
            SquarkNode::Text(s) => {
                let child = document().create_text_node(s.as_str());
                insert_at(parent.as_ref(), i, child.as_ref());
            }
            _ => (),
        };
    }

    fn replace_child(&self, parent: &Element, i: usize, node: SquarkNode) {
        match node {
            SquarkNode::Element(el) => {
                let child = self.create_element(&el);
                self.replace_at(parent.as_ref(), i, child.as_ref());
            }
            SquarkNode::Text(s) => {
                let child = document().create_text_node(s.as_str());
                self.replace_at(parent.as_ref(), i, child.as_ref());
            }
            _ => (),
        };
    }

    fn remove_child(&self, parent: &Node, i: usize) {
        let current = parent.child_nodes().item(i as u32).unwrap();

        self.remove_attached(current.unchecked_ref());
        parent.remove_child(&current).unwrap();
    }

    fn set_handler(&self, el: &Element, name: &str, id: &str) {
        let closure = match name {
            "keydown" => self._set_handler::<web_sys::KeyboardEvent>(el.as_ref(), "keydown", id),
            "input" => self._set_handler::<web_sys::InputEvent>(el.as_ref(), "input", id),
            name => self._set_handler::<web_sys::Event>(el.as_ref(), name, id),
        };

        let handler_id = get_handler_id(el.unchecked_ref()).unwrap_or_else(|| {
            let uuid = uuid();
            set_handler_id(el.unchecked_ref(), &uuid);
            uuid
        });

        let mut map = self.attached_map.borrow_mut();
        let inner = map.entry(handler_id).or_insert_with(HashMap::new);
        if let Some(attached) = inner.remove(name) {
            let target: &EventTarget = el.as_ref();
            target
                .remove_event_listener_with_callback(&name, attached.as_ref().unchecked_ref())
                .unwrap();
        }
        inner.insert(name.to_owned(), closure);
    }

    fn _set_handler<T: ToHandlerArg>(
        &self,
        el: &EventTarget,
        name: &str,
        id: &str,
    ) -> Closure<Fn(JsValue)> {
        let handler = self.pop_handler(id).unwrap();
        let closure = Closure::new(move |ev: JsValue| {
            let ev: T = ev.unchecked_into();
            handler(ev.to_handler_arg());
        });
        el.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref())
            .unwrap();
        closure
    }

    fn remove_attached(&self, el: &Node) {
        if !el.is_instance_of::<Element>() {
            return;
        }

        let el: &Element = el.unchecked_ref();

        let mut map = self.attached_map.borrow_mut();
        if let Some(id) = get_handler_id(el.unchecked_ref()) {
            map.remove(&id);
        }

        let children = el.query_selector_all("[data-has-handler]").unwrap();
        for i in 0..children.length() {
            let child = children.item(i).unwrap();
            if let Some(id) = get_handler_id(child.unchecked_ref()) {
                map.remove(&id);
            }
        }
    }
}

fn nop<T>(_: T) {}

impl<A: App> Runtime<A> for WebRuntime<A> {
    fn get_env<'a>(&'a self) -> &'a Env<A> {
        &self.env
    }

    fn schedule_render(&self) {
        let this = self.clone();
        let closure = Closure::wrap(Box::new(move |_: JsValue| {
            this.run();
        }) as Box<FnMut(_)>);
        window()
            .unwrap()
            .request_animation_frame(closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    fn handle_diff(&self, diff: Diff) {
        self.handle_diff_inner(&self.root, diff);
    }

    fn handle_future<T: Serialize + 'static, E: Serialize + 'static>(&self, future: Box<Future<Item = T, Error = E>>) {
        let p = future_to_promise(
            future
                .map(|v| JsValue::from_serde(&v).unwrap())
                .map_err(|e| JsValue::from_serde(&e).unwrap())
        );
        let closure = Closure::new(nop);
        p.then(&closure);
        closure.forget();
    }
}
