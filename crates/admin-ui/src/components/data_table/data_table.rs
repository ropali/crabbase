use yew::prelude::*;

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
}

#[function_component(DataTable)]
pub fn data_table(props: &DataTableProps) -> Html {
    let on_row_click = props.on_row_click.clone();

    html! {
        <div class="bg-surface-container-lowest border border-outline-variant rounded-xl overflow-hidden shadow-sm">
            <table class="w-full text-left border-collapse">
                <thead class="table-sticky-header bg-surface-container-high/30 backdrop-blur-md">
                    <tr class="border-b border-outline-variant">
                        if props.selectable {
                            <th class="w-10 px-cell_padding_h py-cell_padding_v">
                                <input type="checkbox" class="rounded border-outline-variant text-secondary focus:ring-secondary" />
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

                        html! {
                            <tr class="hover:bg-surface-container-low transition-colors group relative" onclick={onclick}>
                                if props.selectable {
                                    <td class="px-cell_padding_h py-cell_padding_v">
                                        <div class="absolute left-0 top-0 bottom-0 w-1 bg-secondary opacity-0 group-hover:opacity-100 transition-opacity"></div>
                                        <input
                                            type="checkbox"
                                            class="rounded border-outline-variant text-secondary focus:ring-secondary"
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
    }
}
