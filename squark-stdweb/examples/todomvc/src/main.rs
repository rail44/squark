extern crate serde_json;
extern crate squark;
extern crate squark_stdweb;
extern crate stdweb;

use stdweb::traits::*;
use stdweb::unstable::TryFrom;
use stdweb::web::document;
use stdweb::web::html_element::InputElement;
use squark::{h, handler, null, text, App, HandlerArg, View};
use squark_stdweb::Runtime;

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
        let class = if selected {
            "selected".to_string()
        } else {
            "".to_string()
        };
        h(
            "li".to_string(),
            vec![],
            vec![],
            vec![
                h(
                    "a".to_string(),
                    vec![
                        ("class".to_string(), class),
                        ("style".to_string(), "cursor: pointer".to_string()),
                    ],
                    vec![
                        handler("click".to_string(), self, move |_| {
                            Some(Action::ChangeVisibility(this.clone()))
                        }),
                    ],
                    vec![text(self.to_string())],
                ),
            ],
        )
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
        let mut class = String::new();
        if completed {
            class += "completed";
        }
        if editing {
            class += "editing";
        }
        let mut toggle_attributes = vec![
            ("class".to_string(), "toggle".to_string()),
            ("type".to_string(), "checkbox".to_string()),
        ];
        if completed {
            toggle_attributes.push(("checked".to_string(), "true".to_string()));
        }

        h(
            "li".to_string(),
            vec![("class".to_string(), class)],
            vec![],
            vec![
                if editing {
                    let id = format!("edit-{}", i);
                    h(
                        "input".to_string(),
                        vec![
                            ("id".to_string(), id.clone()),
                            ("class".to_string(), "edit".to_string()),
                            ("type".to_string(), "text".to_string()),
                            ("value".to_string(), self.description.clone()),
                        ],
                        vec![
                            handler("input".to_string(), (), |v| match v {
                                HandlerArg::String(v) => Some(Action::UpdateEntry(v)),
                                _ => None,
                            }),
                            handler("keydown".to_string(), (), |v| match v {
                                HandlerArg::String(ref v) if v.as_str() == "Enter" => {
                                    Some(Action::EndEditing)
                                }
                                _ => None,
                            }),
                            handler("blur".to_string(), (), move |_| Some(Action::EndEditing)),
                            handler("render".to_string(), (), move |_| {
                                InputElement::try_from(
                                    document().get_element_by_id(id.as_str()).unwrap(),
                                ).unwrap()
                                    .focus();
                                None
                            }),
                        ],
                        vec![],
                    )
                } else {
                    h(
                        "div".to_string(),
                        vec![("class".to_string(), "view".to_string())],
                        vec![],
                        vec![
                            h(
                                "input".to_string(),
                                toggle_attributes,
                                vec![
                                    handler("click".to_string(), (i, completed), move |_| {
                                        Some(Action::Check(i, !completed))
                                    }),
                                ],
                                vec![],
                            ),
                            h(
                                "label".to_string(),
                                vec![],
                                vec![
                                    handler("dblclick".to_string(), i, move |_| {
                                        Some(Action::EditEntry(i))
                                    }),
                                ],
                                vec![text(self.description.clone())],
                            ),
                            h(
                                "button".to_string(),
                                vec![("class".to_string(), "destroy".to_string())],
                                vec![
                                    handler("click".to_string(), i, move |_| {
                                        Some(Action::Remove(i))
                                    }),
                                ],
                                vec![],
                            ),
                        ],
                    )
                },
            ],
        )
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
    h(
        "header".to_string(),
        vec![("class".to_string(), "header".to_string())],
        vec![],
        vec![
            h(
                "h1".to_string(),
                vec![],
                vec![],
                vec![text("todos".to_string())],
            ),
            h(
                "input".to_string(),
                vec![
                    ("class".to_string(), "new-todo".to_string()),
                    (
                        "placeholder".to_string(),
                        "What needs to be done?".to_string(),
                    ),
                    ("value".to_string(), state.field.clone()),
                ],
                vec![
                    handler("input".to_string(), (), |v| match v {
                        HandlerArg::String(v) => Some(Action::UpdateField(v)),
                        _ => None,
                    }),
                    handler("keydown".to_string(), (), |v| match v {
                        HandlerArg::String(ref v) if v.as_str() == "Enter" => Some(Action::Add),
                        _ => None,
                    }),
                ],
                vec![],
            ),
        ],
    )
}

fn main_view(state: &State) -> View<Action> {
    let is_all_completed = state.is_all_completed();
    h(
        "section".to_string(),
        vec![("class".to_string(), "main".to_string())],
        vec![],
        vec![
            if state.entries.len() > 0 {
                let mut attributes = vec![
                    ("class".to_string(), "toggle-all".to_string()),
                    ("type".to_string(), "checkbox".to_string()),
                ];
                if is_all_completed {
                    attributes.push(("checked".to_string(), "true".to_string()));
                }
                h(
                    "span".to_string(),
                    vec![],
                    vec![],
                    vec![
                        h(
                            "input".to_string(),
                            attributes,
                            vec![
                                handler("click".to_string(), is_all_completed, move |_| {
                                    Some(Action::CheckAll(!is_all_completed))
                                }),
                            ],
                            vec![],
                        ),
                    ],
                )
            } else {
                null()
            },
            h(
                "ul".to_string(),
                vec![
                    ("class".to_string(), "todo-list".to_string()),
                    ("type".to_string(), "checkbox".to_string()),
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
                    .collect(),
            ),
        ],
    )
}

fn footer_view(state: &State) -> View<Action> {
    if state.entries.is_empty() {
        return null();
    }
    h(
        "footer".to_string(),
        vec![("class".to_string(), "footer".to_string())],
        vec![],
        vec![
            h(
                "span".to_string(),
                vec![("class".to_string(), "todo-count".to_string())],
                vec![],
                vec![
                    h(
                        "strong".to_string(),
                        vec![],
                        vec![],
                        vec![text(state.not_completed_count().to_string())],
                    ),
                    text(" item(s) left".to_string()),
                ],
            ),
            h(
                "ul".to_string(),
                vec![("class".to_string(), "filters".to_string())],
                vec![],
                vec![Visibility::All, Visibility::Active, Visibility::Completed]
                    .into_iter()
                    .map(|v| v.view(v == state.visibility))
                    .collect(),
            ),
            if state.has_completed() {
                h(
                    "button".to_string(),
                    vec![("class".to_string(), "clear-completed".to_string())],
                    vec![
                        handler("click".to_string(), (), move |_| {
                            Some(Action::RemoveComplete)
                        }),
                    ],
                    vec![text("Clear completed".to_string())],
                )
            } else {
                null()
            },
        ],
    )
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
                    if state.entries[i].description.as_str() != "" {
                        state.editing = None;
                    }
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
        h(
            "div".to_string(),
            vec![],
            vec![],
            vec![header_view(&state), main_view(&state), footer_view(&state)],
        )
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
