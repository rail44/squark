#![feature(use_extern_macros, proc_macro_non_items)]

extern crate squark;
extern crate squark_macros;
extern crate squark_stdweb;
extern crate stdweb;

use squark::{App, Runtime, View};
use squark_macros::view;
use squark_stdweb::StdwebRuntime;
use stdweb::traits::*;
use stdweb::web::document;

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

    fn reducer(mut state: State, action: Action) -> State {
        match action {
            Action::ChangeCount(c) => {
                state.count = c;
            }
        };
        state
    }

    fn view(state: State) -> View<Action> {
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

fn main() {
    stdweb::initialize();
    StdwebRuntime::<CounterApp>::new(
        document().query_selector("body").unwrap().unwrap(),
        State::new(),
    ).run();
    stdweb::event_loop();
}
