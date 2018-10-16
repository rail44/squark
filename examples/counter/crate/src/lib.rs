#![feature(proc_macro_hygiene)]

extern crate squark;
extern crate squark_macros;
extern crate squark_web;
extern crate wasm_bindgen;
extern crate web_sys;
extern crate console_error_panic_hook;

use std::panic;
use squark::{App, Runtime, View};
use squark_macros::view;
use squark_web::WebRuntime;
use wasm_bindgen::prelude::*;
use web_sys::window;

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
    ChangeCount(isize),
}

#[derive(Clone, Debug)]
struct CounterApp;
impl App for CounterApp {
    type State = State;
    type Action = Action;

    fn reducer(&self, mut state: State, action: Action) -> State {
        match action {
            Action::ChangeCount(c) => {
                state.count = c;
            }
        };
        state
    }

    fn view(&self, state: State) -> View<Action> {
        let count = state.count;
        view! {
            <div>
                { count.to_string() }
                <button onclick={ move |_| Some(Action::ChangeCount(count.clone() + 1)) }>
                    increment
                </button>
                <button onclick={ move |_| Some(Action::ChangeCount(count - 1)) }>
                    decrement
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
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    WebRuntime::<CounterApp>::new(
        window()
            .expect("Failed to get window")
            .document()
            .expect("Failed to get document")
            .query_selector("body")
            .expect("Failed to query")
            .expect("Unable to find body"),
        State::new(),
    )
    .run();
}
