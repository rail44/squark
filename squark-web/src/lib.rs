extern crate wasm_bindgen;

#[macro_use]
extern crate serde_json;
extern crate squark;
extern crate js_sys;

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashMap};

use squark::{App, AttributeValue, Diff, Element, Env, Node, Runtime, HandlerArg, uuid};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

type AttachedMap = HashMap<String, HashMap<String, Closure<Fn(JsValue)>>>;

fn get_id(el: &web::HTMLElement) -> String {
    let id = el.id();
    if &id != "" {
        return id;
    }

    let id = uuid();
    el.set_id(&id);
    id
}

pub mod web {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_name = requestAnimationFrame)]
        pub fn request_animation_frame(f: &Closure<Fn(JsValue)>);

        pub type Document;
        pub static document: Document;

        #[wasm_bindgen(method, js_name = createTextNode)]
        pub fn create_text_node(this: &Document, text: &str) -> HTMLElement;
        #[wasm_bindgen(method, js_name = createElement)]
        pub fn create_element(this: &Document, text: &str) -> HTMLElement;
        #[wasm_bindgen(method, js_name = querySelector)]
        pub fn query_selector(this: &Document, selector: &str) -> HTMLElement;

        pub type HTMLElement;

        #[wasm_bindgen(method, js_name = appendChild)]
        pub fn append_child(this: &HTMLElement, child: &HTMLElement);
        #[wasm_bindgen(method, js_name = setAttribute)]
        pub fn set_attribute(this: &HTMLElement, name: &str, value: &str);
        #[wasm_bindgen(method, js_name = removeAttribute)]
        pub fn remove_attribute(this: &HTMLElement, name: &str);
        #[wasm_bindgen(method, getter, js_name = childNodes)]
        pub fn child_nodes(this: &HTMLElement) -> NodeList;
        #[wasm_bindgen(method, js_name = removeChild)]
        pub fn remove_child(this: &HTMLElement, child: &HTMLElement);
        #[wasm_bindgen(method, js_name = replaceChild)]
        pub fn replace_child(this: &HTMLElement, to: &HTMLElement, from: &HTMLElement);
        #[wasm_bindgen(method, js_name = insertBefore)]
        pub fn insert_before(this: &HTMLElement, node: &HTMLElement, ref_node: &HTMLElement);
        #[wasm_bindgen(method, js_name = addEventListener)]
        pub fn add_event_listener(this: &HTMLElement, name: &str, handler: &Closure<Fn(JsValue)>);
        #[wasm_bindgen(method, js_name = removeEventListener)]
        pub fn remove_event_listener(this: &HTMLElement, name: &str, handler: &Closure<Fn(JsValue)>);
        #[wasm_bindgen(method, getter)]
        pub fn dataset(this: &HTMLElement) -> DOMStringMap;
        #[wasm_bindgen(method, js_name = querySelectorAll)]
        pub fn query_selector_all(this: &HTMLElement, selector: &str) -> NodeList;
        #[wasm_bindgen(method, getter)]
        pub fn id(this: &HTMLElement) -> String;
        #[wasm_bindgen(method, setter)]
        pub fn set_id(this: &HTMLElement, id: &str);

        pub type DOMStringMap;

        pub type NodeList;
        #[wasm_bindgen(method)]
        pub fn item(this: &NodeList, index: u16) -> JsValue;
        #[wasm_bindgen(method, js_name = forEach)]
        pub fn for_each(this: &NodeList, f: &mut FnMut(HTMLElement));

        pub type HTMLInputElement;
        pub type CommonEvent;

        pub type InputEvent;
        #[wasm_bindgen(method, getter)]
        pub fn target(this: &InputEvent) -> JsValue;
        #[wasm_bindgen(method, getter)]
        pub fn value(this: &HTMLInputElement) -> String;

        pub type KeyboardEvent;
        #[wasm_bindgen(method, getter)]
        pub fn key(this: &KeyboardEvent) -> String;
    }
}

trait ToHandlerArg {
    fn to_handler_arg(v: JsValue) -> HandlerArg;
}

impl ToHandlerArg for web::CommonEvent{
    fn to_handler_arg(_v: JsValue) -> HandlerArg {
        json!{null}
    }
}

impl ToHandlerArg for web::InputEvent{
    fn to_handler_arg(v: JsValue) -> HandlerArg {
        let target = Self::from(v).target();
        if target.is_null() {
            return json!{""};
        }
        json!{web::HTMLInputElement::from(target).value()}
    }
}

impl ToHandlerArg for web::KeyboardEvent {
    fn to_handler_arg(v: JsValue) -> HandlerArg {
        json!{Self::from(v).key()}
    }
}

#[derive(Clone)]
pub struct WebRuntime<A: App> {
    env: Env<A>,
    root: Rc<web::HTMLElement>,
    attached_map: Rc<RefCell<AttachedMap>>,
}

fn insert_at(parent: &web::HTMLElement, i: usize, node: &web::HTMLElement) {
    let ref_node = parent.child_nodes().item(i as u16);
    if ref_node.is_null() {
        parent.append_child(&node);
        return;
    }
    parent.insert_before(&node, &ref_node.into());
}

fn set_attribute(el: &web::HTMLElement, name: &str, value: &AttributeValue) {
    match value {
        AttributeValue::Bool(b) => {
            js_sys::Reflect::set(&el.into(), &name.into(), &(*b).into());
            el.set_attribute(name, &b.to_string());
        }
        AttributeValue::String(s) => {
            js_sys::Reflect::set(&el.into(), &name.into(), &s.into());
            el.set_attribute(name, s);
        }
    }
}

impl<A: App> WebRuntime<A> {
    pub fn new(root: web::HTMLElement, state: A::State) -> WebRuntime<A> {
        WebRuntime {
            env: Env::new(state),
            root: Rc::new(root),
            attached_map: Rc::new(RefCell::new(AttachedMap::new())),
        }
    }

    fn handle_diff_inner(&self, el: &web::HTMLElement, diff: Diff) {
        match diff {
            Diff::AddChild(i, node) => self.add_child(el, i, node),
            Diff::PatchChild(i, diffs) => {
                let child = el.child_nodes().item(i as u16).into();
                for diff in diffs {
                    self.handle_diff_inner(&child, diff);
                }
            }
            Diff::ReplaceChild(i, node) => self.replace_child(el, i, node),
            Diff::SetAttribute(name, value) => set_attribute(el, &name, &value),
            Diff::RemoveAttribute(name) => {
                el.remove_attribute(&name);
            }
            Diff::RemoveChild(i) => self.remove_child(el, i),
            Diff::SetHandler(name, id) => self.set_handler(el, &name, &id),
            Diff::RemoveHandler(name, _) => {
                let attached = self
                    .attached_map
                    .borrow_mut()
                    .get_mut(&get_id(el))
                    .and_then(|inner| inner.remove(&name))
                    .expect("Could not find attached listener");
                el.remove_event_listener(&name, &attached);
            }
        }
    }

    fn replace_at(&self, parent: &web::HTMLElement, i: usize, node: &web::HTMLElement) {
        let current = parent.child_nodes().item(i as u16).into();
        self.remove_attached(&current);
        parent.replace_child(&node, &current);
    }

    fn create_element(&self, el: Element) -> web::HTMLElement {
        let web_el = web::document.create_element(el.name.as_str());
        for (ref name, ref value) in &el.attributes {
            set_attribute(&web_el, name, value);
        }

        for (ref name, id) in &el.handlers {
            self.set_handler(&web_el, name, &id);
        }

        for child in el.children {
            match child {
                Node::Element(el) => {
                    let child = self.create_element(el);
                    web_el.append_child(&child);
                }
                Node::Text(s) => {
                    let child = web::document.create_text_node(s.as_str());
                    web_el.append_child(&child);
                }
                _ => (),
            };
        }

        web_el
    }

    fn add_child(&self, parent: &web::HTMLElement, i: usize, node: Node) {
        match node {
            Node::Element(el) => {
                let child = self.create_element(el);
                insert_at(parent, i, &child);
            }
            Node::Text(s) => {
                let child = web::document.create_text_node(s.as_str());
                insert_at(parent, i, &child);
            }
            _ => (),
        };
    }

    fn replace_child(&self, parent: &web::HTMLElement, i: usize, node: Node) {
        match node {
            Node::Element(el) => {
                let child = self.create_element(el);
                self.replace_at(parent, i, &child);
            }
            Node::Text(s) => {
                let child = web::document.create_text_node(s.as_str());
                self.replace_at(parent, i, &child);
            }
            _ => (),
        };
    }

    fn remove_child(&self, parent: &web::HTMLElement, i: usize) {
        let current = parent.child_nodes().item(i as u16).into();
        self.remove_attached(&current);
        parent.remove_child(&current);
    }

    fn set_handler(&self, el: &web::HTMLElement, name: &str, id: &str) {
        js_sys::Reflect::set(&el.dataset().into(), &"hasHandler".into(), &"".into());
        let closure = match name {
            "keydown" => self._set_handler::<web::KeyboardEvent>(&el, "keydown", id),
            "input" => self._set_handler::<web::InputEvent>(&el, "input", id),
            name => self._set_handler::<web::CommonEvent>(&el, name, id),
        };

        let id = get_id(el);
        let mut map = self.attached_map.borrow_mut();
        let inner = map.entry(id).or_insert_with(HashMap::new);
        if let Some(attached) = inner.remove(name) {
            el.remove_event_listener(&name, &attached);
        }
        inner.insert(name.to_string(), closure);
    }

    fn _set_handler<E: ToHandlerArg>(
        &self,
        el: &web::HTMLElement,
        name: &str,
        id: &str,
    ) -> Closure<Fn(JsValue)> {
        let handler = self.pop_handler(id).expect("Could not find handler by given id");
        let closure = Closure::new(move |e: JsValue| {
            handler(E::to_handler_arg(e));
        });
        el.add_event_listener(name, &closure);
        closure
    }

    fn remove_attached(&self, el: &web::HTMLElement) {
        if !el.is_instance_of::<web::HTMLElement>() {
            return;
        }
        let mut map = self.attached_map.borrow_mut();
        map.remove(&get_id(el));

        if !el.is_instance_of::<web::HTMLElement>() {
            return;
        }
        el.query_selector_all("[data-has-handler]")
            .for_each(&mut |child| {
                map.remove(&get_id(&child));
            });
    }
}

impl<A: App> Runtime<A> for WebRuntime<A> {
    fn get_env<'a>(&'a self) -> &'a Env<A> {
        &self.env
    }

    fn schedule_render(&self) {
        let this = self.clone();
        let closure = Closure::new(move |_| {
            this.run();
        });
        web::request_animation_frame(&closure);
        closure.forget();
    }

    fn handle_diff(&self, diff: Diff) {
        self.handle_diff_inner(&self.root, diff);
    }
}

