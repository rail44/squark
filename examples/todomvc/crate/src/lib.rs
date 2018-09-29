#![feature(proc_macro_non_items)]

extern crate serde_json;
extern crate squark;
extern crate squark_macros;
extern crate squark_web;
extern crate wasm_bindgen;

use squark::{App, Child, HandlerArg, Runtime, View, uuid};
use squark_macros::view;
use squark_web::WebRuntime;
use std::iter::FromIterator;
use wasm_bindgen::prelude::*;

#[derive(Clone, Hash, Debug, PartialEq)]
enum Visibility {
    All,
    Active,
    Completed,
}

impl ToString for Visibility {
    fn to_string(&self) -> String {
        match self {
            Visibility::All => "All".to_string(),
            Visibility::Active => "Active".to_string(),
            Visibility::Completed => "Completed".to_string(),
        }
    }
}

impl Visibility {
    pub fn view(&self, selected: bool) -> View<Action> {
        let this = self.clone();
        let class = if selected { "selected" } else { "" };
        view! {
            <li>
                <a class={ class } style="cursor: pointer" onclick={ move |_| { Some(Action::ChangeVisibility(this.clone())) } }>
                    { self.to_string() }
                </a>
            </li>
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Entry {
    id: String,
    description: String,
    completed: bool,
}

impl Entry {
    pub fn new(description: String) -> Entry {
        Entry {
            id: uuid(),
            description,
            completed: false,
        }
    }

    pub fn should_display(&self, visibility: &Visibility) -> bool {
        match visibility {
            Visibility::All => true,
            Visibility::Active => !self.completed,
            Visibility::Completed => self.completed,
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
            <li key={ self.id.clone() } class={ class.join(" ") }>
                {
                    if editing {
                        let id = format!("edit-{}", i);
                        view! {
                            <input
                                id={ id.clone() }
                                class="edit"
                                type="text"
                                value={ self.description.clone() }
                                oninput={ |v| match v {
                                    HandlerArg::String(v) => Some(Action::UpdateEntry(v)),
                                    _ => None,
                                } }
                                onkeydown={ |v| match v {
                                    HandlerArg::String(ref v) if v.as_str() == "Enter" => {
                                        Some(Action::EndEditing)
                                    }
                                    _ => None,
                                } }
                                onblur={ move |_| Some(Action::EndEditing) } />
                        }
                    } else {
                        view! {
                            <div class="view">
                                <input
                                    class="toggle"
                                    type="checkbox"
                                    checked={ completed }
                                    onclick={
                                        move |_| {
                                            Some(Action::Check(i, !completed))
                                        }
                                    }/>
                                <label ondblclick={
                                    move |_| { Some(Action::EditEntry(i))
                                    }
                                }>
                                    { self.description.clone() }
                                </label>
                                <button class="destroy" onclick={ move |_| { Some(Action::Remove(i)) } } />
                            </div>
                        }
                    }
                }
            </li>
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
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
                oninput={ |v| match v {
                    HandlerArg::String(v) => Some(Action::UpdateField(v)),
                    _ => None,
                } }
                onkeydown={ |v| match v {
                    HandlerArg::String(ref v) if v.as_str() == "Enter" => Some(Action::Add),
                    _ => None,
                } } />
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
                                    move |_| {
                                        Some(Action::CheckAll(!is_all_completed))
                                    }
                                } />
                        </span>
                    }
                } else {
                    ().into()
                }

            }
            <ul class="todo-list" type="checkbox">
                {
                    Child::from_iter(state
                        .entries
                        .iter()
                        .enumerate()
                        .filter(|&(_, e)| e.should_display(&state.visibility))
                        .map(|(i, e)| {
                            let is_editing = state.editing.as_ref().map_or(false, |at| &i == at);
                            e.view(i, is_editing)
                        }))
                }
            </ul>
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
            <ul class="filters">
                {
                    Child::from_iter(
                        vec![Visibility::All, Visibility::Active, Visibility::Completed]
                            .into_iter()
                            .map(|v| v.view(v == state.visibility))
                    )
                }
            </ul>
            {
                if state.has_completed() {
                    view! {
                        <button class="clear-completed" onclick={ move |_| { Some(Action::RemoveComplete) } }>
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

    fn reducer(&self, mut state: State, action: Action) -> State {
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

    fn view(&self, state: State) -> View<Action> {
        view! {
            <div>
                { header_view(&state) }
                { main_view(&state) }
                { footer_view(&state) }
            </div>
        }
    }
}

impl Default for TodoApp {
    fn default() -> TodoApp {
        TodoApp
    }
}

#[wasm_bindgen]
pub fn run() {
    WebRuntime::<TodoApp>::new(
        squark_web::web::document.query_selector("#container"),
        State::new(),
    ).run();
}
