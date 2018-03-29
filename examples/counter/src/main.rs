#![feature(proc_macro)]

extern crate squark;
extern crate squark_macros;
extern crate squark_stdweb;
extern crate stdweb;

use stdweb::traits::*;
use stdweb::web::document;
use squark::{handler, text, App, View};
use squark_stdweb::Runtime;
use squark_macros::view;

#[derive(Clone, Debug)]
struct State {
    count: isize,
}

impl State {
    pub fn new() -> State {
        State {
            count: 0,
        }
    }
}

#[derive(Clone, Debug)]
enum Action {
    Increment,
    Decrement,
}

#[derive(Clone, Debug)]
struct CounterApp;
impl App for CounterApp {
    type State = State;
    type Action = Action;

    fn reducer(mut state: State, action: Action) -> State {
        match action {
            Action::Increment => {
                state.count += 1;
            },
            Action::Decrement => {
                state.count -= 1;
            },
        };
        state
    }

    fn view(state: State) -> View<Action> {
        view! {
            <div>
                { text(state.count.to_string()) }
                <button onclick={ handler((), |_| Some(Action::Increment)) }>
                    increment
                </button>
                <button onclick={ handler((), |_| Some(Action::Decrement)) }>
                    decrement
                </button>
            </div>
        }
    }
}

fn main() {
    stdweb::initialize();
    Runtime::<CounterApp>::new(
        document().query_selector("body").unwrap().unwrap(),
        State::new(),
    ).start();
    stdweb::event_loop();
}
