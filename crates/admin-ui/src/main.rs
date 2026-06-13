pub mod components;
pub mod models;

use components::{MainPage, Sidebar, Titlebar};
use models::collection::Collection;
use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    // Basic mock collections state
    let collections = use_state(|| {
        vec![
            Collection {
                id: "users_col".to_string(),
                name: "users".to_string(),
                system: true,
                fields: serde_json::json!({
                    "id": "text",
                    "email": "email",
                    "name": "text"
                }),
            },
            Collection {
                id: "posts_col".to_string(),
                name: "posts".to_string(),
                system: false,
                fields: serde_json::json!({
                    "id": "text",
                    "title": "text",
                    "content": "text",
                    "author_id": "relation"
                }),
            },
        ]
    });

    let selected_collection_id = use_state(|| None::<String>);

    let on_select = {
        let selected_collection_id = selected_collection_id.clone();
        Callback::from(move |id: String| {
            selected_collection_id.set(Some(id));
        })
    };

    let active_collection = (*selected_collection_id)
        .as_ref()
        .and_then(|id| collections.iter().find(|col| &col.id == id).cloned());

    let active_title = active_collection
        .as_ref()
        .map(|col| format!("Collection: {}", col.name))
        .unwrap_or_else(|| "Dashboard".to_string());

    html! {
        <div class="flex h-screen bg-slate-950 font-sans select-none overflow-hidden text-slate-100">
            <Sidebar
                collections={(*collections).clone()}
                selected_collection_id={(*selected_collection_id).clone()}
                on_select={on_select}
            />
            <div class="flex-1 flex flex-col overflow-hidden">
                <Titlebar title={active_title} />
                <MainPage selected_collection={active_collection} />
            </div>
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
