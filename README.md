# squark

[![crates.io](https://img.shields.io/crates/v/squark.svg)](https://crates.io/crates/squark)
[![docs.rs](https://docs.rs/squark/badge.svg)](https://docs.rs/squark/*/squark/)

Virtual DOM implemention and application definition inspired from [HyperApp](https://github.com/hyperapp/hyperapp/).

## squark-macros

[![crates.io](https://img.shields.io/crates/v/squark-macros.svg)](https://crates.io/crates/squark-macros)
[![docs.rs](https://docs.rs/squark-macros/badge.svg)](https://docs.rs/squark-macros/*/squark-macros/)

Crate that providing JSX like macro by `proc_marco` and [pest](https://github.com/pest-parser/pest) parser.

### Syntax

```   
view! {
    <button class="clear-completed" onclick={ handler((), move |_| { Some(Action::Submit) }) }>
        Submit
    </button>
}
```

## squark-stdweb

[![crates.io](https://img.shields.io/crates/v/squark-stdweb.svg)](https://crates.io/crates/squark-stdweb)
[![docs.rs](https://docs.rs/squark-stdweb/badge.svg)](https://docs.rs/squark-stdweb/*/squark-stdweb/)

Squark runtime implemention for web browser with usinng [stdweb](https://github.com/koute/stdweb/).

Here is full example of counter app!

```rust
#![feature(proc_macro)]

extern crate squark;
extern crate squark_macros;
extern crate squark_stdweb;
extern crate stdweb;

use stdweb::traits::*;
use stdweb::web::document;
use squark::{handler, App, View, Runtime};
use squark_stdweb::StdwebRuntime;
use squark_macros::view;

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
            }
            Action::Decrement => {
                state.count -= 1;
            }
        };
        state
    }

    fn view(state: State) -> View<Action> {
        view! {
            <div>
                { state.count.to_string() }
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
    StdwebRuntime::<CounterApp>::new(
        document().query_selector("body").unwrap().unwrap(),
        State::new(),
    ).run();
    stdweb::event_loop();
}
```

Project dir is located at [examples/counter](./examples/counter).

It is also available TodoMVC example at [examples/todomvc](./examples/todomvc) and working on [https://rail44.github.io/squark/](https://rail44.github.io/squark/).
