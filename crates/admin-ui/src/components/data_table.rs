use yew::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct DataRow {
    pub value: Vec<String>,
}

#[derive(Debug, Clone, Properties, PartialEq)]
pub struct DataTableProps {
    pub headers: Vec<String>,
    pub data: Vec<DataRow>,
    #[prop_or_default]
    pub on_row_click: Option<Callback<String>>,
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

#[function_component(DataTable)]
pub fn data_table(props: &DataTableProps) -> Html {
    html! {
        <div class="bg-surface-container-lowest border border-outline-variant rounded-2xl shadow-sm overflow-hidden">
            <div class="overflow-x-auto">
                <table class="w-full text-left border-collapse">
                    <thead>
                        <tr class="bg-surface-container-low border-b border-outline-variant">
                            {
                                props.headers.iter().map(|header| {
                                    html! {
                                        <th class="px-8 py-5 text-[11px] font-bold text-on-surface-variant uppercase tracking-[0.1em]">
                                            <div class="flex items-center gap-2">
                                                {header}
                                            </div>
                                        </th>
                                    }
                                }).collect::<Html>()
                            }
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-outline-variant/30">
                        {
                            props.data.iter().map(|row| {
                                let id = row.value.first().cloned().unwrap_or_default();

                                let on_click = {
                                    let on_row_click = props.on_row_click.clone();
                                    let id = id.clone();
                                    Callback::from(move |_| {
                                        if let Some(ref cb) = on_row_click {
                                            cb.emit(id.clone());
                                        }
                                    })
                                };

                                html! {
                                    <tr class="hover:bg-surface-container-high/20 transition-all group cursor-pointer" onclick={on_click}>
                                        {
                                            row.value.iter().map(|cell_val| {
                                                html! {
                                                    <td class="px-8 py-6 text-on-surface font-medium">{cell_val}</td>
                                                }
                                            }).collect::<Html>()
                                        }
                                    </tr>
                                }
                            }).collect::<Html>()
                        }
                    </tbody>
                </table>
            </div>
        </div>
    }
}
