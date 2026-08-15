pub mod api;
pub mod components;
pub mod models;
pub mod routes;

use components::{
    ActivityLogs, CreateCollectionDrawer, DataPage, Footer, Login, NotificationKind,
    NotificationMessage, NotificationToast, SettingsBackups, SettingsCrons, SettingsGeneral,
    SettingsMail, SettingsStorage, Sidebar, Titlebar,
};
use gloo_events::EventListener;
use models::collection::Collection;
use routes::Route;
use yew::prelude::*;
use yew_router::prelude::*;

#[function_component(AppMain)]
fn app_main() -> Html {
    let navigator = use_navigator();
    let route = use_route::<Route>().unwrap_or(Route::Home);

    let selected_collection = use_state(|| None::<Collection>);
    let is_create_drawer_open = use_state(|| false);
    let collections_refresh_trigger = use_state(|| 0usize);
    let notification = use_state(|| None::<NotificationMessage>);

    let active_view = match &route {
        Route::SettingsMail => "settings_mail",
        Route::SettingsStorage => "settings_storage",
        Route::SettingsBackups => "settings_backups",
        Route::SettingsCrons => "settings_crons",
        Route::SettingsGeneral | Route::SettingsGeneralExplicit => "settings_general",
        Route::Logs => "logs",
        _ => "collections",
    };

    let on_logout = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            api::client::ApiClient::set_token(None);
            if let Some(ref nav) = navigator {
                nav.push(&Route::Login);
            }
        })
    };

    let on_select = {
        let selected_collection = selected_collection.clone();
        let navigator = navigator.clone();
        Callback::from(move |col: Collection| {
            selected_collection.set(Some(col.clone()));
            if let Some(ref nav) = navigator {
                nav.push(&Route::Collection {
                    name: col.name.clone(),
                });
            }
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

    let on_nav_change = {
        let navigator = navigator.clone();
        Callback::from(move |nav_target: String| {
            if let Some(ref nav) = navigator {
                match nav_target.as_str() {
                    "collections" => nav.push(&Route::Home),
                    "logs" => nav.push(&Route::Logs),
                    "settings_mail" => nav.push(&Route::SettingsMail),
                    "settings_storage" => nav.push(&Route::SettingsStorage),
                    "settings_backups" => nav.push(&Route::SettingsBackups),
                    "settings_crons" => nav.push(&Route::SettingsCrons),
                    "settings" | "settings_general" => nav.push(&Route::SettingsGeneral),
                    _ => nav.push(&Route::Home),
                }
            }
        })
    };

    let on_settings_select = {
        let navigator = navigator.clone();
        Callback::from(move |setting_target: String| {
            if let Some(ref nav) = navigator {
                match setting_target.as_str() {
                    "settings_mail" => nav.push(&Route::SettingsMail),
                    "settings_storage" => nav.push(&Route::SettingsStorage),
                    "settings_backups" => nav.push(&Route::SettingsBackups),
                    "settings_crons" => nav.push(&Route::SettingsCrons),
                    "settings_general" => nav.push(&Route::SettingsGeneral),
                    _ => nav.push(&Route::SettingsGeneral),
                }
            }
        })
    };

    let selected_collection_id = (*selected_collection).as_ref().map(|col| col.id.clone());

    let active_title = match &route {
        Route::SettingsMail => "Settings - Mail".to_string(),
        Route::SettingsStorage => "Settings - File storage".to_string(),
        Route::SettingsBackups => "Settings - Backups".to_string(),
        Route::SettingsCrons => "Settings - Crons".to_string(),
        Route::SettingsGeneral | Route::SettingsGeneralExplicit => "Settings - General".to_string(),
        Route::Logs => "Logs".to_string(),
        Route::Collection { name } => format!("Collection: {}", name),
        _ => (*selected_collection)
            .as_ref()
            .map(|col| format!("Collection: {}", col.name))
            .unwrap_or_else(|| "Dashboard".to_string()),
    };

    html! {
        <>
        <NotificationToast
            notification={(*notification).clone()}
            on_dismiss={{
                let notification = notification.clone();
                Callback::from(move |_| notification.set(None))
            }}
        />
        <div class="flex flex-col h-screen overflow-hidden bg-background text-on-surface">
            <Titlebar
                title={active_title}
                on_logout={on_logout}
                active_nav={active_view.to_string()}
                on_nav_change={Some(on_nav_change)}
            />
            <div class="flex-grow flex flex-row overflow-hidden relative">
                {
                    if route != Route::Logs {
                        html! {
                            <Sidebar
                                selected_collection_id={selected_collection_id}
                                on_select={on_select}
                                on_create_click={on_create_click}
                                refresh_trigger={*collections_refresh_trigger}
                                active_view={active_view.to_string()}
                                on_settings_select={Some(on_settings_select)}
                            />
                        }
                    } else {
                        html! {}
                    }
                }
                {
                    match &route {
                        Route::SettingsMail => {
                            let notification = notification.clone();
                            let on_mail_saved = Callback::from(move |_| {
                                let id = js_sys::Date::now() as u64;
                                notification.set(Some(NotificationMessage {
                                    id,
                                    kind: NotificationKind::Success,
                                    title: "Settings saved".to_string(),
                                    message: "Mail settings have been saved successfully.".to_string(),
                                }));
                            });
                            html! { <SettingsMail on_save={on_mail_saved} /> }
                        },
                        Route::SettingsStorage => html! { <SettingsStorage /> },
                        Route::SettingsGeneral | Route::SettingsGeneralExplicit => {
                            let notification = notification.clone();
                            let on_general_saved = Callback::from(move |_| {
                                let id = js_sys::Date::now() as u64;
                                notification.set(Some(NotificationMessage {
                                    id,
                                    kind: NotificationKind::Success,
                                    title: "Settings saved".to_string(),
                                    message: "General settings have been saved successfully.".to_string(),
                                }));
                            });
                            html! { <SettingsGeneral on_save={on_general_saved} /> }
                        },
                        Route::SettingsBackups => html! { <SettingsBackups /> },
                        Route::SettingsCrons => html! { <SettingsCrons /> },
                        Route::Logs => html! { <ActivityLogs /> },
                        _ => html! {
                            <DataPage
                                selected_collection={(*selected_collection).clone()}
                                on_collection_updated={on_collection_updated}
                                on_collection_deleted={on_collection_deleted}
                            />
                        },
                    }
                }
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
        </>
    }
}

#[function_component(App)]
fn app() -> Html {
    let is_logged_in = use_state(|| api::client::ApiClient::get_token().is_some());
    let notification = use_state(|| None::<NotificationMessage>);

    // Global listener for 401 Unauthorized events
    {
        let is_logged_in = is_logged_in.clone();
        let notification = notification.clone();
        use_effect_with((), move |_| {
            let listener = if let Some(window) = web_sys::window() {
                let is_logged_in = is_logged_in.clone();
                let notification = notification.clone();
                Some(EventListener::new(
                    &window,
                    "crabbase_401_unauthorized",
                    move |_| {
                        is_logged_in.set(false);
                        let id = js_sys::Date::now() as u64;
                        notification.set(Some(NotificationMessage {
                            id,
                            kind: NotificationKind::Error,
                            title: "401 Unauthorized".to_string(),
                            message:
                                "Session expired or authentication required. Please log in again."
                                    .to_string(),
                        }));
                    },
                ))
            } else {
                None
            };
            move || drop(listener)
        });
    }

    let on_dismiss_notification = {
        let notification = notification.clone();
        Callback::from(move |_| {
            notification.set(None);
        })
    };

    let on_login_success = {
        let is_logged_in = is_logged_in.clone();
        let notification = notification.clone();
        Callback::from(move |_| {
            is_logged_in.set(true);
            notification.set(None);
        })
    };

    html! {
        <BrowserRouter>
            <NotificationToast notification={(*notification).clone()} on_dismiss={on_dismiss_notification} />
            {
                if !*is_logged_in {
                    html! {
                        <Login on_login_success={on_login_success} />
                    }
                } else {
                    html! {
                        <AppMain />
                    }
                }
            }
        </BrowserRouter>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
