# squark

[![crates.io](https://img.shields.io/crates/v/squark.svg)](https://crates.io/crates/squark)
[![docs.rs](https://docs.rs/squark/badge.svg)](https://docs.rs/squark/*/squark/)

Virtual DOM implemention and definitions of Application and Runtime.

## Fertures

This repository includes

* Pure Rust virtual DOM implemention
* Definition of application inspired from [HyperApp](https://github.com/hyperapp/hyperapp/)
* Definition of runtime to handle diffirence of virtual DOM
* Runtime implementions for several platforms
  * For web browser by using [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen)
  + Server side rendering within Rustic world *(now working)*
* Macros like a JSX to help writing view

**Currently, we depend on `nightly` channel**

## squark-macros

[![crates.io](https://img.shields.io/crates/v/squark-macros.svg)](https://crates.io/crates/squark-macros)
[![docs.rs](https://docs.rs/squark-macros/badge.svg)](https://docs.rs/squark-macros/*/squark_macros/)

Crate that providing JSX like macro by `proc_marco` and [pest](https://github.com/pest-parser/pest) parser.  

### Syntax

```   
view! {
    <button class="some-class" onclick={ |_| { Some(Action::Submit) }>
        Button!
    </button>
}
```

We can generate native Rust expression at compile-time.

## squark-web

[![crates.io](https://img.shields.io/crates/v/squark-web.svg)](https://crates.io/crates/squark-web)
[![docs.rs](https://docs.rs/squark-web/badge.svg)](https://docs.rs/squark-web/*/squark_web/)

Runtime implemention for web browser with usinng [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen/).

Here is full example of counter app!

```rust
#![feature(proc_macro_non_items)]

extern crate squark;
extern crate squark_macros;
extern crate squark_web;
extern crate wasm_bindgen;

use squark::{App, Runtime, View};
use squark_macros::view;
use squark_web::WebRuntime;
use wasm_bindgen::prelude::*;

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

#[wasm_bindgen]
pub fn run() {
    WebRuntime::<CounterApp>::new(
        squark_web::web::document.query_selector("body"),
        State::new(),
    ).run();
}
```

Project dir is located at [examples/counter](./examples/counter).

There is also available TodoMVC example at [examples/todomvc](./examples/todomvc) and working on [https://rail44.github.io/squark/](https://rail44.github.io/squark/).
