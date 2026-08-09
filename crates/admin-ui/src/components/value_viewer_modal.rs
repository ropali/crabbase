use crate::components::rich_text_renderer::{RICH_TEXT_CONTENT_CLASSES, rich_text_to_html};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct ValueViewerModalProps {
    pub field_name: String,
    pub data_type: String,
    pub value: String,
    pub on_close: Callback<()>,
}

#[derive(PartialEq, Clone, Copy)]
enum ViewMode {
    Formatted,
    BigTextbox,
    Raw,
}

#[function_component(ValueViewerModal)]
pub fn value_viewer_modal(props: &ValueViewerModalProps) -> Html {
    let mode = use_state(|| ViewMode::Formatted);
    let copied = use_state(|| false);

    let parsed_json = use_memo(props.value.clone(), |val_str| {
        serde_json::from_str::<serde_json::Value>(val_str).ok()
    });

    let is_json_type = props.data_type.to_lowercase() == "json" || parsed_json.is_some();
    let is_rich_text_type = matches!(
        props.data_type.to_lowercase().as_str(),
        "richtext" | "editor"
    );

    let pretty_json_str = use_memo(parsed_json.clone(), |json_opt| {
        if let Some(v) = json_opt.as_ref() {
            serde_json::to_string_pretty(v).unwrap_or_else(|_| props.value.clone())
        } else {
            props.value.clone()
        }
    });

    let rendered_rich_text = use_memo(props.value.clone(), |value| rich_text_to_html(value));

    let on_close = props.on_close.clone();
    let on_backdrop_click = Callback::from(move |_| {
        on_close.emit(());
    });

    let on_modal_click = Callback::from(|e: MouseEvent| {
        e.stop_propagation();
    });

    let on_copy = {
        let copied = copied.clone();
        let text_to_copy = if is_json_type {
            (*pretty_json_str).clone()
        } else {
            props.value.clone()
        };
        Callback::from(move |_| {
            if let Some(window) = web_sys::window() {
                let navigator = window.navigator();
                let clipboard = navigator.clipboard();
                let _ = clipboard.write_text(&text_to_copy);
                copied.set(true);

                let copied_reset = copied.clone();
                gloo_timers::callback::Timeout::new(2000, move || {
                    copied_reset.set(false);
                })
                .forget();
            }
        })
    };

    let set_mode_formatted = {
        let mode = mode.clone();
        Callback::from(move |_| mode.set(ViewMode::Formatted))
    };

    let set_mode_textbox = {
        let mode = mode.clone();
        Callback::from(move |_| mode.set(ViewMode::BigTextbox))
    };

    let set_mode_raw = {
        let mode = mode.clone();
        Callback::from(move |_| mode.set(ViewMode::Raw))
    };

    let icon = match props.data_type.to_lowercase().as_str() {
        "json" => "data_object",
        "richtext" | "editor" => "article",
        "email" => "mail",
        "url" => "link",
        _ => "subject",
    };

    html! {
        <div onclick={on_backdrop_click.clone()} class="fixed inset-0 bg-inverse-surface/30 backdrop-blur-sm z-50 flex items-center justify-center p-4">
            <div onclick={on_modal_click} class="bg-surface border border-outline-variant rounded-2xl shadow-2xl w-full max-w-3xl flex flex-col max-h-[85vh] overflow-hidden animate-scale-in">
                // Header
                <div class="px-6 py-4 border-b border-outline-variant flex items-center justify-between bg-surface-container-low/50">
                    <div class="flex items-center gap-3">
                        <div class="w-9 h-9 rounded-xl bg-primary/10 text-primary flex items-center justify-center">
                            <span class="material-symbols-outlined text-xl">{ icon }</span>
                        </div>
                        <div>
                            <div class="flex items-center gap-2">
                                <h3 class="font-bold text-on-surface text-lg">{ &props.field_name }</h3>
                                <span class="bg-primary-container text-on-primary-container px-2 py-0.5 rounded-full font-label-xs text-[10px] uppercase font-bold tracking-wider">
                                    { &props.data_type }
                                </span>
                            </div>
                            <p class="font-label-xs text-label-xs text-on-surface-variant">
                                { format!("{} characters", props.value.len()) }
                            </p>
                        </div>
                    </div>

                    <div class="flex items-center gap-2">
                        <button
                            onclick={on_copy}
                            class="px-3 py-1.5 rounded-lg border border-outline-variant hover:bg-surface-container-high transition-colors font-label-xs text-label-xs flex items-center gap-1.5 text-on-surface"
                        >
                            <span class="material-symbols-outlined text-sm">
                                { if *copied { "check" } else { "content_copy" } }
                            </span>
                            { if *copied { "Copied!" } else { "Copy" } }
                        </button>
                        <button onclick={let cb = props.on_close.clone(); move |_| cb.emit(())} class="p-1.5 hover:bg-surface-container-high rounded-full transition-colors text-on-surface-variant">
                            <span class="material-symbols-outlined">{"close"}</span>
                        </button>
                    </div>
                </div>

                // Toolbar / View Mode Switcher
                <div class="px-6 py-2.5 border-b border-outline-variant/60 bg-surface-container-lowest flex items-center justify-between">
                    <div class="flex items-center gap-1 bg-surface-container-high/60 p-1 rounded-xl">
                        if is_json_type {
                            <button
                                onclick={set_mode_formatted}
                                class={if *mode == ViewMode::Formatted {
                                    "px-3 py-1 rounded-lg bg-surface font-label-xs text-label-xs font-bold text-primary shadow-sm transition-all"
                                } else {
                                    "px-3 py-1 rounded-lg font-label-xs text-label-xs text-on-surface-variant hover:text-on-surface transition-all"
                                }}
                            >
                                <div class="flex items-center gap-1">
                                    <span class="material-symbols-outlined text-xs">{"code"}</span>
                                    {"Formatted JSON"}
                                </div>
                            </button>
                        } else if is_rich_text_type {
                            <button
                                onclick={set_mode_formatted}
                                class={if *mode == ViewMode::Formatted {
                                    "px-3 py-1 rounded-lg bg-surface font-label-xs text-label-xs font-bold text-primary shadow-sm transition-all"
                                } else {
                                    "px-3 py-1 rounded-lg font-label-xs text-label-xs text-on-surface-variant hover:text-on-surface transition-all"
                                }}
                            >
                                <div class="flex items-center gap-1">
                                    <span class="material-symbols-outlined text-xs">{"article"}</span>
                                    {"Rendered Markdown"}
                                </div>
                            </button>
                        }
                        <button
                            onclick={set_mode_textbox}
                            class={if *mode == ViewMode::BigTextbox || (!is_json_type && !is_rich_text_type && *mode == ViewMode::Formatted) {
                                "px-3 py-1 rounded-lg bg-surface font-label-xs text-label-xs font-bold text-primary shadow-sm transition-all"
                            } else {
                                "px-3 py-1 rounded-lg font-label-xs text-label-xs text-on-surface-variant hover:text-on-surface transition-all"
                            }}
                        >
                            <div class="flex items-center gap-1">
                                <span class="material-symbols-outlined text-xs">{"edit_note"}</span>
                                {"Big Textbox"}
                            </div>
                        </button>
                        <button
                            onclick={set_mode_raw}
                            class={if *mode == ViewMode::Raw {
                                "px-3 py-1 rounded-lg bg-surface font-label-xs text-label-xs font-bold text-primary shadow-sm transition-all"
                            } else {
                                "px-3 py-1 rounded-lg font-label-xs text-label-xs text-on-surface-variant hover:text-on-surface transition-all"
                            }}
                        >
                            <div class="flex items-center gap-1">
                                <span class="material-symbols-outlined text-xs">{"notes"}</span>
                                {"Raw Text"}
                            </div>
                        </button>
                    </div>

                    <span class="text-body-xs font-body-xs text-on-surface-variant">
                        if is_json_type {
                            {"Valid JSON"}
                        } else if is_rich_text_type {
                            {"Markdown Content"}
                        } else {
                            {"Text Content"}
                        }
                    </span>
                </div>

                // Content Area
                <div class="flex-1 p-6 overflow-y-auto bg-surface-container-lowest custom-scrollbar">
                    {
                        match *mode {
                            ViewMode::Formatted if is_json_type => {
                                if let Some(json_val) = parsed_json.as_ref() {
                                    html! {
                                        <div class="bg-surface text-on-surface p-5 rounded-xl font-mono text-sm border border-outline-variant/60 overflow-x-auto shadow-inner select-text">
                                            { render_json_value_html(json_val, 0) }
                                        </div>
                                    }
                                } else {
                                    html! {
                                        <textarea
                                            readonly=true
                                            class="w-full h-[360px] font-mono text-sm bg-surface border border-outline-variant rounded-xl p-4 text-on-surface focus:outline-none custom-scrollbar select-text"
                                            value={props.value.clone()}
                                        />
                                    }
                                }
                            }
                            ViewMode::Formatted if is_rich_text_type => {
                                html! {
                                    <div class="bg-white p-5 rounded-xl border border-outline-variant/40 shadow-inner min-h-[360px]">
                                        if props.value.trim().is_empty() {
                                            <p class="text-on-surface-variant italic">{"Nothing to preview yet."}</p>
                                        } else {
                                            <div class={RICH_TEXT_CONTENT_CLASSES}>
                                                { Html::from_html_unchecked(AttrValue::from((*rendered_rich_text).clone())) }
                                            </div>
                                        }
                                    </div>
                                }
                            }
                            ViewMode::Raw => {
                                html! {
                                    <pre class="bg-surface-container-high/40 p-5 rounded-xl font-mono text-sm text-on-surface border border-outline-variant/40 overflow-x-auto whitespace-pre-wrap select-text">
                                        { &props.value }
                                    </pre>
                                }
                            }
                            _ => {
                                let display_text = if is_json_type {
                                    (*pretty_json_str).clone()
                                } else {
                                    props.value.clone()
                                };
                                html! {
                                    <div class="flex flex-col gap-2">
                                        <textarea
                                            readonly=true
                                            class="w-full h-[360px] font-mono text-sm bg-surface border border-outline-variant rounded-xl p-4 text-on-surface focus:outline-none custom-scrollbar select-text"
                                            value={display_text}
                                        />
                                    </div>
                                }
                            }
                        }
                    }
                </div>

                // Footer
                <div class="px-6 py-3 border-t border-outline-variant bg-surface-container-low/30 flex justify-end">
                    <button
                        onclick={let cb = props.on_close.clone(); move |_| cb.emit(())}
                        class="px-6 py-2 bg-primary text-on-primary font-bold rounded-lg text-body-sm hover:bg-primary-container transition-colors shadow-sm"
                    >
                        {"Done"}
                    </button>
                </div>
            </div>
        </div>
    }
}

pub fn render_json_value_html(val: &serde_json::Value, indent_level: usize) -> Html {
    match val {
        serde_json::Value::Null => html! {
            <span class="text-error italic font-mono">{"null"}</span>
        },
        serde_json::Value::Bool(b) => html! {
            <span class="text-primary font-bold font-mono">{ b.to_string() }</span>
        },
        serde_json::Value::Number(n) => html! {
            <span class="text-tertiary font-mono">{ n.to_string() }</span>
        },
        serde_json::Value::String(s) => html! {
            <span class="text-secondary font-mono">{ format!("\"{}\"", s) }</span>
        },
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                html! { <span class="text-on-surface-variant font-mono">{"[]"}</span> }
            } else {
                html! {
                    <span class="font-mono">
                        <span class="text-on-surface-variant">{"["}</span>
                        <div class="pl-4 border-l border-outline-variant my-0.5">
                            { for arr.iter().enumerate().map(|(idx, item)| {
                                let is_last = idx == arr.len() - 1;
                                html! {
                                    <div class="py-0.5">
                                        { render_json_value_html(item, indent_level + 1) }
                                        if !is_last { <span class="text-on-surface-variant">{","}</span> }
                                    </div>
                                }
                            }) }
                        </div>
                        <span class="text-on-surface-variant">{"]"}</span>
                    </span>
                }
            }
        }
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                html! { <span class="text-on-surface-variant font-mono">{"{}"}</span> }
            } else {
                let keys: Vec<_> = map.keys().collect();
                html! {
                    <span class="font-mono">
                        <span class="text-on-surface-variant">{"{"}</span>
                        <div class="pl-4 border-l border-outline-variant my-0.5">
                            { for keys.iter().enumerate().map(|(idx, key)| {
                                let item = &map[*key];
                                let is_last = idx == keys.len() - 1;
                                html! {
                                    <div class="py-0.5 flex flex-wrap items-start gap-1">
                                        <span class="text-on-surface font-bold">{ format!("\"{}\"", key) }</span>
                                        <span class="text-on-surface-variant">{":"} </span>
                                        { render_json_value_html(item, indent_level + 1) }
                                        if !is_last { <span class="text-on-surface-variant">{","}</span> }
                                    </div>
                                }
                            }) }
                        </div>
                        <span class="text-on-surface-variant">{"}"}</span>
                    </span>
                }
            }
        }
    }
}
