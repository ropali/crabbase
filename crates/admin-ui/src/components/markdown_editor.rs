use crate::components::rich_text_renderer::{RICH_TEXT_CONTENT_CLASSES, rich_text_to_html};
use web_sys::{Element, HtmlTextAreaElement};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct MarkdownEditorProps {
    pub id: String,
    pub initial_value: Option<String>,
    pub on_change: Callback<String>,
}

#[derive(PartialEq, Clone, Copy)]
enum EditorTab {
    Write,
    Preview,
}

#[function_component(MarkdownEditor)]
pub fn markdown_editor(props: &MarkdownEditorProps) -> Html {
    let active_tab = use_state(|| EditorTab::Write);
    let value = use_state(|| props.initial_value.clone().unwrap_or_default());
    let on_change = props.on_change.clone();
    let preview_ref = use_node_ref();

    // Update preview HTML when switching to Preview tab
    {
        let value = value.clone();
        let active_tab = *active_tab;
        let preview_ref = preview_ref.clone();

        use_effect_with((active_tab, (*value).clone()), move |(tab, val)| {
            if *tab == EditorTab::Preview {
                if let Some(preview_el) = preview_ref.cast::<Element>() {
                    let html = if val.trim().is_empty() {
                        String::from(
                            "<p class=\"text-on-surface-variant italic\">Nothing to preview...</p>",
                        )
                    } else {
                        rich_text_to_html(val)
                    };
                    preview_el.set_inner_html(&html);
                }
            }
            || ()
        });
    }

    let on_input = {
        let value = value.clone();
        Callback::from(move |e: InputEvent| {
            let textarea: HtmlTextAreaElement = e.target_unchecked_into();
            let new_val = textarea.value();
            value.set(new_val.clone());
            on_change.emit(new_val);
        })
    };

    let set_tab_write = {
        let active_tab = active_tab.clone();
        Callback::from(move |_| active_tab.set(EditorTab::Write))
    };

    let set_tab_preview = {
        let active_tab = active_tab.clone();
        Callback::from(move |_| active_tab.set(EditorTab::Preview))
    };

    html! {
        <div class="flex flex-col gap-2 bg-surface-container-lowest/60 border border-outline-variant/60 rounded-xl p-3 shadow-sm">
            <div class="flex items-center gap-1 bg-surface-container-high/60 p-0.5 rounded-lg border border-outline-variant/40 self-start">
                <button
                    type="button"
                    onclick={set_tab_write}
                    class={if *active_tab == EditorTab::Write {
                        "px-3 py-1 rounded bg-surface font-label-sm text-[12px] font-bold text-secondary shadow-xs"
                    } else {
                        "px-3 py-1 rounded font-label-sm text-[12px] text-on-surface-variant hover:text-on-surface"
                    }}
                >
                    {"Write"}
                </button>
                <button
                    type="button"
                    onclick={set_tab_preview}
                    class={if *active_tab == EditorTab::Preview {
                        "px-3 py-1 rounded bg-surface font-label-sm text-[12px] font-bold text-secondary shadow-xs"
                    } else {
                        "px-3 py-1 rounded font-label-sm text-[12px] text-on-surface-variant hover:text-on-surface"
                    }}
                >
                    {"Preview"}
                </button>
            </div>

            <div class="mt-1">
                if *active_tab == EditorTab::Write {
                    <textarea
                        id={props.id.clone()}
                        class="w-full bg-white text-on-surface border border-outline-variant/60 rounded-xl p-3 font-mono text-sm outline-none focus:border-primary focus:ring-1 focus:ring-primary min-h-[300px] shadow-inner custom-scrollbar"
                        placeholder="Write Markdown here..."
                        value={(*value).clone()}
                        oninput={on_input}
                    />
                } else {
                    <div
                        ref={preview_ref}
                        class={classes!(
                            "w-full",
                            "min-h-[300px]",
                            "rounded-xl",
                            "border",
                            "border-outline-variant/60",
                            "bg-white",
                            "p-4",
                            "overflow-y-auto",
                            "shadow-inner",
                            RICH_TEXT_CONTENT_CLASSES
                        )}
                    />
                }
            </div>

            <div class="flex items-center justify-end px-1 pt-1 text-[11px] text-on-surface-variant">
                <span class="flex items-center gap-1">
                    <span class="material-symbols-outlined text-[14px]">{"markdown"}</span>
                    {"Supports "}
                    <a
                        href="https://www.markdownguide.org/"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="text-secondary hover:underline font-medium"
                    >
                        {"Markdown"}
                    </a>
                </span>
            </div>
        </div>
    }
}
