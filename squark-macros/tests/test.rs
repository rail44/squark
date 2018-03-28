#![feature(test, proc_macro)]

extern crate squark;
extern crate squark_macros;

use squark_macros::view;
use squark::{View, null, text, handler};

fn v() -> View<()> {
    let not_completed_count = 1234;
    let has_completed = true;
    view! {
        <footer class="footer">
            <h1 class="todo-count">
                <strong>{ text(not_completed_count.to_string()) }</strong>
                 item(s) left
            </h1>
            <br />
            {
                if has_completed {
                    view! {
                        <button class="clear-completed" onclick={ handler((), move |_| { Some(()) }) }>
                            {
                                view! {
                                    <button class="clear-completed" onclick={ handler((), move |_| { Some(()) }) }>
                                        Clear completed
                                    </button>
                                }
                            }
                        </button>
                    }
                } else {
                    null()
                }
            }
        </footer>
    }
}

#[test]
fn it_works() {
    let v = v();
    println!("{:?}", v.0);
}
