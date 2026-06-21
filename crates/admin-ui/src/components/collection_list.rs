use yew::prelude::*;
use yew_router::prelude::*;

use crate::api::client::ApiClient;
use crate::models::collection::Collection;
use crate::routes::Route;

#[derive(Properties, PartialEq)]
pub struct CollectionListProps {
    pub selected_collection_id: Option<String>,
    pub on_select: Callback<Collection>,
    #[prop_or_default]
    pub is_system: bool,
}

#[function_component(CollectionList)]
pub fn collection_list(props: &CollectionListProps) -> Html {
    let collections = use_state(Vec::<Collection>::new);
    let err = use_state(|| None::<String>);

    let navigator = use_navigator().expect("Router context not found");

    {
        let collections = collections.clone();
        let err = err.clone();

        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                // Trunk handles proxying "/api" requests to "http://localhost:8989/api"
                let client = ApiClient::new("/api".to_string(), None);

                match client.get_collections().await {
                    Ok(res) => collections.set(res.items),
                    Err(e) => {
                        err.set(Some(e.to_string()));
                    }
                }
            });
            || ()
        });
    }

    if let Some(err_msg) = &*err {
        html! {
            <div class="text-red-500 font-label-xs text-label-xs px-3 py-2 border border-red-200 bg-red-50 rounded-lg">
                { format!("Error: {}", err_msg) }
            </div>
        }
    } else {
        let display_collections: Vec<Collection> = collections
            .iter()
            .filter(|col| {
                let is_sys = col.system;
                is_sys == props.is_system
            })
            .cloned()
            .collect();

        html! {
            <>
                {
                    display_collections.iter().map(|col| {
                        let id = col.id.clone();
                        let on_click = {
                            let nav = navigator.clone();
                            let on_select = props.on_select.clone();
                            let col = col.clone();
                            Callback::from(move |_| {
                                on_select.emit(col.clone());
                                nav.push(&Route::Collection { name: col.name.clone() });
                                })
                        };
                        let is_selected = props.selected_collection_id.as_ref() == Some(&id);
                        let class = if is_selected {
                            "bg-secondary-container text-on-secondary-container font-bold rounded-lg px-3 py-2 flex items-center gap-3 translate-x-1 transition-transform sidebar-nav-link cursor-pointer"
                        } else {
                            "text-on-surface-variant hover:bg-surface-container-high rounded-lg px-3 py-2 flex items-center gap-3 transition-all sidebar-nav-link cursor-pointer"
                        };

                        let icon = if col.name == "users" || col.name == "_superusers" {
                            "person"
                        } else if col.name == "posts" {
                            "article"
                        } else if col.name == "messages" || col.name == "messagesReport" {
                            "chat"
                        } else {
                            "database"
                        };

                        let font_variation = if is_selected {
                            "font-variation-settings: 'FILL' 1;"
                        } else {
                            "font-variation-settings: 'FILL' 0;"
                        };

                        html! {
                          <a onclick={on_click} class={class} data-collection={col.name.clone()}>
                            <span class="material-symbols-outlined" style={font_variation}>{icon}</span>
                            <span class="font-label-xs text-label-xs">{&col.name}</span>
                          </a>
                        }
                    }).collect::<Html>()
                }
            </>
        }
    }
}
