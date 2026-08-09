use crate::components::value_viewer_modal::render_json_value_html;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct JsonFieldSidebarProps {
    pub label: String,
    pub value: String,
    pub on_change: Callback<String>,
}

#[function_component(JsonFieldSidebar)]
pub fn json_field_sidebar(props: &JsonFieldSidebarProps) -> Html {
    let mode = use_state(|| "edit"); // "edit" or "preview"
    let copied = use_state(|| false);

    let parsed_json = use_memo(props.value.clone(), |val_str| {
        serde_json::from_str::<serde_json::Value>(val_str).ok()
    });

    let is_valid_json = parsed_json.is_some() || props.value.trim().is_empty();

    let on_format = {
        let value = props.value.clone();
        let on_change = props.on_change.clone();
        Callback::from(move |_| {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&value) {
                if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
                    on_change.emit(pretty);
                }
            }
        })
    };

    let on_copy = {
        let value = props.value.clone();
        let copied = copied.clone();
        Callback::from(move |_| {
            if let Some(window) = web_sys::window() {
                let navigator = window.navigator();
                let clipboard = navigator.clipboard();
                let _ = clipboard.write_text(&value);
                copied.set(true);

                let copied_reset = copied.clone();
                gloo_timers::callback::Timeout::new(2000, move || {
                    copied_reset.set(false);
                })
                .forget();
            }
        })
    };

    let on_input = {
        let on_change = props.on_change.clone();
        Callback::from(move |e: InputEvent| {
            let textarea: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            on_change.emit(textarea.value());
        })
    };

    let toggle_mode = {
        let mode = mode.clone();
        Callback::from(move |_| {
            if *mode == "edit" {
                mode.set("preview");
            } else {
                mode.set("edit");
            }
        })
    };

    html! {
        <div class="group flex flex-col gap-1.5 bg-surface-container-lowest/50 border border-outline-variant/60 rounded-xl p-3.5 shadow-sm">
            // Label & Actions Toolbar
            <div class="flex items-center justify-between">
                <div class="flex items-center gap-1.5">
                    <span class="material-symbols-outlined text-[16px] text-primary">{"data_object"}</span>
                    <span class="font-label-xs text-label-xs font-bold text-on-surface">{ &props.label }</span>
                    <span class="bg-primary/10 text-primary px-1.5 py-0.5 rounded text-[10px] uppercase font-bold tracking-wider">
                        {"JSON"}
                    </span>
                </div>

                <div class="flex items-center gap-1">
                    <button
                        type="button"
                        onclick={toggle_mode}
                        title={if *mode == "edit" { "Switch to Syntax Highlighting" } else { "Switch to Big Textbox Edit" }}
                        class="px-2 py-0.5 rounded bg-surface-container-high hover:bg-outline-variant text-on-surface-variant font-label-xs text-[11px] flex items-center gap-1 transition-colors"
                    >
                        <span class="material-symbols-outlined text-[13px]">
                            { if *mode == "edit" { "code" } else { "edit_note" } }
                        </span>
                        { if *mode == "edit" { "Highlight" } else { "Edit" } }
                    </button>

                    if parsed_json.is_some() {
                        <button
                            type="button"
                            onclick={on_format}
                            title="Format JSON with 2-space indentation"
                            class="px-2 py-0.5 rounded bg-surface-container-high hover:bg-outline-variant text-on-surface-variant font-label-xs text-[11px] flex items-center gap-1 transition-colors"
                        >
                            <span class="material-symbols-outlined text-[13px]">{"auto_fix_high"}</span>
                            {"Format"}
                        </button>
                    }

                    <button
                        type="button"
                        onclick={on_copy}
                        title="Copy JSON to clipboard"
                        class="px-2 py-0.5 rounded bg-surface-container-high hover:bg-outline-variant text-on-surface-variant font-label-xs text-[11px] flex items-center gap-1 transition-colors"
                    >
                        <span class="material-symbols-outlined text-[13px]">
                            { if *copied { "check" } else { "content_copy" } }
                        </span>
                        { if *copied { "Copied" } else { "Copy" } }
                    </button>
                </div>
            </div>

            // Editor / Viewer Body
            <div>
                {
                    if let Some(json_val) = parsed_json.as_ref() {
                        if *mode == "preview" {
                            html! {
                                <div class="bg-surface-container-lowest text-on-surface p-4 rounded-lg font-mono text-xs border border-outline-variant/60 overflow-x-auto max-h-[240px] custom-scrollbar select-text shadow-inner">
                                    { render_json_value_html(json_val, 0) }
                                </div>
                            }
                        } else {
                            html! {
                                <textarea
                                    class="w-full bg-white border border-outline-variant rounded-lg p-3 font-mono text-xs text-on-surface outline-none focus:border-primary h-[200px] custom-scrollbar"
                                    placeholder="{\n  \"key\": \"value\"\n}"
                                    value={props.value.clone()}
                                    oninput={on_input}
                                />
                            }
                        }
                    } else {
                        html! {
                            <textarea
                                class="w-full bg-white border border-outline-variant rounded-lg p-3 font-mono text-xs text-on-surface outline-none focus:border-primary h-[200px] custom-scrollbar"
                                placeholder="{\n  \"key\": \"value\"\n}"
                                value={props.value.clone()}
                                oninput={on_input}
                            />
                        }
                    }
                }
            </div>

            // Validation Footer
            <div class="flex items-center justify-between text-[11px] pt-1">
                <span class="text-on-surface-variant/70 font-mono">
                    { format!("{} chars", props.value.len()) }
                </span>
                if !props.value.trim().is_empty() {
                    if is_valid_json {
                        <span class="text-emerald-600 font-semibold flex items-center gap-1">
                            <span class="material-symbols-outlined text-[13px]">{"check_circle"}</span>
                            {"Valid JSON"}
                        </span>
                    } else {
                        <span class="text-amber-600 font-semibold flex items-center gap-1">
                            <span class="material-symbols-outlined text-[13px]">{"warning"}</span>
                            {"Invalid JSON"}
                        </span>
                    }
                }
            </div>
        </div>
    }
}
