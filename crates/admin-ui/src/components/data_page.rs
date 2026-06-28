use std::collections::HashSet;

use web_sys::console;
use yew::prelude::*;

use crate::api::client::ApiClient;
use crate::components::{
    CreateRecordDrawer, DataTable, PageHeader,
    data_table::{CellValue, DynamicRow, schema::columns_from_schema},
};
use crate::models::collection::{Collection as CollectionModel, Record};

#[derive(Properties, PartialEq)]
pub struct DataPageProps {
    pub selected_collection: Option<CollectionModel>,
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

    let current_page = use_state(|| 1usize);
    let total_items = use_state(|| 0usize);
    let items_per_page = use_state(|| 30usize);

    let selected_rows_ids = use_state(std::collections::HashSet::<String>::new);
    let refresh_trigger = use_state(|| 0usize);

    // Reset current page and selected records when the collection changes
    {
        let current_page = current_page.clone();
        let selected_rows_ids = selected_rows_ids.clone();
        let col_name = props
            .selected_collection
            .as_ref()
            .map(|col| col.name.clone());
        use_effect_with(col_name, move |_| {
            current_page.set(1);
            selected_rows_ids.set(std::collections::HashSet::new());
            || ()
        });
    }

    // Fetch records when collection, page or refresh trigger changes
    {
        let records = records.clone();
        let err = err_state.clone();
        let total_items = total_items.clone();
        let items_per_page = items_per_page.clone();
        let selected_col = props.selected_collection.clone();
        let col_name = selected_col.as_ref().map(|col| col.name.clone());
        let page_val = *current_page;
        let refresh_trigger_val = *refresh_trigger;

        use_effect_with(
            (col_name, page_val, refresh_trigger_val),
            move |(current_col_name, page, _)| {
                let records = records.clone();
                let total_items = total_items.clone();
                let items_per_page = items_per_page.clone();
                let err = err.clone();
                let selected_col = selected_col.clone();
                let page = *page;

                if let Some(col_name) = current_col_name {
                    let col_name = col_name.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        let client = ApiClient::new("/api".to_string(), None);
                        match client.get_records(&col_name, Some(page), Some(30)).await {
                            Ok(res) => {
                                if let Some(col) = selected_col {
                                    let mapped: Vec<DynamicRow> = res
                                        .items
                                        .into_iter()
                                        .map(|r| record_to_dynamic_row(r, &col))
                                        .collect();
                                    records.set(mapped);
                                    total_items.set(res.total);
                                    items_per_page.set(res.per_page);
                                }
                            }
                            Err(e) => {
                                err.set(Some(e.to_string()));
                            }
                        };
                    });
                } else {
                    records.set(Vec::new());
                    total_items.set(0);
                }
                || ()
            },
        );
    }

    let drawer_open_clone = drawer_open.clone();
    let on_row_click = Callback::from(move |_row: DynamicRow| {
        drawer_open_clone.set(true);
    });

    let on_search = Callback::from(|_query: String| {});

    let on_create = {
        let drawer_open = drawer_open.clone();
        Callback::from(move |_| {
            drawer_open.set(true);
        })
    };

    let on_drawer_close = {
        let drawer_open = drawer_open.clone();
        Callback::from(move |_| {
            drawer_open.set(false);
        })
    };

    let on_drawer_success = {
        let drawer_open = drawer_open.clone();
        let refresh_trigger = refresh_trigger.clone();
        Callback::from(move |_| {
            refresh_trigger.set(*refresh_trigger + 1);
            drawer_open.set(false);
        })
    };

    let on_page_change = {
        let current_page = current_page.clone();
        Callback::from(move |page: usize| {
            current_page.set(page);
        })
    };

    let on_select_all = {
        let selected_rows_ids = selected_rows_ids.clone();
        let records = records.clone();
        Callback::from(move |checked: bool| {
            if checked {
                let all_ids: HashSet<String> = records
                    .iter()
                    .filter_map(|row| {
                        if let CellValue::Text(id) = row.get("id") {
                            Some(id.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                selected_rows_ids.set(all_ids);
            } else {
                selected_rows_ids.set(HashSet::new());
            }
        })
    };

    let on_select_one = {
        let selected_rows_ids = selected_rows_ids.clone();
        Callback::from(move |(id, checked): (String, bool)| {
            let mut current = (*selected_rows_ids).clone();
            if checked {
                current.insert(id);
            } else {
                current.remove(&id);
            }
            selected_rows_ids.set(current);
        })
    };

    let on_reset_selection = {
        let selected_rows_ids = selected_rows_ids.clone();
        Callback::from(move |_| {
            selected_rows_ids.set(HashSet::new());
        })
    };

    let on_delete_selected = {
        let selected_rows_ids = selected_rows_ids.clone();
        let col_name = props
            .selected_collection
            .as_ref()
            .map(|col| col.name.clone());
        let refresh_trigger = refresh_trigger.clone();
        let err_state = err_state.clone();

        Callback::from(move |_| {
            if let Some(col_name) = &col_name {
                let selected_ids = (*selected_rows_ids).clone();
                let col_name = col_name.clone();
                let selected_rows_ids = selected_rows_ids.clone();
                let refresh_trigger = refresh_trigger.clone();
                let err_state = err_state.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    // FIXME: Perform bacth delete operations
                    let client = ApiClient::new("/api".to_string(), None);
                    let mut success = true;
                    for id in &selected_ids {
                        match client.delete_record(&col_name, id).await {
                            Ok(_) => {}
                            Err(e) => {
                                success = false;
                                err_state
                                    .set(Some(format!("Failed to delete record {}: {}", id, e)));
                                break;
                            }
                        }
                    }
                    if success {
                        selected_rows_ids.set(HashSet::new());
                        refresh_trigger.set(*refresh_trigger + 1);
                    }
                });
            }
        })
    };

    let on_download_json = {
        let selected_rows_ids = selected_rows_ids.clone();
        let records = records.clone();
        let col_name = props
            .selected_collection
            .as_ref()
            .map(|col| col.name.clone())
            .unwrap_or_else(|| "records".to_string());
        Callback::from(move |_| {
            let selected_ids = (*selected_rows_ids).clone();
            let selected_records: Vec<serde_json::Value> = records
                .iter()
                .filter(|row| {
                    if let CellValue::Text(id) = row.get("id") {
                        selected_ids.contains(id)
                    } else {
                        false
                    }
                })
                .map(|row| {
                    let mut map = serde_json::Map::new();
                    for (k, v) in &row.values {
                        let val = match v {
                            CellValue::Text(s) => serde_json::Value::String(s.clone()),
                            CellValue::Number(n) => {
                                if let Some(num) = serde_json::Number::from_f64(*n) {
                                    serde_json::Value::Number(num)
                                } else {
                                    serde_json::Value::Null
                                }
                            }
                            CellValue::Bool(b) => serde_json::Value::Bool(*b),
                            CellValue::Null => serde_json::Value::Null,
                        };
                        map.insert(k.clone(), val);
                    }
                    serde_json::Value::Object(map)
                })
                .collect();

            if let Ok(json_str) = serde_json::to_string_pretty(&selected_records) {
                let filename = format!("{}_export.json", col_name);
                if let Err(e) = download_json(&json_str, &filename) {
                    console::error_1(&format!("Failed to download JSON: {:?}", e).into());
                }
            }
        })
    };

    let columns = if let Some(col) = &props.selected_collection {
        columns_from_schema(col)
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
                            <div class="flex-1 flex flex-col min-h-0 px-6 pb-6">
                                <DataTable
                                    columns={columns}
                                    data={(*records).clone()}
                                    selectable={true}
                                    on_row_click={on_row_click}
                                    current_page={*current_page}
                                    total_items={*total_items}
                                    items_per_page={*items_per_page}
                                    on_page_change={on_page_change}
                                    on_select_all={on_select_all}
                                    on_select_one={on_select_one}
                                    selected_ids={(*selected_rows_ids).clone()}
                                />
                            </div>
                            {
                                if !selected_rows_ids.is_empty() {
                                    html! {
                                        <div class="absolute bottom-8 left-1/2 -translate-x-1/2 bg-surface-container-lowest border border-outline-variant rounded-full shadow-2xl px-6 py-2 flex items-center gap-4 z-50 transition-all duration-300">
                                            <div class="flex items-center gap-2">
                                                <span class="text-body-sm font-body-sm text-on-surface-variant">
                                                    {"Selected "}
                                                    <span class="font-bold">{ selected_rows_ids.len() }</span>
                                                    {" records"}
                                                </span>
                                                <button onclick={on_reset_selection} class="ml-2 px-3 py-1 rounded-lg bg-surface-container-high hover:bg-outline-variant text-on-surface-variant font-label-xs text-label-xs transition-colors">
                                                    {"Reset"}
                                                </button>
                                            </div>
                                            <div class="w-px h-6 bg-outline-variant/30"></div>
                                            <div class="flex items-center gap-2">
                                                <button onclick={on_delete_selected} class="flex items-center gap-2 px-4 py-1.5 rounded-lg border border-error text-error hover:bg-error/5 font-label-xs text-label-xs transition-colors">
                                                    <span class="material-symbols-outlined text-sm">{"delete"}</span>
                                                    {"Delete"}
                                                </button>
                                                <button onclick={on_download_json} class="flex items-center gap-2 px-4 py-1.5 rounded-lg bg-on-surface text-surface-container-lowest hover:bg-on-surface-variant font-label-xs text-label-xs transition-colors">
                                                    <span class="material-symbols-outlined text-sm">{"download"}</span>
                                                    {"JSON"}
                                                </button>
                                            </div>
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }
                            }
                            {
                                if *drawer_open {
                                    html! {
                                        <CreateRecordDrawer
                                            collection_name={col.name.clone()}
                                            collection_fields={col.fields.clone()}
                                            on_close={on_drawer_close}
                                            on_success={on_drawer_success}
                                        />
                                    }
                                } else {
                                    html! {}
                                }
                            }
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

fn download_json(json_str: &str, filename: &str) -> Result<(), web_sys::wasm_bindgen::JsValue> {
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::wasm_bindgen::JsValue;
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    // Create Blob
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(json_str));

    let blob_options = BlobPropertyBag::new();
    blob_options.set_type("application/json");

    let blob = Blob::new_with_str_sequence_and_options(&parts, &blob_options)?;
    let url = Url::create_object_url_with_blob(&blob)?;

    // Create Anchor element
    let anchor = document
        .create_element("a")?
        .dyn_into::<HtmlAnchorElement>()?;
    anchor.set_href(&url);
    anchor.set_download(filename);

    // Append to body
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("no body"))?;
    body.append_child(&anchor)?;

    // Click anchor
    anchor.click();

    // Cleanup
    body.remove_child(&anchor)?;
    Url::revoke_object_url(&url)?;

    Ok(())
}
