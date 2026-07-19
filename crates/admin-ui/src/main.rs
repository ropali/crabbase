pub mod api;
pub mod components;
pub mod models;
pub mod routes;

use components::{CreateCollectionDrawer, DataPage, Footer, Login, Sidebar, Titlebar};
use models::collection::Collection;
use yew::prelude::*;
use yew_router::prelude::*;

#[function_component(App)]
fn app() -> Html {
    let is_logged_in = use_state(|| api::client::ApiClient::get_token().is_some());
    let selected_collection = use_state(|| None::<Collection>);
    let is_create_drawer_open = use_state(|| false);
    let collections_refresh_trigger = use_state(|| 0usize);

    let on_login_success = {
        let is_logged_in = is_logged_in.clone();
        Callback::from(move |_| {
            is_logged_in.set(true);
        })
    };

    let on_logout = {
        let is_logged_in = is_logged_in.clone();
        Callback::from(move |_| {
            api::client::ApiClient::set_token(None);
            is_logged_in.set(false);
        })
    };

    let on_select = {
        let selected_collection = selected_collection.clone();
        Callback::from(move |col: Collection| {
            selected_collection.set(Some(col));
        })
    };

    let on_create_click = {
        let is_create_drawer_open = is_create_drawer_open.clone();
        Callback::from(move |_| {
            is_create_drawer_open.set(true);
        })
    };

    let on_drawer_close = {
        let is_create_drawer_open = is_create_drawer_open.clone();
        Callback::from(move |_| {
            is_create_drawer_open.set(false);
        })
    };

    let on_drawer_success = {
        let is_create_drawer_open = is_create_drawer_open.clone();
        let collections_refresh_trigger = collections_refresh_trigger.clone();
        Callback::from(move |_| {
            collections_refresh_trigger.set(*collections_refresh_trigger + 1);
            is_create_drawer_open.set(false);
        })
    };

    let on_collection_updated = {
        let selected_collection = selected_collection.clone();
        let collections_refresh_trigger = collections_refresh_trigger.clone();
        Callback::from(move |col: Collection| {
            selected_collection.set(Some(col));
            collections_refresh_trigger.set(*collections_refresh_trigger + 1);
        })
    };

    let on_collection_deleted = {
        let selected_collection = selected_collection.clone();
        let collections_refresh_trigger = collections_refresh_trigger.clone();
        Callback::from(move |_| {
            selected_collection.set(None);
            collections_refresh_trigger.set(*collections_refresh_trigger + 1);
        })
    };

    let selected_collection_id = (*selected_collection).as_ref().map(|col| col.id.clone());

    let active_title = (*selected_collection)
        .as_ref()
        .map(|col| format!("Collection: {}", col.name))
        .unwrap_or_else(|| "Dashboard".to_string());

    html! {
        <BrowserRouter>
        {
            if !*is_logged_in {
                html! {
                    <Login on_login_success={on_login_success} />
                }
            } else {
                html! {
                    <div class="flex flex-col h-screen overflow-hidden bg-background text-on-surface">
                        <Titlebar title={active_title} on_logout={on_logout} />
                        <div class="flex-grow flex flex-row overflow-hidden relative">
                            <Sidebar
                                selected_collection_id={selected_collection_id}
                                on_select={on_select}
                                on_create_click={on_create_click}
                                refresh_trigger={*collections_refresh_trigger}
                            />
                            <DataPage
                                selected_collection={(*selected_collection).clone()}
                                on_collection_updated={on_collection_updated}
                                on_collection_deleted={on_collection_deleted}
                            />
                            {
                                if *is_create_drawer_open {
                                    html! {
                                        <CreateCollectionDrawer on_close={on_drawer_close} on_success={on_drawer_success} />
                                    }
                                } else {
                                    html! {}
                                }
                            }
                        </div>
                        <Footer />
                    </div>
                }
            }
        }
        </BrowserRouter>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
