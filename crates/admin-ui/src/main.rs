pub mod api;
pub mod components;
pub mod models;
pub mod routes;

use components::{DataPage, Footer, Sidebar, Titlebar};
use models::collection::Collection;
use yew::prelude::*;
use yew_router::prelude::*;

#[function_component(App)]
fn app() -> Html {
    let selected_collection = use_state(|| None::<Collection>);

    let on_select = {
        let selected_collection = selected_collection.clone();
        Callback::from(move |col: Collection| {
            selected_collection.set(Some(col));
        })
    };

    let selected_collection_id = (*selected_collection).as_ref().map(|col| col.id.clone());

    let active_title = (*selected_collection)
        .as_ref()
        .map(|col| format!("Collection: {}", col.name))
        .unwrap_or_else(|| "Dashboard".to_string());

    html! {
        <BrowserRouter>
        <div class="flex flex-col h-screen overflow-hidden bg-background text-on-surface">
            <Titlebar title={active_title} />
            <div class="flex-grow flex flex-row overflow-hidden">
                <Sidebar
                    selected_collection_id={selected_collection_id}
                    on_select={on_select}
                />
                <DataPage selected_collection={(*selected_collection).clone()} />
            </div>
            <Footer />
        </div>
        </BrowserRouter>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
