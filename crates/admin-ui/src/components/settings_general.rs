use crate::api::client::ApiClient;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SettingsGeneralProps {
    #[prop_or_default]
    pub on_save: Option<Callback<()>>,
}

#[function_component(SettingsGeneral)]
pub fn settings_general(props: &SettingsGeneralProps) -> Html {
    let app_name = use_state(|| String::new());
    let app_url = use_state(|| String::new());
    let contact_email = use_state(|| String::new());
    let public_registration = use_state(|| false);

    let is_loading = use_state(|| true);
    let is_saving = use_state(|| false);
    let error_msg = use_state(|| Option::<String>::None);

    // Fetch settings on mount
    {
        let app_name = app_name.clone();
        let app_url = app_url.clone();
        let contact_email = contact_email.clone();
        let public_registration = public_registration.clone();
        let is_loading = is_loading.clone();
        let error_msg = error_msg.clone();

        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let client = ApiClient::default();
                match client.get_app_settings().await {
                    Ok(Some(data)) => {
                        if let Some(v) = data.get("appName").and_then(|v| v.as_str()) {
                            app_name.set(v.to_string());
                        }
                        if let Some(v) = data.get("appUrl").and_then(|v| v.as_str()) {
                            app_url.set(v.to_string());
                        }
                        if let Some(v) = data.get("contactEmail").and_then(|v| v.as_str()) {
                            contact_email.set(v.to_string());
                        }
                        if let Some(v) = data
                            .get("allowPublicUserRegistration")
                            .and_then(|v| v.as_bool())
                        {
                            public_registration.set(v);
                        }
                    }
                    Ok(None) => { /* no settings saved yet — keep defaults */ }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to load settings: {}", e)));
                    }
                }
                is_loading.set(false);
            });
            || ()
        });
    }

    let toggle_registration = {
        let public_registration = public_registration.clone();
        Callback::from(move |_| {
            public_registration.set(!*public_registration);
        })
    };

    let on_save_click = {
        let app_name = app_name.clone();
        let app_url = app_url.clone();
        let contact_email = contact_email.clone();
        let public_registration = public_registration.clone();
        let is_saving = is_saving.clone();
        let error_msg = error_msg.clone();
        let on_save = props.on_save.clone();

        Callback::from(move |_| {
            let app_name = (*app_name).clone();
            let app_url = (*app_url).clone();
            let contact_email = (*contact_email).clone();
            let public_registration = *public_registration;
            let is_saving = is_saving.clone();
            let error_msg = error_msg.clone();
            let on_save = on_save.clone();

            is_saving.set(true);
            error_msg.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                let body = serde_json::json!({
                    "appName": app_name,
                    "appUrl": app_url,
                    "contactEmail": contact_email,
                    "allowPublicUserRegistration": public_registration,
                });
                let client = ApiClient::default();
                match client.save_app_settings(body).await {
                    Ok(_) => {
                        if let Some(ref cb) = on_save {
                            cb.emit(());
                        }
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to save settings: {}", e)));
                    }
                }
                is_saving.set(false);
            });
        })
    };

    let on_cancel = {
        let app_name = app_name.clone();
        let app_url = app_url.clone();
        let contact_email = contact_email.clone();
        let public_registration = public_registration.clone();
        let error_msg = error_msg.clone();
        Callback::from(move |_| {
            app_name.set(String::new());
            app_url.set(String::new());
            contact_email.set(String::new());
            public_registration.set(false);
            error_msg.set(None);
        })
    };

    html! {
        <main class="flex-1 overflow-y-auto p-margin_page bg-surface-container-low">
            <div class="max-w-4xl mx-auto">
                <div class="flex flex-col gap-1 mb-8">
                    <div class="flex items-center gap-2 text-on-surface-variant font-label-xs text-label-xs mb-1 opacity-80">
                        <span>{"Settings"}</span>
                        <span class="material-symbols-outlined text-[12px]">{"chevron_right"}</span>
                        <span class="font-bold text-on-surface">{"General"}</span>
                    </div>
                    <h2 class="font-headline-lg text-headline-lg text-on-surface">{"General settings"}</h2>
                    <p class="font-body-md text-body-md text-on-surface-variant">
                        {"Manage your Crabbase engine app configuration, identity, and access controls."}
                    </p>
                </div>

                {
                    if let Some(ref msg) = *error_msg {
                        html! {
                            <div class="mb-6 p-4 bg-error-container/10 border border-error-container/30 text-error rounded-lg flex items-center justify-between">
                                <div class="flex items-center gap-2">
                                    <span class="material-symbols-outlined">{"error"}</span>
                                    <span class="font-body-sm text-body-sm font-bold">{msg.clone()}</span>
                                </div>
                                <button onclick={{
                                    let error_msg = error_msg.clone();
                                    move |_| error_msg.set(None)
                                }} class="text-error hover:opacity-80">
                                    <span class="material-symbols-outlined text-sm">{"close"}</span>
                                </button>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }

                <div class={classes!(
                    "bg-surface-container-lowest", "border", "border-outline-variant", "p-8",
                    "rounded-xl", "space-y-8", "shadow-sm",
                    if *is_loading { "opacity-60 pointer-events-none" } else { "" }
                )}>
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                        <div class="space-y-1.5">
                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider flex items-center gap-1">
                                {"Application Name "} <span class="text-primary">{"*"}</span>
                            </label>
                            <input
                                class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-body-sm text-body-sm rounded-lg"
                                type="text"
                                placeholder="e.g. My App"
                                value={(*app_name).clone()}
                                oninput={{
                                    let app_name = app_name.clone();
                                    Callback::from(move |e: InputEvent| {
                                        if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                            app_name.set(target.value());
                                        }
                                    })
                                }}
                            />
                        </div>

                        <div class="space-y-1.5">
                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider flex items-center gap-1">
                                {"Application URL "} <span class="text-primary">{"*"}</span>
                            </label>
                            <input
                                class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-body-sm text-body-sm rounded-lg"
                                type="text"
                                placeholder="https://myapp.com"
                                value={(*app_url).clone()}
                                oninput={{
                                    let app_url = app_url.clone();
                                    Callback::from(move |e: InputEvent| {
                                        if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                            app_url.set(target.value());
                                        }
                                    })
                                }}
                            />
                        </div>
                    </div>

                    <div class="space-y-1.5">
                        <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider flex items-center gap-1">
                            {"Contact Email"}
                        </label>
                        <input
                            class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-body-sm text-body-sm rounded-lg"
                            type="email"
                            placeholder="contact@myapp.com"
                            value={(*contact_email).clone()}
                            oninput={{
                                let contact_email = contact_email.clone();
                                Callback::from(move |e: InputEvent| {
                                    if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                        contact_email.set(target.value());
                                    }
                                })
                            }}
                        />
                    </div>

                    <div class="flex items-center justify-between py-4 border-y border-outline-variant/30">
                        <div>
                            <span class="font-body-md text-body-md font-bold block">{"Allow Public User Registration"}</span>
                            <span class="font-body-sm text-body-sm text-on-surface-variant opacity-70">{"When enabled, new users can sign up without admin invitations."}</span>
                        </div>
                        <button
                            type="button"
                            onclick={toggle_registration}
                            class={classes!(
                                "relative", "inline-flex", "h-6", "w-11", "shrink-0", "cursor-pointer",
                                "rounded-full", "border-2", "border-transparent", "transition-colors",
                                "duration-200", "ease-in-out", "focus:outline-none",
                                if *public_registration { "bg-primary-container" } else { "bg-outline-variant" }
                            )}
                        >
                            <span
                                class={classes!(
                                    "pointer-events-none", "inline-block", "h-5", "w-5", "transform",
                                    "rounded-full", "bg-white", "shadow", "ring-0", "transition",
                                    "duration-200", "ease-in-out",
                                    if *public_registration { "translate-x-5" } else { "translate-x-0" }
                                )}
                            />
                        </button>
                    </div>

                    <div class="flex items-center justify-end gap-4 pt-4 border-t border-outline-variant/30">
                        <button
                            type="button"
                            onclick={on_cancel}
                            class="px-6 py-2.5 font-label-xs text-label-xs font-bold text-on-surface-variant hover:bg-surface-container-high transition-colors rounded-lg cursor-pointer"
                        >
                            {"Cancel"}
                        </button>
                        <button
                            type="button"
                            onclick={on_save_click}
                            disabled={*is_saving}
                            class={classes!(
                                "px-8", "py-2.5", "font-label-xs", "text-label-xs", "font-bold",
                                "rounded-lg", "shadow-sm", "transition-all",
                                if *is_saving {
                                    "bg-primary/60 text-on-primary cursor-not-allowed"
                                } else {
                                    "bg-primary text-on-primary active:scale-[0.98] hover:bg-on-primary-fixed-variant cursor-pointer"
                                }
                            )}
                        >
                            { if *is_saving { "Saving..." } else { "Save changes" } }
                        </button>
                    </div>
                </div>
            </div>
        </main>
    }
}
