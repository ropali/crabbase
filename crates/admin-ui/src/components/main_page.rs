use crate::models::collection::Collection;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct MainPageProps {
    pub selected_collection: Option<Collection>,
}

#[function_component(MainPage)]
pub fn main_page(props: &MainPageProps) -> Html {
    html! {
        <div class="flex-1 bg-slate-950 p-6 overflow-y-auto">
            {
                if let Some(col) = &props.selected_collection {
                    html! {
                        <div class="space-y-6">
                            <div class="bg-slate-900 border border-slate-800 rounded-lg p-6">
                                <h2 class="text-xl font-bold text-slate-100 mb-2">{ &col.name }</h2>
                                <p class="text-sm text-slate-400">{"Collection ID: "} { &col.id }</p>
                                <div class="mt-4">
                                    <h3 class="text-sm font-semibold text-slate-300 mb-2">{"Fields Configuration"}</h3>
                                    <pre class="bg-slate-950 p-4 rounded border border-slate-800 text-xs text-slate-300 overflow-x-auto">
                                        { serde_json::to_string_pretty(&col.fields).unwrap_or_default() }
                                    </pre>
                                </div>
                            </div>
                        </div>
                    }
                } else {
                    html! {
                        <div class="flex flex-col items-center justify-center h-full text-slate-500">
                            <p class="text-lg">{"Select a collection from the sidebar to view details"}</p>
                        </div>
                    }
                }
            }
        </div>
    }
}
