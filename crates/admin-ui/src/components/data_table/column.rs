use super::cell_value::CellValue;
use std::rc::Rc;
use yew::prelude::*;

#[derive(Clone)]
pub struct ColumnDef {
    pub key: String,
    pub header: String,
    pub icon: Option<&'static str>,
    pub sortable: bool,
    pub render: Rc<dyn Fn(&CellValue) -> Html>,
}

impl PartialEq for ColumnDef {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.header == other.header
            && self.icon == other.icon
            && self.sortable == other.sortable
    }
}

impl ColumnDef {
    pub fn new(key: &str, header: &str) -> Self {
        Self {
            key: key.to_string(),
            header: header.to_string(),
            icon: None,
            render: Rc::new(default_render),
            sortable: false,
        }
    }

    pub fn render(mut self, f: impl Fn(&CellValue) -> Html + 'static) -> Self {
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

pub fn default_render(v: &CellValue) -> Html {
    html! {
        <span class="font-body-sm text-body-sm text-on-surface">{ v.display() }</span>
    }
}

pub fn id_render(v: &CellValue) -> Html {
    html! {
        <span class="font-code-md text-code-md bg-surface-container-high/50 px-2 py-0.5 rounded border border-outline-variant/30 text-on-surface">
            { v.display() }
        </span>
    }
}

pub fn bool_render(v: &CellValue) -> Html {
    match v.as_bool() {
        Some(true) => html! {
            <span class="bg-[#e6f4ea] text-[#1e7e34] px-2 py-0.5 rounded font-label-xs text-label-xs">
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

pub fn date_render(v: &CellValue) -> Html {
    html! {
        <span class="font-label-xs text-label-xs text-on-surface-variant">{ v.display() }</span>
    }
}

pub fn code_render(v: &CellValue) -> Html {
    html! {
        <span class="font-code-md text-code-md text-on-surface-variant">{ v.display() }</span>
    }
}

pub fn nullable_text_render(v: &CellValue) -> Html {
    if v.is_null() {
        html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> }
    } else {
        html! { <span class="font-body-sm text-body-sm">{ v.display() }</span> }
    }
}
