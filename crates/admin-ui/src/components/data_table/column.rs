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
