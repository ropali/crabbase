use crate::{
    components::{
        DataTable,
        data_table::{DataRow, DataTableProps},
    },
    models::collection::Collection,
};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct MainPageProps {
    pub selected_collection: Option<Collection>,
}

#[function_component(MainPage)]
pub fn main_page(props: &MainPageProps) -> Html {
    let data_table_props = DataTableProps {
        headers: vec![
            "ID".to_string(),
            "Email".to_string(),
            "Crated At".to_string(),
            "Updated At".to_string(),
        ],
        data: vec![
            DataRow {
                value: vec![
                    "1".to_string(),
                    "abc@example.com".to_string(),
                    "12/12/12".to_string(),
                    "12/12/12".to_string(),
                ],
            },
            DataRow {
                value: vec![
                    "2".to_string(),
                    "abc@example.com".to_string(),
                    "12/12/12".to_string(),
                    "12/12/12".to_string(),
                ],
            },
            DataRow {
                value: vec![
                    "3".to_string(),
                    "abc@example.com".to_string(),
                    "12/12/12".to_string(),
                    "12/12/12".to_string(),
                ],
            },
        ],
        on_row_click: None,
    };
    html! {
        <div class="flex-1 bg-[#FFFAF8] p-6 overflow-y-auto">
            {
                if let Some(_col) = &props.selected_collection {
                    html! {
                        <div class="space-y-6">
                            <div class="bg-surface-container-lowest border border-outline-variant rounded-lg p-6 shadow-sm">
                                <DataTable
                                    headers={data_table_props.headers.clone()}
                                    data={data_table_props.data.clone()}

                                />
                            </div>
                        </div>
                    }
                } else {
                    html! {
                        <div class="flex flex-col items-center justify-center h-full text-on-surface-variant opacity-60">
                            <p class="text-lg">{"Select a collection from the sidebar to view details"}</p>
                        </div>
                    }
                }
            }
        </div>
    }
}
