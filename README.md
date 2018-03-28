# squark

Virtual DOM implemention and application definition inspired from [HyperApp](https://github.com/hyperapp/hyperapp/).

## squark-macro

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

Implemention for web browser with usinng [stdweb](https://github.com/koute/stdweb/).

Example code is available at [examples/todomvc](./examples/todomvc) and working on [https://rail44.github.io/squark/](https://rail44.github.io/squark/).
