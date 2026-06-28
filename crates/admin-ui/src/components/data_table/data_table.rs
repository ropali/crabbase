use std::collections::HashSet;

use web_sys::HtmlInputElement;
use web_sys::wasm_bindgen::JsCast;
use yew::prelude::*;

use super::cell_value::CellValue;
use super::column::ColumnDef;
use super::row::DynamicRow;

#[derive(Debug, Clone, PartialEq)]
pub struct DataRow {
    pub value: Vec<String>,
}

#[derive(Properties, PartialEq)]
pub struct PaginationProps {
    pub current_page: usize,
    pub total_items: usize,
    pub items_per_page: usize,
    pub on_page_change: Callback<usize>,
}

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    pub on_search: Callback<String>,
    pub on_create: Callback<()>,
}

#[derive(Properties, PartialEq, Clone)]
pub struct DataTableProps {
    pub columns: Vec<ColumnDef>,
    pub data: Vec<DynamicRow>,
    #[prop_or_default]
    pub selectable: bool,
    #[prop_or_default]
    pub on_row_click: Option<Callback<DynamicRow>>,
    #[prop_or_default]
    pub on_sort: Option<Callback<String>>,

    #[prop_or_default]
    pub on_select_all: Option<Callback<bool>>,

    #[prop_or_default]
    pub on_select_one: Option<Callback<(String, bool)>>,

    // Optional pagination props
    #[prop_or_default]
    pub current_page: Option<usize>,
    #[prop_or_default]
    pub total_items: Option<usize>,
    #[prop_or_default]
    pub items_per_page: Option<usize>,
    #[prop_or_default]
    pub on_page_change: Option<Callback<usize>>,

    #[prop_or_default]
    pub selected_ids: HashSet<String>,
}

#[function_component(DataTable)]
pub fn data_table(props: &DataTableProps) -> Html {
    let on_row_click = props.on_row_click.clone();

    let on_select_all = props.on_select_all.clone();

    let on_header_change = Callback::from(move |e: Event| {
        if let Some(cb) = &on_select_all {
            if let Some(target) = e.target() {
                if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                    cb.emit(input.checked());
                }
            }
        }
    });

    let pagination_footer = if let (
        Some(current_page),
        Some(total_items),
        Some(items_per_page),
        Some(on_page_change),
    ) = (
        props.current_page,
        props.total_items,
        props.items_per_page,
        &props.on_page_change,
    ) {
        let total_pages = (total_items + items_per_page - 1) / items_per_page;
        let start_item = if total_items == 0 {
            0
        } else {
            (current_page - 1) * items_per_page + 1
        };
        let end_item = std::cmp::min(current_page * items_per_page, total_items);

        let on_prev = {
            let on_page_change = on_page_change.clone();
            Callback::from(move |_| {
                if current_page > 1 {
                    on_page_change.emit(current_page - 1);
                }
            })
        };

        let on_next = {
            let on_page_change = on_page_change.clone();
            Callback::from(move |_| {
                if current_page < total_pages {
                    on_page_change.emit(current_page + 1);
                }
            })
        };

        let is_prev_disabled = current_page <= 1;
        let is_next_disabled = current_page >= total_pages || total_pages == 0;

        let prev_class = if is_prev_disabled {
            "text-on-surface-variant/40 cursor-not-allowed bg-surface-container-high/30 px-3 py-1.5 rounded-lg border border-outline-variant flex items-center gap-1 font-label-xs text-label-xs"
        } else {
            "text-on-surface hover:bg-surface-container-high active:scale-95 transition-all bg-surface-container-lowest px-3 py-1.5 rounded-lg border border-outline-variant flex items-center gap-1 font-label-xs text-label-xs"
        };

        let next_class = if is_next_disabled {
            "text-on-surface-variant/40 cursor-not-allowed bg-surface-container-high/30 px-3 py-1.5 rounded-lg border border-outline-variant flex items-center gap-1 font-label-xs text-label-xs"
        } else {
            "text-on-surface hover:bg-surface-container-high active:scale-95 transition-all bg-surface-container-lowest px-3 py-1.5 rounded-lg border border-outline-variant flex items-center gap-1 font-label-xs text-label-xs"
        };

        html! {
            <div class="px-6 py-4 flex items-center justify-between border-t border-outline-variant bg-surface-container-low/20">
                <div class="text-body-sm font-body-sm text-on-surface-variant">
                    { format!("Showing {}-{} of {} records", start_item, end_item, total_items) }
                </div>
                <div class="flex items-center gap-4">
                    <span class="text-body-sm font-body-sm text-on-surface-variant">
                        { format!("Page {} of {}", current_page, if total_pages == 0 { 1 } else { total_pages }) }
                    </span>
                    <div class="flex gap-2">
                        <button onclick={on_prev} disabled={is_prev_disabled} class={prev_class}>
                            <span class="material-symbols-outlined text-sm">{"chevron_left"}</span>
                            {"Prev"}
                        </button>
                        <button onclick={on_next} disabled={is_next_disabled} class={next_class}>
                            {"Next"}
                            <span class="material-symbols-outlined text-sm">{"chevron_right"}</span>
                        </button>
                    </div>
                </div>
            </div>
        }
    } else {
        html! {}
    };

    html! {
        <div class="flex-1 flex flex-col min-h-0 bg-surface-container-lowest border border-outline-variant rounded-xl overflow-hidden shadow-sm">
            <div class="flex-grow overflow-y-auto custom-scrollbar">
                <table class="w-full text-left border-collapse">
                    <thead class="table-sticky-header bg-surface-container-high/90 backdrop-blur-md sticky top-0 z-10">
                        <tr class="border-b border-outline-variant">
                            if props.selectable {
                                <th class="w-10 px-cell_padding_h py-cell_padding_v">
                                    <input
                                    type="checkbox"
                                    checked={!props.data.is_empty() && props.selected_ids.len() == props.data.len()}
                                    class="rounded border-outline-variant text-secondary focus:ring-secondary"
                                        onchange={on_header_change}
                                    />
                                </th>
                            }
                            { for props.columns.iter().map(|col| {
                                let on_sort = props.on_sort.clone();
                                let key = col.key.clone();
                                let onclick = if col.sortable {
                                    on_sort.map(|cb| Callback::from(move |_| cb.emit(key.clone())))
                                } else {
                                    None
                                };
                                html! {
                                    <th
                                        class="px-cell_padding_h py-cell_padding_v font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider cursor-pointer"
                                        onclick={onclick}
                                    >
                                        <div class="flex items-center gap-2">
                                            if let Some(icon) = col.icon {
                                                <span class="material-symbols-outlined text-sm">{ icon }</span>
                                            }
                                            { &col.header }
                                        </div>
                                    </th>
                                }
                            }) }
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-outline-variant">
                        { for props.data.iter().map(|row| {
                            let row_clone = row.clone();
                            let onclick = on_row_click.clone().map(|cb| {
                                Callback::from(move |_| cb.emit(row_clone.clone()))
                            });

                            let row_id = match row.get("id") {
                                CellValue::Text(id) => id.clone(),
                                _ => "".to_string(),
                            };
                            let is_checked = props.selected_ids.contains(&row_id);

                            let on_select_one = props.on_select_one.clone();
                            let row_id_clone = row_id.clone();
                            let on_change = Callback::from(move |e: Event| {
                                if let Some(cb) = &on_select_one {
                                    if let Some(target) = e.target() {
                                        if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                                            cb.emit((row_id_clone.clone(), input.checked()));
                                        }
                                    }
                                }
                            });

                            html! {
                                <tr class="hover:bg-surface-container-low transition-colors group relative" onclick={onclick}>
                                    if props.selectable {
                                        <td class="px-cell_padding_h py-cell_padding_v">
                                            <div class="absolute left-0 top-0 bottom-0 w-1 bg-secondary opacity-0 group-hover:opacity-100 transition-opacity"></div>
                                            <input
                                                type="checkbox"
                                                class="rounded border-outline-variant text-secondary focus:ring-secondary"
                                                checked={is_checked}
                                                onchange={on_change}
                                                onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}
                                            />
                                        </td>
                                    }
                                    { for props.columns.iter().map(|col| {
                                        let cell_val = row.get(&col.key);
                                        html! {
                                            <td class="px-cell_padding_h py-cell_padding_v">
                                                { (col.render)(cell_val) }
                                            </td>
                                        }
                                    }) }
                                </tr>
                            }
                        }) }
                    </tbody>
                </table>
            </div>
            { pagination_footer }
        </div>
    }
}
