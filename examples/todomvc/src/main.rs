#![feature(proc_macro)]

extern crate serde_json;
extern crate squark;
extern crate squark_macros;
extern crate squark_stdweb;
extern crate stdweb;

use stdweb::traits::*;
use stdweb::unstable::TryFrom;
use stdweb::web::document;
use stdweb::web::html_element::InputElement;
use squark::{handler, App, HandlerArg, View};
use squark_stdweb::Runtime;
use squark_macros::view;

#[derive(Clone, Hash, Debug, PartialEq)]
enum Visibility {
    All,
    Active,
    Completed,
}

impl ToString for Visibility {
    fn to_string(&self) -> String {
        match self {
            &Visibility::All => "All".to_string(),
            &Visibility::Active => "Active".to_string(),
            &Visibility::Completed => "Completed".to_string(),
        }
    }
}

impl Visibility {
    pub fn view(&self, selected: bool) -> View<Action> {
        let this = self.clone();
        let class = if selected { "selected" } else { "" };
        view! {
            <li>
                <a class={ class } style="cursor: pointer" onclick={ handler(self, move |_| { Some(Action::ChangeVisibility(this.clone())) }) }>
                    { self.to_string() }
                </a>
            </li>
        }
    }
}

#[derive(Clone, Debug)]
struct Entry {
    description: String,
    completed: bool,
}

impl Entry {
    pub fn new(description: String) -> Entry {
        Entry {
            description,
            completed: false,
        }
    }

    pub fn should_display(&self, visibility: &Visibility) -> bool {
        match visibility {
            &Visibility::All => true,
            &Visibility::Active => !self.completed,
            &Visibility::Completed => self.completed,
        }
    }

    pub fn view(&self, i: usize, editing: bool) -> View<Action> {
        let completed = self.completed.clone();
        let mut class = vec![];
        if completed {
            class.push("completed");
        }
        if editing {
            class.push("editing");
        }

        view! {
            <li class={ class.join(" ") }>
                {
                    if editing {
                        let id = format!("edit-{}", i);
                        view! {
                            <input
                                id={ id.clone() }
                                class="edit"
                                type="text"
                                value={ self.description.clone() }
                                oninput={ handler((), |v| match v {
                                    HandlerArg::String(v) => Some(Action::UpdateEntry(v)),
                                    _ => None,
                                }) }
                                onkeydown={ handler((), |v| match v {
                                    HandlerArg::String(ref v) if v.as_str() == "Enter" => {
                                        Some(Action::EndEditing)
                                    }
                                    _ => None,
                                }) }
                                onblur={ handler((), move |_| Some(Action::EndEditing)) }
                                onrender={ handler((), move |_| {
                                    InputElement::try_from(
                                        document().get_element_by_id(id.as_str()).unwrap()
                                    ).unwrap().focus();
                                    None
                                }) } />
                        }
                    } else {
                        view! {
                            <div class="view">
                                <input
                                    class="toggle"
                                    type="checkbox"
                                    checked={ completed }
                                    onclick={
                                        handler((i, completed), move |_| {
                                            Some(Action::Check(i, !completed))
                                        })
                                    }/>
                                <label ondblclick={
                                    handler(i, move |_| { Some(Action::EditEntry(i))
                                    })
                                }>
                                    { self.description.clone() }
                                </label>
                                <button class="destroy" onclick={ handler(i, move |_| { Some(Action::Remove(i)) }) } />
                            </div>
                        }
                    }
                }
            </li>
        }
    }
}

#[derive(Clone, Debug)]
struct State {
    entries: Vec<Entry>,
    field: String,
    editing: Option<usize>,
    visibility: Visibility,
}

impl State {
    pub fn new() -> State {
        State {
            entries: vec![],
            field: "".to_string(),
            editing: None,
            visibility: Visibility::All,
        }
    }

    pub fn has_completed(&self) -> bool {
        self.entries.iter().any(|e| e.completed)
    }

    pub fn not_completed_count(&self) -> usize {
        self.entries.len() - self.completed_count()
    }

    pub fn completed_count(&self) -> usize {
        self.entries.iter().filter(|e| e.completed).count()
    }

    pub fn is_all_completed(&self) -> bool {
        self.completed_count() == self.entries.len()
    }
}

#[derive(Clone, Debug)]
enum Action {
    UpdateField(String),
    EditEntry(usize),
    UpdateEntry(String),
    EndEditing,
    Add,
    Remove(usize),
    RemoveComplete,
    Check(usize, bool),
    CheckAll(bool),
    ChangeVisibility(Visibility),
}

fn header_view(state: &State) -> View<Action> {
    view! {
        <header class="header">
            <h1>todos</h1>
            <input
                class="new-todo"
                placeholder="What needs to be done?"
                value={ state.field.clone() }
                oninput={ handler((), |v| match v {
                    HandlerArg::String(v) => Some(Action::UpdateField(v)),
                    _ => None,
                }) }
                onkeydown={ handler((), |v| match v {
                    HandlerArg::String(ref v) if v.as_str() == "Enter" => Some(Action::Add),
                    _ => None,
                }) } />
        </header>
    }
}

fn main_view(state: &State) -> View<Action> {
    let is_all_completed = state.is_all_completed();
    view! {
        <section class="main">
            {
                if state.entries.len() > 0 {
                    view! {
                        <span>
                            <input
                                class="toggle-all"
                                type="checkbox"
                                checked={ is_all_completed }
                                onclick={
                                    handler(is_all_completed, move |_| {
                                        Some(Action::CheckAll(!is_all_completed))
                                    })
                                } />
                        </span>
                    }
                } else {
                    ().into()
                }

            }
            {
                View::new(
                    "ul".to_string(),
                    vec![
                        ("class".to_string(), "todo-list".into()),
                        ("type".to_string(), "checkbox".into()),
                    ],
                    vec![],
                    state
                        .entries
                        .iter()
                        .enumerate()
                        .filter(|&(_, e)| e.should_display(&state.visibility))
                        .map(|(i, e)| {
                            let is_editing = state.editing.as_ref().map_or(false, |at| &i == at);
                            e.view(i, is_editing)
                        })
                        .collect()
                )
            }
        </section>
    }
}

fn footer_view(state: &State) -> View<Action> {
    if state.entries.is_empty() {
        return ().into();
    }
    view! {
        <footer class="footer">
            <span class="todo-count">
                <strong>
                    { state.not_completed_count().to_string() }
                </strong>
                item(s) left
            </span>
            {
                View::new(
                    "ul".to_string(),
                    vec![
                        ("class".to_string(), "filters".into()),
                    ],
                    vec![],
                    vec![Visibility::All, Visibility::Active, Visibility::Completed]
                        .into_iter()
                        .map(|v| v.view(v == state.visibility))
                        .collect()
                )
            }
            {
                if state.has_completed() {
                    view! {
                        <button class="clear-completed" onclick={ handler((), move |_| { Some(Action::RemoveComplete) }) }>
                            Clear completed
                        </button>
                    }
                } else {
                    ().into()
                }
            }
        </footer>
    }
}

#[derive(Clone, Debug)]
struct TodoApp;
impl App for TodoApp {
    type State = State;
    type Action = Action;

    fn reducer(mut state: State, action: Action) -> State {
        match action {
            Action::Add => {
                if state.field.as_str() != "" {
                    let entry = Entry::new(state.field);
                    state.entries.push(entry);
                    state.field = "".to_string();
                }
            }
            Action::UpdateField(s) => {
                state.field = s;
            }
            Action::EndEditing => {
                if let Some(i) = state.editing {
                    if state.entries[i].description.as_str() == "" {
                        state.entries.remove(i);
                    }
                    state.editing = None;
                }
            }
            Action::UpdateEntry(s) => {
                if let Some(i) = state.editing {
                    state.entries[i].description = s;
                }
            }
            Action::CheckAll(b) => for mut entry in &mut state.entries {
                entry.completed = b;
            },
            Action::Check(at, b) => {
                state.entries[at].completed = b;
            }
            Action::Remove(at) => {
                state.entries.remove(at);
            }
            Action::RemoveComplete => {
                let entries = state.entries.drain(..).filter(|e| !e.completed).collect();
                state.entries = entries;
            }
            Action::EditEntry(i) => {
                state.editing = Some(i);
            }
            Action::ChangeVisibility(v) => {
                state.visibility = v;
            }
        };
        state
    }

    fn view(state: State) -> View<Action> {
        view! {
            <div>
                { header_view(&state) }
                { main_view(&state) }
                { footer_view(&state) }
            </div>
        }
    }
}

fn main() {
    stdweb::initialize();
    Runtime::<TodoApp>::new(
        document().query_selector("#container").unwrap().unwrap(),
        State::new(),
    ).start();
    stdweb::event_loop();
}
