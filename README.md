# squark

Rust frontend framework, for web browser and more.

**Currently, we depend on `nightly` channel**

## Design

* Separating runtime definition and implemention
  + `squark` crate has no dependency for specific platform
* Architecture inspired from [Elm](https://elm-lang.org/) and [HyperApp](https://github.com/hyperapp/hyperapp/)
  + Simplicy
  + Elegant
* Supporting futures-0.1
  + reducer can emit task for async work such as fetch resource

## crates

### squark

[![crates.io](https://img.shields.io/crates/v/squark.svg)](https://crates.io/crates/squark)
[![docs.rs](https://docs.rs/squark/badge.svg)](https://docs.rs/squark/*/squark/)

Core crate.

* Pure Rust virtual DOM implemention
* Definition of GUI application
* Definition of runtime to handle diffirence of virtual DOM

### squark-macros

[![crates.io](https://img.shields.io/crates/v/squark-macros.svg)](https://crates.io/crates/squark-macros)
[![docs.rs](https://docs.rs/squark-macros/badge.svg)](https://docs.rs/squark-macros/*/squark_macros/)

It provides macro like JSX for helping writing view.  
Very thanks to [pest](https://github.com/pest-parser/pest) parser.

#### Syntax

```   
view! {
    <button class="some-class" onclick={ |_| Some(Action::Submit) }>
        Button!
    </button>
}
```

We can generate native Rust expression at compile-time.


### squark-web

[![crates.io](https://img.shields.io/crates/v/squark-web.svg)](https://crates.io/crates/squark-web)
[![docs.rs](https://docs.rs/squark-web/badge.svg)](https://docs.rs/squark-web/*/squark_web/)

Runtime implemention for web browser with usinng [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen/).

Here is full example of counter app!

```rust
#![feature(proc_macro_hygiene)]

extern crate squark;
extern crate squark_macros;
extern crate squark_web;
extern crate wasm_bindgen;
extern crate web_sys;

use squark::{App, Runtime, View, Task};
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

    fn reducer(&self, mut state: State, action: Action) -> (State, Task<Action>) {
        match action {
            Action::ChangeCount(c) => {
                state.count = c;
            }
        };
        (state, Task::empty())
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
```

Project dir is located at [examples/counter](./examples/counter).

There are some other examples available on [examples](./examples), most of them use [rust-webpack-template](https://github.com/rustwasm/rust-webpack-template).  
TodoMVC is working on [https://rail44.github.io/squark/](https://rail44.github.io/squark/).
