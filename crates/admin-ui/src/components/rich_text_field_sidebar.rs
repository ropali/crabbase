use crate::components::rich_text_renderer::{RICH_TEXT_CONTENT_CLASSES, rich_text_to_html};
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlDocument, HtmlElement, HtmlTextAreaElement, Node};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct RichTextFieldSidebarProps {
    pub label: String,
    pub value: String,
    pub on_change: Callback<String>,
}

#[derive(PartialEq, Clone, Copy)]
enum EditorTab {
    Visual,
    Markdown,
}

#[derive(Clone, Copy)]
enum VisualCommand {
    Bold,
    Italic,
    Heading1,
    Heading2,
    BulletList,
    Quote,
    CodeBlock,
    Link,
}

#[derive(Clone, Copy)]
struct SelectionState {
    inside_editor: bool,
    collapsed: bool,
}

#[function_component(RichTextFieldSidebar)]
pub fn rich_text_field_sidebar(props: &RichTextFieldSidebarProps) -> Html {
    let active_tab = use_state(|| EditorTab::Visual);
    let copied = use_state(|| false);
    let markdown_value = use_state(|| props.value.clone());
    let visual_editor_ref = use_node_ref();
    let should_sync_visual = use_mut_ref(|| true);

    {
        let markdown_value = markdown_value.clone();
        let should_sync_visual = should_sync_visual.clone();
        use_effect_with(props.value.clone(), move |external_value| {
            if *markdown_value != *external_value {
                *should_sync_visual.borrow_mut() = true;
                markdown_value.set(external_value.clone());
            }
            || ()
        });
    }

    {
        let visual_editor_ref = visual_editor_ref.clone();
        let should_sync_visual = should_sync_visual.clone();
        use_effect_with(
            ((*markdown_value).clone(), *active_tab),
            move |(markdown, tab)| {
                if *tab == EditorTab::Visual && *should_sync_visual.borrow() {
                    if let Some(editor) = visual_editor_ref.cast::<Element>() {
                        let rendered_html = if markdown.trim().is_empty() {
                            "<p></p>".to_string()
                        } else {
                            rich_text_to_html(markdown)
                        };
                        editor.set_inner_html(&rendered_html);
                    }
                    *should_sync_visual.borrow_mut() = false;
                }
                || ()
            },
        );
    }

    let sync_markdown_from_visual: Rc<dyn Fn()> = {
        let visual_editor_ref = visual_editor_ref.clone();
        let markdown_value = markdown_value.clone();
        let on_change = props.on_change.clone();
        let should_sync_visual = should_sync_visual.clone();
        Rc::new(move || {
            let Some(editor) = visual_editor_ref.cast::<Element>() else {
                return;
            };

            let new_value = visual_editor_to_markdown(&editor);
            if *markdown_value != new_value {
                *should_sync_visual.borrow_mut() = false;
                markdown_value.set(new_value.clone());
                on_change.emit(new_value);
            }
        })
    };

    let on_visual_input = {
        let sync_markdown_from_visual = sync_markdown_from_visual.clone();
        Callback::from(move |_e: InputEvent| {
            sync_markdown_from_visual();
        })
    };

    let on_raw_input = {
        let markdown_value = markdown_value.clone();
        let on_change = props.on_change.clone();
        let should_sync_visual = should_sync_visual.clone();
        Callback::from(move |e: InputEvent| {
            let textarea: HtmlTextAreaElement = e.target_unchecked_into();
            let new_value = textarea.value();
            *should_sync_visual.borrow_mut() = true;
            markdown_value.set(new_value.clone());
            on_change.emit(new_value);
        })
    };

    let apply_visual_command: Rc<dyn Fn(VisualCommand)> = {
        let visual_editor_ref = visual_editor_ref.clone();
        let sync_markdown_from_visual = sync_markdown_from_visual.clone();
        Rc::new(move |command| {
            let Some(editor) = visual_editor_ref.cast::<HtmlElement>() else {
                return;
            };

            execute_visual_command(&editor, command);
            sync_markdown_from_visual();
        })
    };

    let make_toolbar_handler = |command| {
        let apply_visual_command = apply_visual_command.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();
            apply_visual_command(command);
        })
    };

    let btn_bold = make_toolbar_handler(VisualCommand::Bold);
    let btn_italic = make_toolbar_handler(VisualCommand::Italic);
    let btn_h1 = make_toolbar_handler(VisualCommand::Heading1);
    let btn_h2 = make_toolbar_handler(VisualCommand::Heading2);
    let btn_ul = make_toolbar_handler(VisualCommand::BulletList);
    let btn_quote = make_toolbar_handler(VisualCommand::Quote);
    let btn_code = make_toolbar_handler(VisualCommand::CodeBlock);
    let btn_link = make_toolbar_handler(VisualCommand::Link);

    let on_copy = {
        let markdown_value = markdown_value.clone();
        let copied = copied.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();
            if let Some(window) = web_sys::window() {
                let navigator = window.navigator();
                let clipboard = navigator.clipboard();
                let text_to_copy = (*markdown_value).clone();
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

    let set_tab_visual = {
        let active_tab = active_tab.clone();
        Callback::from(move |_| active_tab.set(EditorTab::Visual))
    };

    let set_tab_markdown = {
        let active_tab = active_tab.clone();
        Callback::from(move |_| active_tab.set(EditorTab::Markdown))
    };

    let editor_text = (*markdown_value).clone();
    let word_count = editor_text.split_whitespace().count();
    let char_count = editor_text.chars().count();
    let has_content = !editor_text.trim().is_empty();

    html! {
        <div class="group flex flex-col gap-2 bg-surface-container-lowest/60 border border-outline-variant/60 rounded-xl p-3.5 shadow-sm">
            <div class="flex items-center justify-between gap-3">
                <div class="flex items-center gap-1.5">
                    <span class="material-symbols-outlined text-[16px] text-secondary">{"article"}</span>
                    <span class="font-label-xs text-label-xs font-bold text-on-surface">{ &props.label }</span>
                    <span class="bg-secondary-container text-on-secondary-container px-1.5 py-0.5 rounded text-[10px] uppercase font-bold tracking-wider">
                        {"Markdown"}
                    </span>
                </div>

                <div class="flex items-center gap-1">
                    <div class="flex items-center bg-surface-container-high/60 p-0.5 rounded-lg border border-outline-variant/40">
                        <button
                            type="button"
                            onclick={set_tab_visual}
                            class={if *active_tab == EditorTab::Visual {
                                "px-2.5 py-0.5 rounded bg-surface font-label-xs text-[11px] font-bold text-secondary shadow-xs"
                            } else {
                                "px-2.5 py-0.5 rounded font-label-xs text-[11px] text-on-surface-variant hover:text-on-surface"
                            }}
                        >
                            {"Visual Editor"}
                        </button>
                        <button
                            type="button"
                            onclick={set_tab_markdown}
                            class={if *active_tab == EditorTab::Markdown {
                                "px-2.5 py-0.5 rounded bg-surface font-label-xs text-[11px] font-bold text-secondary shadow-xs"
                            } else {
                                "px-2.5 py-0.5 rounded font-label-xs text-[11px] text-on-surface-variant hover:text-on-surface"
                            }}
                        >
                            {"Raw Markdown"}
                        </button>
                    </div>

                    <button
                        type="button"
                        onclick={on_copy}
                        title="Copy markdown"
                        class="px-2 py-0.5 rounded bg-surface-container-high hover:bg-outline-variant text-on-surface-variant font-label-xs text-[11px] flex items-center gap-1 transition-colors"
                    >
                        <span class="material-symbols-outlined text-[13px]">
                            { if *copied { "check" } else { "content_copy" } }
                        </span>
                        { if *copied { "Copied" } else { "Copy MD" } }
                    </button>
                </div>
            </div>

            if *active_tab == EditorTab::Visual {
                <>
                    <div class="flex items-center gap-1 bg-surface-container-low p-1.5 rounded-lg border border-outline-variant/60 flex-wrap shadow-inner">
                        <button
                            type="button"
                            onmousedown={btn_bold}
                            title="Bold"
                            class="w-7 h-7 rounded flex items-center justify-center font-bold text-xs hover:bg-primary/10 hover:text-primary text-on-surface transition-colors border border-outline-variant/30 active:scale-95"
                        >
                            {"B"}
                        </button>
                        <button
                            type="button"
                            onmousedown={btn_italic}
                            title="Italic"
                            class="w-7 h-7 rounded flex items-center justify-center italic font-serif text-xs hover:bg-primary/10 hover:text-primary text-on-surface transition-colors border border-outline-variant/30 active:scale-95"
                        >
                            {"I"}
                        </button>
                        <div class="w-px h-4 bg-outline-variant/60 mx-0.5"></div>
                        <button
                            type="button"
                            onmousedown={btn_h1}
                            title="Heading 1"
                            class="px-1.5 h-7 rounded flex items-center justify-center font-bold text-xs hover:bg-primary/10 hover:text-primary text-on-surface transition-colors border border-outline-variant/30 active:scale-95"
                        >
                            {"H1"}
                        </button>
                        <button
                            type="button"
                            onmousedown={btn_h2}
                            title="Heading 2"
                            class="px-1.5 h-7 rounded flex items-center justify-center font-bold text-xs hover:bg-primary/10 hover:text-primary text-on-surface transition-colors border border-outline-variant/30 active:scale-95"
                        >
                            {"H2"}
                        </button>
                        <div class="w-px h-4 bg-outline-variant/60 mx-0.5"></div>
                        <button
                            type="button"
                            onmousedown={btn_ul}
                            title="Bullet list"
                            class="w-7 h-7 rounded flex items-center justify-center hover:bg-primary/10 hover:text-primary text-on-surface transition-colors border border-outline-variant/30 active:scale-95"
                        >
                            <span class="material-symbols-outlined text-[16px]">{"format_list_bulleted"}</span>
                        </button>
                        <button
                            type="button"
                            onmousedown={btn_quote}
                            title="Quote"
                            class="w-7 h-7 rounded flex items-center justify-center hover:bg-primary/10 hover:text-primary text-on-surface transition-colors border border-outline-variant/30 active:scale-95"
                        >
                            <span class="material-symbols-outlined text-[16px]">{"format_quote"}</span>
                        </button>
                        <button
                            type="button"
                            onmousedown={btn_code}
                            title="Code block"
                            class="w-7 h-7 rounded flex items-center justify-center hover:bg-primary/10 hover:text-primary text-on-surface transition-colors border border-outline-variant/30 active:scale-95"
                        >
                            <span class="material-symbols-outlined text-[16px]">{"code"}</span>
                        </button>
                        <button
                            type="button"
                            onmousedown={btn_link}
                            title="Link"
                            class="w-7 h-7 rounded flex items-center justify-center hover:bg-primary/10 hover:text-primary text-on-surface transition-colors border border-outline-variant/30 active:scale-95"
                        >
                            <span class="material-symbols-outlined text-[16px]">{"link"}</span>
                        </button>
                        <span class="ml-auto text-[11px] text-on-surface-variant/80">
                            {"Formatting applies directly to the rendered content"}
                        </span>
                    </div>

                    <div class="flex flex-col gap-2">
                        <div class="flex items-center justify-between text-[11px] text-on-surface-variant">
                            <span class="font-semibold uppercase tracking-wider">{"Visual Content"}</span>
                            <span>{"Rendered editor first, raw Markdown available in the source tab"}</span>
                        </div>

                        <div class="relative">
                            if !has_content {
                                <div class="pointer-events-none absolute left-4 top-4 text-on-surface-variant italic">
                                    {"Type directly into the rendered editor or use the toolbar to insert formatted content."}
                                </div>
                            }
                            <div
                                ref={visual_editor_ref.clone()}
                                contenteditable="true"
                                spellcheck="true"
                                role="textbox"
                                tabindex="0"
                                oninput={on_visual_input}
                                class={classes!(
                                    "w-full",
                                    "min-h-[320px]",
                                    "max-h-[360px]",
                                    "overflow-y-auto",
                                    "rounded-xl",
                                    "border",
                                    "border-outline-variant/60",
                                    "bg-white",
                                    "p-4",
                                    "shadow-inner",
                                    "outline-none",
                                    "focus:border-primary",
                                    "focus:ring-1",
                                    "focus:ring-primary",
                                    "custom-scrollbar",
                                    RICH_TEXT_CONTENT_CLASSES
                                )}
                            />
                        </div>
                    </div>
                </>
            } else {
                <div class="flex flex-col gap-2">
                    <div class="flex items-center justify-between text-[11px] text-on-surface-variant">
                        <span class="font-semibold uppercase tracking-wider">{"Markdown Source"}</span>
                        <span>{"Direct access to the stored Markdown value"}</span>
                    </div>
                    <textarea
                        class="w-full bg-[#1e1e2e] text-[#cdd6f4] border border-outline-variant/60 rounded-xl p-3 font-mono text-xs outline-none focus:border-secondary h-[320px] custom-scrollbar"
                        placeholder="Write Markdown here..."
                        value={(*markdown_value).clone()}
                        oninput={on_raw_input}
                    />
                </div>
            }

            <div class="flex items-center justify-between text-[11px] text-on-surface-variant/75 pt-0.5">
                <span class="font-mono">{ format!("{} words | {} chars", word_count, char_count) }</span>
                <span class="font-medium text-secondary flex items-center gap-1">
                    <span class="material-symbols-outlined text-[13px]">{"markdown"}</span>
                    {"Visual Markdown Editor"}
                </span>
            </div>
        </div>
    }
}

fn execute_visual_command(editor: &HtmlElement, command: VisualCommand) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let html_document = document.unchecked_into::<HtmlDocument>();
    let selection_state = current_selection_state(editor);

    let _ = editor.focus();

    match command {
        VisualCommand::Bold => {
            if selection_state.inside_editor && !selection_state.collapsed {
                let _ = html_document.exec_command("bold");
            } else {
                let _ = html_document.exec_command_with_show_ui_and_value(
                    "insertHTML",
                    false,
                    "<strong>bold text</strong>",
                );
            }
        }
        VisualCommand::Italic => {
            if selection_state.inside_editor && !selection_state.collapsed {
                let _ = html_document.exec_command("italic");
            } else {
                let _ = html_document.exec_command_with_show_ui_and_value(
                    "insertHTML",
                    false,
                    "<em>italic text</em>",
                );
            }
        }
        VisualCommand::Heading1 => {
            if selection_state.inside_editor && !selection_state.collapsed {
                let _ =
                    html_document.exec_command_with_show_ui_and_value("formatBlock", false, "<h1>");
            } else {
                let _ = html_document.exec_command_with_show_ui_and_value(
                    "insertHTML",
                    false,
                    "<h1>Heading 1</h1>",
                );
            }
        }
        VisualCommand::Heading2 => {
            if selection_state.inside_editor && !selection_state.collapsed {
                let _ =
                    html_document.exec_command_with_show_ui_and_value("formatBlock", false, "<h2>");
            } else {
                let _ = html_document.exec_command_with_show_ui_and_value(
                    "insertHTML",
                    false,
                    "<h2>Heading 2</h2>",
                );
            }
        }
        VisualCommand::BulletList => {
            if selection_state.inside_editor && !selection_state.collapsed {
                let _ = html_document.exec_command("insertUnorderedList");
            } else {
                let _ = html_document.exec_command_with_show_ui_and_value(
                    "insertHTML",
                    false,
                    "<ul><li>List item</li></ul>",
                );
            }
        }
        VisualCommand::Quote => {
            if selection_state.inside_editor && !selection_state.collapsed {
                let _ = html_document.exec_command_with_show_ui_and_value(
                    "formatBlock",
                    false,
                    "<blockquote>",
                );
            } else {
                let _ = html_document.exec_command_with_show_ui_and_value(
                    "insertHTML",
                    false,
                    "<blockquote>Quoted text</blockquote>",
                );
            }
        }
        VisualCommand::CodeBlock => {
            if selection_state.inside_editor && !selection_state.collapsed {
                let _ = html_document.exec_command_with_show_ui_and_value(
                    "formatBlock",
                    false,
                    "<pre>",
                );
            } else {
                let _ = html_document.exec_command_with_show_ui_and_value(
                    "insertHTML",
                    false,
                    "<pre><code>your code here</code></pre>",
                );
            }
        }
        VisualCommand::Link => {
            if selection_state.inside_editor && !selection_state.collapsed {
                let _ = html_document.exec_command_with_show_ui_and_value(
                    "createLink",
                    false,
                    "https://example.com",
                );
            } else {
                let _ = html_document.exec_command_with_show_ui_and_value(
                    "insertHTML",
                    false,
                    "<a href=\"https://example.com\">Link text</a>",
                );
            }
        }
    }
}

fn current_selection_state(editor: &HtmlElement) -> SelectionState {
    let Some(window) = web_sys::window() else {
        return SelectionState {
            inside_editor: false,
            collapsed: true,
        };
    };

    let Ok(Some(selection)) = window.get_selection() else {
        return SelectionState {
            inside_editor: false,
            collapsed: true,
        };
    };

    let editor_node: &Node = editor.unchecked_ref();
    let inside_editor = selection
        .anchor_node()
        .as_ref()
        .is_some_and(|node| editor_node.contains(Some(node)));

    SelectionState {
        inside_editor,
        collapsed: selection.is_collapsed(),
    }
}

fn visual_editor_to_markdown(root: &Element) -> String {
    let root_node: &Node = root.unchecked_ref();
    let markdown = serialize_children(root_node, false);
    normalize_markdown(&markdown)
}

fn serialize_children(node: &Node, in_pre: bool) -> String {
    let children = node.child_nodes();
    let mut output = String::new();

    for idx in 0..children.length() {
        if let Some(child) = children.item(idx) {
            output.push_str(&serialize_node(&child, in_pre));
        }
    }

    output
}

fn serialize_node(node: &Node, in_pre: bool) -> String {
    match node.node_type() {
        Node::TEXT_NODE => serialize_text_node(node, in_pre),
        Node::ELEMENT_NODE => {
            let Some(element) = node.dyn_ref::<Element>() else {
                return String::new();
            };
            serialize_element(element)
        }
        _ => String::new(),
    }
}

fn serialize_text_node(node: &Node, in_pre: bool) -> String {
    let text = node
        .text_content()
        .unwrap_or_default()
        .replace('\u{00a0}', " ");

    if in_pre {
        text
    } else {
        escape_inline_markdown(&text)
    }
}

fn serialize_element(element: &Element) -> String {
    let node: &Node = element.unchecked_ref();

    match element.tag_name().as_str() {
        "BR" => "\n".to_string(),
        "P" | "DIV" => serialize_paragraph(element),
        "H1" => serialize_heading(element, "# "),
        "H2" => serialize_heading(element, "## "),
        "H3" => serialize_heading(element, "### "),
        "STRONG" | "B" => wrap_inline("**", &serialize_children(node, false)),
        "EM" | "I" => wrap_inline("*", &serialize_children(node, false)),
        "A" => serialize_link(element),
        "UL" => serialize_list(element, false),
        "OL" => serialize_list(element, true),
        "LI" => trim_blank_lines(&serialize_children(node, false)),
        "BLOCKQUOTE" => serialize_blockquote(element),
        "PRE" => serialize_pre(element),
        "CODE" => serialize_inline_code(element),
        _ => serialize_children(node, false),
    }
}

fn serialize_paragraph(element: &Element) -> String {
    let node: &Node = element.unchecked_ref();
    let content = trim_blank_lines(&serialize_children(node, false));
    if content.is_empty() {
        String::new()
    } else {
        format!("{}\n\n", escape_block_starts(&content))
    }
}

fn serialize_heading(element: &Element, prefix: &str) -> String {
    let node: &Node = element.unchecked_ref();
    let content = trim_blank_lines(&serialize_children(node, false));
    if content.is_empty() {
        String::new()
    } else {
        format!("{prefix}{content}\n\n")
    }
}

fn serialize_link(element: &Element) -> String {
    let node: &Node = element.unchecked_ref();
    let text = trim_blank_lines(&serialize_children(node, false));
    let href = element
        .get_attribute("href")
        .unwrap_or_else(|| "https://example.com".to_string());
    let label = if text.is_empty() { href.clone() } else { text };
    format!("[{label}]({href})")
}

fn serialize_list(element: &Element, ordered: bool) -> String {
    let children = element.child_nodes();
    let mut items = Vec::new();
    let mut index = 1usize;

    for idx in 0..children.length() {
        let Some(child) = children.item(idx) else {
            continue;
        };
        let Some(item_element) = child.dyn_ref::<Element>() else {
            continue;
        };
        if item_element.tag_name() != "LI" {
            continue;
        }

        let content = trim_blank_lines(&serialize_children(&child, false));
        if content.is_empty() {
            continue;
        }

        let marker = if ordered {
            format!("{index}. ")
        } else {
            "- ".to_string()
        };
        let formatted = content
            .lines()
            .enumerate()
            .map(|(line_index, line)| {
                if line_index == 0 {
                    format!("{marker}{line}")
                } else {
                    format!("  {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        items.push(formatted);
        index += 1;
    }

    if items.is_empty() {
        String::new()
    } else {
        format!("{}\n\n", items.join("\n"))
    }
}

fn serialize_blockquote(element: &Element) -> String {
    let node: &Node = element.unchecked_ref();
    let content = trim_blank_lines(&serialize_children(node, false));
    if content.is_empty() {
        return String::new();
    }

    let prefixed = content
        .lines()
        .map(|line| {
            if line.is_empty() {
                ">".to_string()
            } else {
                format!("> {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("{prefixed}\n\n")
}

fn serialize_pre(element: &Element) -> String {
    let code = element
        .text_content()
        .unwrap_or_default()
        .replace('\u{00a0}', " ");
    let code = code.trim_matches('\n');

    if code.is_empty() {
        String::new()
    } else {
        format!("```\n{code}\n```\n\n")
    }
}

fn serialize_inline_code(element: &Element) -> String {
    let text = element
        .text_content()
        .unwrap_or_default()
        .replace('\u{00a0}', " ");
    format!("`{}`", text.replace('`', "\\`"))
}

fn wrap_inline(wrapper: &str, content: &str) -> String {
    let content = trim_blank_lines(content);
    if content.is_empty() {
        String::new()
    } else {
        format!("{wrapper}{content}{wrapper}")
    }
}

fn escape_inline_markdown(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('`', "\\`")
}

fn escape_block_starts(text: &str) -> String {
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            let indent_len = line.len() - trimmed.len();
            let indent = &line[..indent_len];

            if trimmed.starts_with('#')
                || trimmed.starts_with('>')
                || trimmed.starts_with("```")
                || trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed.starts_with("+ ")
                || starts_with_ordered_list_marker(trimmed)
            {
                format!("{indent}\\{trimmed}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn starts_with_ordered_list_marker(line: &str) -> bool {
    let digit_count = line.chars().take_while(|ch| ch.is_ascii_digit()).count();
    digit_count > 0
        && line.chars().nth(digit_count).is_some_and(|ch| ch == '.')
        && line
            .chars()
            .nth(digit_count + 1)
            .is_some_and(|ch| ch == ' ')
}

fn trim_blank_lines(text: &str) -> String {
    text.trim_matches('\n').trim().to_string()
}

fn normalize_markdown(markdown: &str) -> String {
    let mut normalized = markdown.replace("\r\n", "\n").replace('\u{00a0}', " ");

    while normalized.contains("\n\n\n") {
        normalized = normalized.replace("\n\n\n", "\n\n");
    }

    normalized.trim().to_string()
}
