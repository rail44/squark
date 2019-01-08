#![feature(proc_macro_hygiene)]
extern crate squark;
extern crate squark_macros;
extern crate squark_web;
extern crate wasm_bindgen;
extern crate wasm_bindgen_futures;
extern crate web_sys;
extern crate js_sys;
extern crate futures;

use squark::{App, Runtime, View, Task};
use squark_macros::view;
use squark_web::WebRuntime;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::window;
use js_sys::Promise;
use futures::prelude::*;

#[derive(Clone, Debug, PartialEq)]
struct State {
    count: isize,
}

impl State {
    pub fn new() -> State {
        State { count: 0 }
    }
}

#[derive(Clone, Debug)]
enum Action {
    Increment,
    Decrement,
    Timeout,
}

#[derive(Clone, Debug)]
struct CounterApp;
impl App for CounterApp {
    type State = State;
    type Action = Action;

    fn reducer(&self, mut state: State, action: Action) -> (State, Task<Action>) {
        let mut task = Task::empty();
        match action {
            Action::Increment => {
                state.count += 1;
            }
            Action::Decrement => {
                state.count -= 1;
            }
            Action::Timeout => {
                let p = Promise::new(&mut move |resolve, _| {
                    let closure = Closure::wrap(Box::new(move |_: JsValue| {
                        resolve.call0(&JsValue::null()).unwrap();
                    }) as Box<FnMut(_)>);
                    window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), 1000).unwrap();
                    closure.forget();
                });
                let future = JsFuture::from(p)
                    .map(move |_| {
                        Action::Increment
                    })
                    .map_err(|e| panic!("delay errored; err={:?}", e));
                task.push(Box::new(future));
            }
        };
        (state, task)
    }

    fn view(&self, state: State) -> View<Action> {
        let count = state.count;
        view! {
            <div>
                { count.to_string() }
                <button onclick={ move |_| Some(Action::Increment) }>
                    increment
                </button>
                <button onclick={ move |_| Some(Action::Decrement) }>
                    decrement
                </button>
                <button onclick={ move |_| Some(Action::Timeout) }>
                    timeout
                </button>
            </div>
        }
    }
}

impl Default for CounterApp {
    fn default() -> CounterApp {
        CounterApp
    }
}

#[wasm_bindgen]
pub fn run() {
    WebRuntime::<CounterApp>::new(
        window()
            .unwrap()
            .document()
            .expect("Failed to get document")
            .query_selector("body")
            .unwrap()
            .unwrap(),
        State::new(),
    )
    .run();
}
