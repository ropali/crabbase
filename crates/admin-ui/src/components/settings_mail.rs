use crate::api::client::ApiClient;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SettingsMailProps {
    #[prop_or_default]
    pub on_save: Option<Callback<()>>,
}

#[function_component(SettingsMail)]
pub fn settings_mail(props: &SettingsMailProps) -> Html {
    let sender_name = use_state(|| String::new());
    let sender_address = use_state(|| String::new());
    let smtp_host = use_state(|| String::new());
    let smtp_port = use_state(|| "587".to_string());
    let smtp_username = use_state(|| String::new());
    let smtp_password = use_state(|| String::new());
    let show_password = use_state(|| false);

    let is_loading = use_state(|| true);
    let is_saving = use_state(|| false);
    let error_msg = use_state(|| Option::<String>::None);

    // Fetch settings on mount
    {
        let sender_name = sender_name.clone();
        let sender_address = sender_address.clone();
        let smtp_host = smtp_host.clone();
        let smtp_port = smtp_port.clone();
        let smtp_username = smtp_username.clone();
        let smtp_password = smtp_password.clone();
        let is_loading = is_loading.clone();
        let error_msg = error_msg.clone();

        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let client = ApiClient::default();
                match client.get_mail_settings().await {
                    Ok(data) => {
                        if let Some(v) = data.get("senderName").and_then(|v| v.as_str()) {
                            sender_name.set(v.to_string());
                        }
                        if let Some(v) = data.get("senderAddress").and_then(|v| v.as_str()) {
                            sender_address.set(v.to_string());
                        }
                        if let Some(v) = data.get("smtpHost").and_then(|v| v.as_str()) {
                            smtp_host.set(v.to_string());
                        }
                        if let Some(v) = data.get("smtpPort").and_then(|v| v.as_u64()) {
                            smtp_port.set(v.to_string());
                        }
                        if let Some(v) = data.get("smtpUsername").and_then(|v| v.as_str()) {
                            smtp_username.set(v.to_string());
                        }
                        if let Some(v) = data.get("smtpPassword").and_then(|v| v.as_str()) {
                            smtp_password.set(v.to_string());
                        }
                        is_loading.set(false);
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to load settings: {}", e)));
                        is_loading.set(false);
                    }
                }
            });
            || ()
        });
    }

    let toggle_show_password = {
        let show_password = show_password.clone();
        Callback::from(move |_| {
            show_password.set(!*show_password);
        })
    };

    let on_save_click = {
        let sender_name = sender_name.clone();
        let sender_address = sender_address.clone();
        let smtp_host = smtp_host.clone();
        let smtp_port = smtp_port.clone();
        let smtp_username = smtp_username.clone();
        let smtp_password = smtp_password.clone();
        let is_saving = is_saving.clone();
        let error_msg = error_msg.clone();
        let on_save = props.on_save.clone();

        Callback::from(move |_| {
            let sender_name = (*sender_name).clone();
            let sender_address = (*sender_address).clone();
            let smtp_host = (*smtp_host).clone();
            let smtp_port = (*smtp_port).clone();
            let smtp_username = (*smtp_username).clone();
            let smtp_password = (*smtp_password).clone();
            let is_saving = is_saving.clone();
            let error_msg = error_msg.clone();
            let on_save = on_save.clone();

            is_saving.set(true);
            error_msg.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                let port: u16 = smtp_port.parse().unwrap_or(587);
                let body = serde_json::json!({
                    "senderName": sender_name,
                    "senderAddress": sender_address,
                    "smtpHost": smtp_host,
                    "smtpPort": port,
                    "smtpUsername": smtp_username,
                    "smtpPassword": smtp_password,
                });
                let client = ApiClient::default();
                match client.save_mail_settings(body).await {
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

    let on_cancel_click = {
        let sender_name = sender_name.clone();
        let sender_address = sender_address.clone();
        let smtp_host = smtp_host.clone();
        let smtp_port = smtp_port.clone();
        let smtp_username = smtp_username.clone();
        let smtp_password = smtp_password.clone();
        let error_msg = error_msg.clone();
        Callback::from(move |_| {
            sender_name.set(String::new());
            sender_address.set(String::new());
            smtp_host.set(String::new());
            smtp_port.set("587".to_string());
            smtp_username.set(String::new());
            smtp_password.set(String::new());
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
                        <span class="font-bold text-on-surface">{"Mail settings"}</span>
                    </div>
                    <h2 class="font-headline-lg text-headline-lg text-on-surface">{"Mail settings"}</h2>
                    <p class="font-body-md text-body-md text-on-surface-variant">
                        {"Configure common settings for sending emails from your Crabbase engine."}
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
                                <button onclick={
                                    let error_msg = error_msg.clone();
                                    move |_| error_msg.set(None)
                                } class="text-error hover:opacity-80">
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
                    "rounded-xl", "space-y-10", "shadow-sm",
                    if *is_loading { "opacity-60 pointer-events-none" } else { "" }
                )}>
                    /* Top Grid Section */
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                        /* Sender Name */
                        <div class="space-y-1.5">
                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider flex items-center gap-1">
                                {"Sender name "} <span class="text-primary">{"*"}</span>
                            </label>
                            <div class="relative">
                                <input
                                    class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-body-sm text-body-sm rounded-lg"
                                    placeholder="e.g. My App Support"
                                    type="text"
                                    value={(*sender_name).clone()}
                                    oninput={
                                        let sender_name = sender_name.clone();
                                        Callback::from(move |e: InputEvent| {
                                            if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                sender_name.set(target.value());
                                            }
                                        })
                                    }
                                />
                            </div>
                        </div>

                        /* Sender Address */
                        <div class="space-y-1.5">
                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider flex items-center gap-1">
                                {"Sender address "} <span class="text-primary">{"*"}</span>
                            </label>
                            <div class="relative">
                                <input
                                    class="w-full bg-surface-container p-3 pr-10 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-body-sm text-body-sm rounded-lg"
                                    placeholder="support@example.com"
                                    type="email"
                                    value={(*sender_address).clone()}
                                    oninput={
                                        let sender_address = sender_address.clone();
                                        Callback::from(move |e: InputEvent| {
                                            if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                sender_address.set(target.value());
                                            }
                                        })
                                    }
                                />
                                <span class="material-symbols-outlined absolute right-3 top-1/2 -translate-y-1/2 text-outline-variant">{"mail"}</span>
                            </div>
                        </div>
                    </div>

                    /* SMTP Settings */
                    <div id="smtp-settings" class="space-y-6">
                        <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                            <div class="md:col-span-3 space-y-1.5">
                                <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider">
                                    {"SMTP server host "} <span class="text-primary">{"*"}</span>
                                </label>
                                <input
                                    class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-code-md text-code-md rounded-lg"
                                    placeholder="smtp.postmarkapp.com"
                                    type="text"
                                    value={(*smtp_host).clone()}
                                    oninput={
                                        let smtp_host = smtp_host.clone();
                                        Callback::from(move |e: InputEvent| {
                                            if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                smtp_host.set(target.value());
                                            }
                                        })
                                    }
                                />
                            </div>
                            <div class="space-y-1.5">
                                <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider">
                                    {"Port "} <span class="text-primary">{"*"}</span>
                                </label>
                                <input
                                    class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-code-md text-code-md rounded-lg"
                                    placeholder="587"
                                    type="text"
                                    value={(*smtp_port).clone()}
                                    oninput={
                                        let smtp_port = smtp_port.clone();
                                        Callback::from(move |e: InputEvent| {
                                            if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                smtp_port.set(target.value());
                                            }
                                        })
                                    }
                                />
                            </div>
                        </div>

                        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div class="space-y-1.5">
                                <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider">
                                    {"Username"}
                                </label>
                                <input
                                    class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-code-md text-code-md rounded-lg"
                                    placeholder="API Token or Username"
                                    type="text"
                                    value={(*smtp_username).clone()}
                                    oninput={
                                        let smtp_username = smtp_username.clone();
                                        Callback::from(move |e: InputEvent| {
                                            if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                smtp_username.set(target.value());
                                            }
                                        })
                                    }
                                />
                            </div>
                            <div class="space-y-1.5">
                                <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider">
                                    {"Password"}
                                </label>
                                <div class="relative">
                                    <input
                                        class="w-full bg-surface-container p-3 pr-10 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-code-md text-code-md rounded-lg"
                                        placeholder="Your secret password"
                                        type={if *show_password { "text" } else { "password" }}
                                        value={(*smtp_password).clone()}
                                        oninput={
                                            let smtp_password = smtp_password.clone();
                                            Callback::from(move |e: InputEvent| {
                                                if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                    smtp_password.set(target.value());
                                                }
                                            })
                                        }
                                    />
                                    <button
                                        type="button"
                                        onclick={toggle_show_password}
                                        class="material-symbols-outlined absolute right-3 top-1/2 -translate-y-1/2 text-outline-variant cursor-pointer hover:text-outline transition-colors"
                                    >
                                        {if *show_password { "visibility_off" } else { "visibility" }}
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>

                    /* Actions */
                    <div class="flex items-center justify-end gap-4 pt-6 border-t border-outline-variant/30">
                        <button
                            type="button"
                            onclick={on_cancel_click}
                            class="px-6 py-2.5 font-label-xs text-label-xs font-bold text-on-surface-variant hover:bg-surface-container-high transition-colors rounded-lg active:scale-95 cursor-pointer"
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

                /* Deliverability Tip Info Box */
                <div class="mt-8 p-4 bg-tertiary-container/10 border border-tertiary-container/20 rounded-lg flex gap-4">
                    <span class="material-symbols-outlined text-tertiary">{"info"}</span>
                    <div class="space-y-1">
                        <p class="font-body-sm text-body-sm font-bold text-on-surface">{"Deliverability Tip"}</p>
                        <p class="font-body-sm text-body-sm text-on-surface-variant opacity-80">
                            {"Make sure to configure your SPF, DKIM, and DMARC records on your domain provider to ensure emails don't end up in spam folders."}
                        </p>
                    </div>
                </div>
            </div>
        </main>
    }
}
