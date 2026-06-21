use yew::prelude::*;

use crate::api::client::ApiClient;
use crate::components::{
    DataTable, PageHeader,
    data_table::{CellValue, ColumnDef, DynamicRow},
};
use crate::models::collection::{Collection as CollectionModel, Record};

#[derive(Properties, PartialEq)]
pub struct DataPageProps {
    pub selected_collection: Option<CollectionModel>,
}

fn badge(text: &str, true_style: bool) -> Html {
    if true_style {
        html! {
            <span class="bg-[#e6f4ea] text-[#1e7e34] px-2 py-0.5 rounded font-label-xs text-label-xs">
                { text }
            </span>
        }
    } else {
        html! {
            <span class="bg-surface-container-highest text-on-surface-variant px-2 py-0.5 rounded font-label-xs text-label-xs">
                { text }
            </span>
        }
    }
}

fn record_to_dynamic_row(r: Record, col: &CollectionModel) -> DynamicRow {
    let mut values = std::collections::BTreeMap::new();

    // Add meta fields
    values.insert("id".to_string(), CellValue::Text(r.id));
    values.insert("created".to_string(), CellValue::Text(r.created));
    values.insert("updated".to_string(), CellValue::Text(r.updated));

    // Add dynamic fields
    for field in &col.fields {
        let cell_value = if let Some(val) = r.data.get(&field.name) {
            match val {
                serde_json::Value::Null => CellValue::Null,
                serde_json::Value::Bool(b) => CellValue::Bool(*b),
                serde_json::Value::Number(n) => CellValue::Number(n.as_f64().unwrap_or(0.0)),
                serde_json::Value::String(s) => CellValue::Text(s.clone()),
                _ => CellValue::Text(val.to_string()),
            }
        } else {
            CellValue::Null
        };
        values.insert(field.name.clone(), cell_value);
    }

    DynamicRow { values }
}

#[function_component(DataPage)]
pub fn data_page(props: &DataPageProps) -> Html {
    let records = use_state(Vec::<DynamicRow>::new);
    let drawer_open = use_state(|| false);
    let err_state = use_state(|| None::<String>);

    {
        let records = records.clone();
        let err = err_state.clone();
        let selected_col = props.selected_collection.clone();
        let col_name = selected_col.as_ref().map(|col| col.name.clone());

        use_effect_with(col_name.clone(), move |current_col_name| {
            let records = records.clone();
            if let Some(col_name) = current_col_name {
                let col_name = col_name.clone();
                let selected_col = selected_col.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let client = ApiClient::new("/api".to_string(), None);
                    match client.get_records(&col_name).await {
                        Ok(res) => {
                            if let Some(col) = selected_col {
                                let mapped: Vec<DynamicRow> = res
                                    .items
                                    .into_iter()
                                    .map(|r| record_to_dynamic_row(r, &col))
                                    .collect();
                                records.set(mapped);
                            }
                        }
                        Err(e) => {
                            err.set(Some(e.to_string()));
                        }
                    };
                });
            } else {
                records.set(Vec::new());
            }
            || ()
        });
    }

    let drawer_open_clone = drawer_open.clone();
    let on_row_click = Callback::from(move |_row: DynamicRow| {
        drawer_open_clone.set(true);
    });

    let on_search = Callback::from(|_query: String| {});
    let on_create = Callback::from(|_| {});

    let columns = if let Some(col) = &props.selected_collection {
        let mut cols = Vec::new();

        // Add "id" column first
        cols.push(
            ColumnDef::new("id", "id", |r: &DynamicRow| {
                let val = r.values.get("id").map(|v| v.as_str()).unwrap_or_default();
                html! {
                    <span class="font-code-md text-code-md bg-surface-container-high/50 px-2 py-0.5 rounded border border-outline-variant/30 text-on-surface">
                        { val }
                    </span>
                }
            }).icon("key").sortable(true)
        );

        // Add dynamically each field of the collection
        for field in &col.fields {
            let key = field.name.clone();
            let key_clone = key.clone();
            let header = field.name.clone();

            // Choose icon depending on data type
            let icon = match field.data_type.as_str() {
                "Email" => Some("mail"),
                "Bool" => Some("check_box"),
                "Number" => Some("tag"),
                "Relation" => Some("link"),
                "Datetime" | "AutoDatetime" => Some("schedule"),
                _ => Some("notes"),
            };

            let mut col_def = ColumnDef::new(&key, &header, move |r: &DynamicRow| {
                if let Some(cell_val) = r.values.get(&key_clone) {
                    match cell_val {
                        CellValue::Bool(b) => badge(if *b { "True" } else { "False" }, *b),
                        CellValue::Null => {
                            html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> }
                        }
                        _ => {
                            html! { <span class="font-body-sm text-body-sm">{ cell_val.as_str() }</span> }
                        }
                    }
                } else {
                    html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> }
                }
            });

            if let Some(ic) = icon {
                col_def = col_def.icon(ic);
            }
            cols.push(col_def);
        }

        // Add "created" column
        cols.push(
            ColumnDef::new("created", "created", |r: &DynamicRow| {
                let val = r.values.get("created").map(|v| v.as_str()).unwrap_or_default();
                html! { <span class="font-label-xs text-label-xs text-on-surface-variant">{ val }</span> }
            }).icon("schedule").sortable(true)
        );

        // Add "updated" column
        cols.push(
            ColumnDef::new("updated", "updated", |r: &DynamicRow| {
                let val = r.values.get("updated").map(|v| v.as_str()).unwrap_or_default();
                html! { <span class="font-label-xs text-label-xs text-on-surface-variant">{ val }</span> }
            }).icon("update").sortable(true)
        );

        cols
    } else {
        Vec::new()
    };

    html! {
        <main class="flex-1 flex flex-col overflow-hidden relative">
            {
                if let Some(col) = &props.selected_collection {
                    html! {
                        <>
                            <PageHeader
                                collection_name={col.name.clone()}
                                on_search={on_search}
                                on_create={on_create}
                            />
                            <div class="flex-1 overflow-auto px-6 pb-6 custom-scrollbar">
                                <DataTable<DynamicRow>
                                    columns={columns}
                                    data={(*records).clone()}
                                    selectable={true}
                                    on_row_click={on_row_click}
                                />
                            </div>
                        </>
                    }
                } else {
                    html! {
                        <div class="flex flex-col items-center justify-center h-full text-on-surface-variant opacity-60">
                            <p class="text-lg">{"Select a collection from the sidebar to view details"}</p>
                        </div>
                    }
                }
            }
        </main>
    }
}
