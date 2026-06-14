pub mod components;
pub mod models;

use components::{Footer, MainPage, Sidebar, Titlebar};
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
                collection_type: "auth".to_string(),
                records: 12,
                modified: "2026-06-14T12:00:00Z".to_string(),
            },
            Collection {
                id: "posts_col".to_string(),
                name: "posts".to_string(),
                collection_type: "base".to_string(),
                records: 45,
                modified: "2026-06-14T12:30:00Z".to_string(),
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
        <div class="flex flex-col h-screen overflow-hidden bg-background text-on-surface">
            <Titlebar title={active_title} />
            <div class="flex-grow flex flex-row overflow-hidden">
                <Sidebar
                    collections={(*collections).clone()}
                    selected_collection_id={(*selected_collection_id).clone()}
                    on_select={on_select}
                />
                <MainPage selected_collection={active_collection} />
            </div>
            <Footer />
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
