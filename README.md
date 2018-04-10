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
  + For web browser by using [stdweb](https://github.com/koute/stdweb/).
  * For web browser by using [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen)*(working)*
  + Server side rendering within Rustic world *(working)*
* Macros like a JSX to help write view

## squark-macros

[![crates.io](https://img.shields.io/crates/v/squark-macros.svg)](https://crates.io/crates/squark-macros)
[![docs.rs](https://docs.rs/squark-macros/badge.svg)](https://docs.rs/squark-macros/*/squark_macros/)

Crate that providing JSX like macro by `proc_marco` and [pest](https://github.com/pest-parser/pest) parser.  

Note that it depends on Rust `nightly-2017-12-01` because of [language regssion](https://github.com/rust-lang/rust/issues/46489).

### Syntax

```   
view! {
    <button class="some-class" onclick={ |_| { Some(Action::Submit) }>
        Button!
    </button>
}
```

We can generate native Rust expression at compile-time.

## squark-stdweb

[![crates.io](https://img.shields.io/crates/v/squark-stdweb.svg)](https://crates.io/crates/squark-stdweb)
[![docs.rs](https://docs.rs/squark-stdweb/badge.svg)](https://docs.rs/squark-stdweb/*/squark_stdweb/)

Runtime implemention for web browser with usinng [stdweb](https://github.com/koute/stdweb/).

Here is full example of counter app!

```rust
#![feature(proc_macro)]

extern crate squark;
extern crate squark_macros;
extern crate squark_stdweb;
extern crate stdweb;

use stdweb::traits::*;
use stdweb::web::document;
use squark::{App, Runtime, View};
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
        let count = state.count.clone();
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
```

Project dir is located at [examples/counter](./examples/counter).
You can try it with `cargo-web`.

```sh
# on ./examples/counter
cargo install cargo-web
cargo toolchain install nightly-2017-12-01
cargo +nightly-2017-12-01 web start --target=wasm32-unknown-unknown
```

There is also available TodoMVC example at [examples/todomvc](./examples/todomvc) and working on [https://rail44.github.io/squark/](https://rail44.github.io/squark/).
