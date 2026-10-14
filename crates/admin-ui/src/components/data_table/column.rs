use super::cell_value::CellValue;
use std::rc::Rc;
use yew::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ViewCellPayload {
    pub field_name: String,
    pub data_type: String,
    pub value: String,
}

#[derive(Clone)]
pub struct ColumnDef {
    pub key: String,
    pub header: String,
    pub data_type: String,
    pub icon: Option<&'static str>,
    pub sortable: bool,
    pub render: Rc<dyn Fn(&CellValue, Option<&Callback<ViewCellPayload>>) -> Html>,
}

impl PartialEq for ColumnDef {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.header == other.header
            && self.data_type == other.data_type
            && self.icon == other.icon
            && self.sortable == other.sortable
    }
}

impl ColumnDef {
    pub fn new(key: &str, header: &str) -> Self {
        Self {
            key: key.to_string(),
            header: header.to_string(),
            data_type: "text".to_string(),
            icon: None,
            render: Rc::new(default_render),
            sortable: false,
        }
    }

    pub fn data_type(mut self, dt: &str) -> Self {
        self.data_type = dt.to_string();
        self
    }

    pub fn render(
        mut self,
        f: impl Fn(&CellValue, Option<&Callback<ViewCellPayload>>) -> Html + 'static,
    ) -> Self {
        self.render = Rc::new(f);
        self
    }

    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }
}

pub fn default_render(v: &CellValue, _on_view: Option<&Callback<ViewCellPayload>>) -> Html {
    html! {
        <span class="font-body-sm text-body-sm text-on-surface">{ v.display() }</span>
    }
}

pub fn id_render(v: &CellValue, _on_view: Option<&Callback<ViewCellPayload>>) -> Html {
    html! {
        <span class="font-code-md text-code-md bg-surface-container-high/50 px-2 py-0.5 rounded border border-outline-variant/30 text-on-surface">
            { v.display() }
        </span>
    }
}

pub fn bool_render(v: &CellValue, _on_view: Option<&Callback<ViewCellPayload>>) -> Html {
    match v.as_bool() {
        Some(true) => html! {
            <span class="bg-[#e6f4ea] text-[#1e7e34] px-2 py-0.5 rounded font-label-xs text-label-xs font-semibold">
                { "True" }
            </span>
        },
        _ => html! {
            <span class="bg-surface-container-highest text-on-surface-variant px-2 py-0.5 rounded font-label-xs text-label-xs">
                { v.display() }
            </span>
        },
    }
}

pub fn date_render(v: &CellValue, _on_view: Option<&Callback<ViewCellPayload>>) -> Html {
    html! {
        <span class="font-label-xs text-label-xs text-on-surface-variant">{ v.display() }</span>
    }
}

pub fn code_render(v: &CellValue, _on_view: Option<&Callback<ViewCellPayload>>) -> Html {
    html! {
        <span class="font-code-md text-code-md text-on-surface-variant">{ v.display() }</span>
    }
}

pub fn nullable_text_render(v: &CellValue, _on_view: Option<&Callback<ViewCellPayload>>) -> Html {
    if v.is_null() {
        html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> }
    } else {
        html! { <span class="font-body-sm text-body-sm">{ v.display() }</span> }
    }
}

pub fn clipped_text_render(
    _field_name: String,
    _data_type: String,
    v: &CellValue,
    _on_view: Option<&Callback<ViewCellPayload>>,
) -> Html {
    if v.is_null() {
        return html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> };
    }

    let text = v.display();
    let is_long = text.len() > 30 || text.contains('\n') || v.is_json();

    if is_long {
        let display_text = if text.len() > 30 {
            format!("{}...", &text[..30])
        } else {
            text.clone()
        };

        html! {
            <div
                title={text.clone()}
                class="group flex items-center gap-1.5 px-2 py-1 rounded-lg border border-transparent text-left transition-all max-w-[220px]"
            >
                <span class="font-body-sm text-body-sm text-on-surface truncate">
                    { display_text }
                </span>
                <span class="material-symbols-outlined text-[14px] text-on-surface-variant/60 group-hover:text-primary transition-colors flex-shrink-0">
                    {"unfold_more"}
                </span>
            </div>
        }
    } else {
        html! {
            <span class="font-body-sm text-body-sm text-on-surface">{ text }</span>
        }
    }
}

pub fn json_render(
    _field_name: String,
    v: &CellValue,
    _on_view: Option<&Callback<ViewCellPayload>>,
) -> Html {
    if v.is_null() {
        return html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> };
    }

    let raw_text = v.display();

    let is_empty = raw_text == "{}" || raw_text == "[]" || raw_text.is_empty();
    let badge_text = if is_empty {
        "{} empty".to_string()
    } else if raw_text.len() > 25 {
        format!("{}...", &raw_text[..25])
    } else {
        raw_text.clone()
    };

    html! {
        <div
            title="Click row to view & edit JSON in sidebar"
            class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-surface-container-high/60 border border-outline-variant/40 font-code-md text-code-md text-on-surface transition-all max-w-[220px] group"
        >
            <span class="material-symbols-outlined text-sm text-primary group-hover:scale-110 transition-transform">
                {"data_object"}
            </span>
            <span class="truncate font-mono text-xs">
                { badge_text }
            </span>
        </div>
    }
}

pub fn richtext_render(
    _field_name: String,
    v: &CellValue,
    _on_view: Option<&Callback<ViewCellPayload>>,
) -> Html {
    if v.is_null() {
        return html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> };
    }

    let text = v.display();
    let display_text = if text.len() > 25 {
        format!("{}...", &text[..25])
    } else {
        text.clone()
    };

    html! {
        <div
            title="Click row to view & edit content in sidebar"
            class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-secondary-container/30 border border-secondary/20 font-body-sm text-body-sm text-on-surface transition-all max-w-[220px] group"
        >
            <span class="material-symbols-outlined text-sm text-secondary group-hover:scale-110 transition-transform">
                {"article"}
            </span>
            <span class="truncate text-xs font-medium">
                { display_text }
            </span>
        </div>
    }
}

pub fn relation_render(
    v: &CellValue,
    is_multiple: bool,
    _on_view: Option<&Callback<ViewCellPayload>>,
) -> Html {
    if v.is_null() {
        return html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> };
    }

    if is_multiple {
        let items: Vec<String> = match v {
            CellValue::Json(serde_json::Value::Array(arr)) => arr
                .iter()
                .filter_map(|x| match x {
                    serde_json::Value::String(s) => Some(s.clone()),
                    other => Some(other.to_string()),
                })
                .collect(),
            CellValue::Text(s) => {
                if let Ok(arr) = serde_json::from_str::<Vec<String>>(s) {
                    arr
                } else if s.trim().is_empty() || s == "[]" {
                    vec![]
                } else {
                    s.split(',')
                        .map(|item| item.trim().to_string())
                        .filter(|item| !item.is_empty())
                        .collect()
                }
            }
            _ => vec![],
        };

        if items.is_empty() {
            html! {
                <span class="inline-flex items-center gap-1 text-on-surface-variant/50 text-xs italic">
                    <span class="material-symbols-outlined text-[14px]">{"link_off"}</span>
                    <span>{ "0 relations" }</span>
                </span>
            }
        } else if items.len() == 1 {
            html! {
                <div class="inline-flex items-center gap-1.5 bg-primary/10 text-primary border border-primary/20 px-2 py-0.5 rounded font-mono text-xs">
                    <span class="material-symbols-outlined text-[13px]">{"link"}</span>
                    <span class="truncate max-w-[130px]" title={items[0].clone()}>{ &items[0] }</span>
                </div>
            }
        } else {
            let tooltip = items.join(", ");
            html! {
                <div
                    title={tooltip}
                    class="inline-flex items-center gap-1.5 bg-primary/10 text-primary border border-primary/20 px-2.5 py-0.5 rounded-full font-mono text-xs font-semibold"
                >
                    <span class="material-symbols-outlined text-[13px]">{"library_add"}</span>
                    <span>{ format!("{} relations", items.len()) }</span>
                </div>
            }
        }
    } else {
        let text = v.display();
        if text.is_empty() || text == "null" {
            return html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> };
        }
        html! {
            <div class="inline-flex items-center gap-1 bg-surface-container-high/60 px-2 py-0.5 rounded border border-outline-variant/40 font-code-md text-code-md text-on-surface max-w-[180px]">
                <span class="material-symbols-outlined text-[13px] text-primary shrink-0">{"link"}</span>
                <span class="truncate font-mono text-xs" title={text.clone()}>{ text }</span>
            </div>
        }
    }
}

pub fn file_render(
    _field_name: String,
    v: &CellValue,
    _on_view: Option<&Callback<ViewCellPayload>>,
) -> Html {
    if v.is_null() {
        return html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> };
    }

    let text = v.display();
    if text.trim().is_empty() || text == "null" || text == "N/A" {
        return html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> };
    }

    let files: Vec<String> = if text.trim_start().starts_with('[') {
        serde_json::from_str::<Vec<String>>(&text).unwrap_or_else(|_| vec![text.clone()])
    } else {
        vec![text.clone()]
    };

    if files.is_empty() {
        return html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> };
    }

    let base_url = crate::api::client::ApiClient::get_base_url();
    let base_trimmed = base_url.trim_end_matches('/');

    let stop_prop = Callback::from(|e: MouseEvent| {
        e.stop_propagation();
    });

    let render_single_file = |storage_key: &str| {
        let filename = storage_key
            .rsplit('/')
            .next()
            .unwrap_or(storage_key)
            .to_string();

        let display_name = if filename.len() > 18 {
            format!("{}...", &filename[..15])
        } else {
            filename.clone()
        };

        let file_url = format!("{}/files/{}", base_trimmed, storage_key);

        let lower = storage_key.to_lowercase();
        let is_image = lower.ends_with(".png")
            || lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".gif")
            || lower.ends_with(".webp")
            || lower.ends_with(".svg")
            || lower.ends_with(".avif");

        let stop_prop_click = stop_prop.clone();
        let stop_prop_dl = stop_prop.clone();

        if is_image {
            html! {
                <div class="inline-flex items-center gap-1.5 max-w-[220px]" title={storage_key.to_string()}>
                    <a
                        href={file_url.clone()}
                        target="_blank"
                        rel="noopener noreferrer"
                        onclick={stop_prop_click}
                        class="shrink-0 relative group/thumb"
                        title={format!("View image: {}", filename)}
                    >
                        <img
                            src={file_url.clone()}
                            alt={filename.clone()}
                            class="w-7 h-7 object-cover rounded border border-outline-variant bg-surface hover:scale-125 transition-transform shadow-xs"
                        />
                    </a>
                    <span class="font-mono text-xs text-on-surface truncate min-w-0" title={filename.clone()}>
                        { display_name }
                    </span>
                    <a
                        href={file_url.clone()}
                        download={filename.clone()}
                        onclick={stop_prop_dl}
                        class="p-1 rounded text-on-surface-variant hover:text-primary hover:bg-surface-container-high transition-colors shrink-0 flex items-center justify-center"
                        title={format!("Download {}", filename)}
                    >
                        <span class="material-symbols-outlined text-[15px]">{"download"}</span>
                    </a>
                </div>
            }
        } else {
            html! {
                <div class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded bg-surface-container-high/60 border border-outline-variant/40 max-w-[220px]" title={storage_key.to_string()}>
                    <span class="material-symbols-outlined text-[16px] text-primary shrink-0">
                        {"description"}
                    </span>
                    <span class="font-mono text-xs text-on-surface truncate min-w-0" title={filename.clone()}>
                        { display_name }
                    </span>
                    <a
                        href={file_url.clone()}
                        download={filename.clone()}
                        onclick={stop_prop_dl}
                        class="p-0.5 rounded text-on-surface-variant hover:text-primary hover:bg-surface-container transition-colors shrink-0 flex items-center justify-center ml-0.5"
                        title={format!("Download {}", filename)}
                    >
                        <span class="material-symbols-outlined text-[15px]">{"download"}</span>
                    </a>
                </div>
            }
        }
    };

    if files.len() == 1 {
        render_single_file(&files[0])
    } else {
        html! {
            <div class="flex items-center gap-1.5 flex-wrap">
                { render_single_file(&files[0]) }
                <span class="text-xs text-on-surface-variant bg-surface-container px-1.5 py-0.5 rounded font-mono">
                    { format!("+{}", files.len() - 1) }
                </span>
            </div>
        }
    }
}
