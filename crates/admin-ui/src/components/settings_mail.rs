use crate::api::client::ApiClient;
use yew::prelude::*;

// ─── Tab ─────────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum ActiveTab {
    Smtp,
    Templates,
}

// ─── EmailTemplate state ─────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
struct TemplateState {
    subject: String,
    body_html: String,
    body_text: String,
}

impl Default for TemplateState {
    fn default() -> Self {
        Self {
            subject: String::new(),
            body_html: String::new(),
            body_text: String::new(),
        }
    }
}

// ─── Props ───────────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
pub struct SettingsMailProps {
    #[prop_or_default]
    pub on_save: Option<Callback<()>>,
}

// ─── Component ───────────────────────────────────────────────────────────────

#[function_component(SettingsMail)]
pub fn settings_mail(props: &SettingsMailProps) -> Html {
    // ── Tab ──────────────────────────────────────────────────────────────────
    let active_tab = use_state(|| ActiveTab::Smtp);

    // ── SMTP state ───────────────────────────────────────────────────────────
    let sender_name = use_state(|| String::new());
    let sender_address = use_state(|| String::new());
    let smtp_host = use_state(|| String::new());
    let smtp_port = use_state(|| "587".to_string());
    let smtp_username = use_state(|| String::new());
    let smtp_password = use_state(|| String::new());
    let show_password = use_state(|| false);
    let is_smtp_saving = use_state(|| false);

    // ── Template state ───────────────────────────────────────────────────────
    let tmpl_password_reset = use_state(|| TemplateState::default());
    let tmpl_user_registration = use_state(|| TemplateState::default());
    let expanded_card: UseStateHandle<Option<&'static str>> = use_state(|| None);
    let is_tmpl_saving = use_state(|| false);

    // ── Shared ───────────────────────────────────────────────────────────────
    let is_loading = use_state(|| true);
    let error_msg = use_state(|| Option::<String>::None);
    let success_msg = use_state(|| Option::<String>::None);

    // ── Load both settings on mount ───────────────────────────────────────────
    {
        let sender_name = sender_name.clone();
        let sender_address = sender_address.clone();
        let smtp_host = smtp_host.clone();
        let smtp_port = smtp_port.clone();
        let smtp_username = smtp_username.clone();
        let smtp_password = smtp_password.clone();
        let tmpl_password_reset = tmpl_password_reset.clone();
        let tmpl_user_registration = tmpl_user_registration.clone();
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
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to load SMTP settings: {}", e)));
                    }
                }
                match client.get_email_templates().await {
                    Ok(data) => {
                        if let Some(pr) = data.get("passwordReset") {
                            tmpl_password_reset.set(TemplateState {
                                subject: pr
                                    .get("subject")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                body_html: pr
                                    .get("bodyHtml")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                body_text: pr
                                    .get("bodyText")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            });
                        }
                        if let Some(ur) = data.get("userRegistration") {
                            tmpl_user_registration.set(TemplateState {
                                subject: ur
                                    .get("subject")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                body_html: ur
                                    .get("bodyHtml")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                body_text: ur
                                    .get("bodyText")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            });
                        }
                    }
                    Err(_) => {} // Templates not yet configured – keep defaults
                }
                is_loading.set(false);
            });
            || ()
        });
    }

    // ── Tab switch callbacks ──────────────────────────────────────────────────

    let switch_to_smtp = {
        let tab = active_tab.clone();
        let error_msg = error_msg.clone();
        let success_msg = success_msg.clone();
        Callback::from(move |_: MouseEvent| {
            tab.set(ActiveTab::Smtp);
            error_msg.set(None);
            success_msg.set(None);
        })
    };

    let switch_to_templates = {
        let tab = active_tab.clone();
        let error_msg = error_msg.clone();
        let success_msg = success_msg.clone();
        Callback::from(move |_: MouseEvent| {
            tab.set(ActiveTab::Templates);
            error_msg.set(None);
            success_msg.set(None);
        })
    };

    // ── Derived tab flags (computed before html!) ─────────────────────────────
    let is_smtp_tab = *active_tab == ActiveTab::Smtp;
    let is_tmpl_tab = *active_tab == ActiveTab::Templates;

    // ── SMTP callbacks ────────────────────────────────────────────────────────

    let toggle_show_password = {
        let show_password = show_password.clone();
        Callback::from(move |_| show_password.set(!*show_password))
    };

    let on_smtp_save = {
        let sender_name = sender_name.clone();
        let sender_address = sender_address.clone();
        let smtp_host = smtp_host.clone();
        let smtp_port = smtp_port.clone();
        let smtp_username = smtp_username.clone();
        let smtp_password = smtp_password.clone();
        let is_smtp_saving = is_smtp_saving.clone();
        let error_msg = error_msg.clone();
        let success_msg = success_msg.clone();
        let on_save = props.on_save.clone();

        Callback::from(move |_| {
            let sender_name = (*sender_name).clone();
            let sender_address = (*sender_address).clone();
            let smtp_host = (*smtp_host).clone();
            let smtp_port = (*smtp_port).clone();
            let smtp_username = (*smtp_username).clone();
            let smtp_password = (*smtp_password).clone();
            let is_smtp_saving = is_smtp_saving.clone();
            let error_msg = error_msg.clone();
            let success_msg = success_msg.clone();
            let on_save = on_save.clone();

            is_smtp_saving.set(true);
            error_msg.set(None);
            success_msg.set(None);

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
                        success_msg.set(Some("SMTP settings saved successfully.".to_string()));
                        if let Some(ref cb) = on_save {
                            cb.emit(());
                        }
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to save settings: {}", e)));
                    }
                }
                is_smtp_saving.set(false);
            });
        })
    };

    let on_smtp_cancel = {
        let sender_name = sender_name.clone();
        let sender_address = sender_address.clone();
        let smtp_host = smtp_host.clone();
        let smtp_port = smtp_port.clone();
        let smtp_username = smtp_username.clone();
        let smtp_password = smtp_password.clone();
        let error_msg = error_msg.clone();
        let success_msg = success_msg.clone();
        Callback::from(move |_| {
            sender_name.set(String::new());
            sender_address.set(String::new());
            smtp_host.set(String::new());
            smtp_port.set("587".to_string());
            smtp_username.set(String::new());
            smtp_password.set(String::new());
            error_msg.set(None);
            success_msg.set(None);
        })
    };

    // ── Template callbacks ────────────────────────────────────────────────────

    let on_tmpl_save = {
        let tmpl_password_reset = tmpl_password_reset.clone();
        let tmpl_user_registration = tmpl_user_registration.clone();
        let is_tmpl_saving = is_tmpl_saving.clone();
        let error_msg = error_msg.clone();
        let success_msg = success_msg.clone();

        Callback::from(move |_| {
            let pr = (*tmpl_password_reset).clone();
            let ur = (*tmpl_user_registration).clone();
            let is_tmpl_saving = is_tmpl_saving.clone();
            let error_msg = error_msg.clone();
            let success_msg = success_msg.clone();

            is_tmpl_saving.set(true);
            error_msg.set(None);
            success_msg.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                let body = serde_json::json!({
                    "passwordReset": {
                        "key": "password_reset",
                        "name": "Password Reset",
                        "subject": pr.subject,
                        "bodyHtml": pr.body_html,
                        "bodyText": pr.body_text,
                    },
                    "userRegistration": {
                        "key": "user_registration",
                        "name": "User Registration",
                        "subject": ur.subject,
                        "bodyHtml": ur.body_html,
                        "bodyText": ur.body_text,
                    }
                });
                let client = ApiClient::default();
                match client.save_email_templates(body).await {
                    Ok(_) => {
                        success_msg.set(Some("Email templates saved successfully.".to_string()));
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to save templates: {}", e)));
                    }
                }
                is_tmpl_saving.set(false);
            });
        })
    };

    // ── Render ────────────────────────────────────────────────────────────────

    html! {
        <main class="flex-1 overflow-y-auto p-margin_page bg-surface-container-low">
            <div class="max-w-4xl mx-auto">

                // ── Page header ───────────────────────────────────────────────
                <div class="flex flex-col gap-1 mb-8">
                    <div class="flex items-center gap-2 text-on-surface-variant font-label-xs text-label-xs mb-1 opacity-80">
                        <span>{"Settings"}</span>
                        <span class="material-symbols-outlined text-[12px]">{"chevron_right"}</span>
                        <span class="font-bold text-on-surface">{"Mail settings"}</span>
                    </div>
                    <h2 class="font-headline-lg text-headline-lg text-on-surface">{"Mail settings"}</h2>
                    <p class="font-body-md text-body-md text-on-surface-variant">
                        {"Configure SMTP settings and customize email templates for your Crabbase engine."}
                    </p>
                </div>

                // ── Tabs ──────────────────────────────────────────────────────
                <div class="flex gap-1 mb-6 border-b border-outline-variant/40">
                    <button
                        type="button"
                        onclick={switch_to_smtp}
                        class={classes!(
                            "px-4", "py-2.5", "font-label-xs", "text-label-xs", "font-bold",
                            "border-b-2", "transition-colors", "cursor-pointer",
                            if is_smtp_tab {
                                "border-primary text-primary"
                            } else {
                                "border-transparent text-on-surface-variant hover:text-on-surface"
                            }
                        )}
                    >
                        <span class="flex items-center gap-2">
                            <span class="material-symbols-outlined text-[16px]">{"settings_ethernet"}</span>
                            {"SMTP Settings"}
                        </span>
                    </button>
                    <button
                        type="button"
                        onclick={switch_to_templates}
                        class={classes!(
                            "px-4", "py-2.5", "font-label-xs", "text-label-xs", "font-bold",
                            "border-b-2", "transition-colors", "cursor-pointer",
                            if is_tmpl_tab {
                                "border-primary text-primary"
                            } else {
                                "border-transparent text-on-surface-variant hover:text-on-surface"
                            }
                        )}
                    >
                        <span class="flex items-center gap-2">
                            <span class="material-symbols-outlined text-[16px]">{"mail"}</span>
                            {"Email Templates"}
                        </span>
                    </button>
                </div>

                // ── Banners ───────────────────────────────────────────────────
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
                    } else if let Some(ref msg) = *success_msg {
                        html! {
                            <div class="mb-6 p-4 bg-tertiary-container/10 border border-tertiary-container/30 text-tertiary rounded-lg flex items-center justify-between">
                                <div class="flex items-center gap-2">
                                    <span class="material-symbols-outlined">{"check_circle"}</span>
                                    <span class="font-body-sm text-body-sm font-bold">{msg.clone()}</span>
                                </div>
                                <button onclick={
                                    let success_msg = success_msg.clone();
                                    move |_| success_msg.set(None)
                                } class="text-tertiary hover:opacity-80">
                                    <span class="material-symbols-outlined text-sm">{"close"}</span>
                                </button>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }

                // ── Tab panels ────────────────────────────────────────────────
                <div class={classes!(if *is_loading { "opacity-60 pointer-events-none" } else { "" })}>
                    { if is_smtp_tab {
                        html! { <SmtpPanel
                            sender_name={(*sender_name).clone()}
                            sender_address={(*sender_address).clone()}
                            smtp_host={(*smtp_host).clone()}
                            smtp_port={(*smtp_port).clone()}
                            smtp_username={(*smtp_username).clone()}
                            smtp_password={(*smtp_password).clone()}
                            show_password={*show_password}
                            is_saving={*is_smtp_saving}
                            on_sender_name_change={{
                                let s = sender_name.clone();
                                Callback::from(move |v: String| s.set(v))
                            }}
                            on_sender_address_change={{
                                let s = sender_address.clone();
                                Callback::from(move |v: String| s.set(v))
                            }}
                            on_smtp_host_change={{
                                let s = smtp_host.clone();
                                Callback::from(move |v: String| s.set(v))
                            }}
                            on_smtp_port_change={{
                                let s = smtp_port.clone();
                                Callback::from(move |v: String| s.set(v))
                            }}
                            on_smtp_username_change={{
                                let s = smtp_username.clone();
                                Callback::from(move |v: String| s.set(v))
                            }}
                            on_smtp_password_change={{
                                let s = smtp_password.clone();
                                Callback::from(move |v: String| s.set(v))
                            }}
                            on_toggle_password={toggle_show_password}
                            on_save={on_smtp_save}
                            on_cancel={on_smtp_cancel}
                        /> }
                    } else {
                        html! { <TemplatesPanel
                            password_reset={(*tmpl_password_reset).clone()}
                            user_registration={(*tmpl_user_registration).clone()}
                            expanded_card={(*expanded_card).clone()}
                            is_saving={*is_tmpl_saving}
                            on_password_reset_change={{
                                let s = tmpl_password_reset.clone();
                                Callback::from(move |v: TemplateState| s.set(v))
                            }}
                            on_user_registration_change={{
                                let s = tmpl_user_registration.clone();
                                Callback::from(move |v: TemplateState| s.set(v))
                            }}
                            on_expand={{
                                let s = expanded_card.clone();
                                Callback::from(move |key: Option<&'static str>| s.set(key))
                            }}
                            on_save={on_tmpl_save}
                        /> }
                    }}
                </div>

                // ── Deliverability tip ────────────────────────────────────────
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

// ─── SMTP Panel ───────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct SmtpPanelProps {
    sender_name: String,
    sender_address: String,
    smtp_host: String,
    smtp_port: String,
    smtp_username: String,
    smtp_password: String,
    show_password: bool,
    is_saving: bool,
    on_sender_name_change: Callback<String>,
    on_sender_address_change: Callback<String>,
    on_smtp_host_change: Callback<String>,
    on_smtp_port_change: Callback<String>,
    on_smtp_username_change: Callback<String>,
    on_smtp_password_change: Callback<String>,
    on_toggle_password: Callback<MouseEvent>,
    on_save: Callback<MouseEvent>,
    on_cancel: Callback<MouseEvent>,
}

#[function_component(SmtpPanel)]
fn smtp_panel(p: &SmtpPanelProps) -> Html {
    html! {
        <div class="bg-surface-container-lowest border border-outline-variant p-8 rounded-xl space-y-10 shadow-sm">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div class="space-y-1.5">
                    <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider flex items-center gap-1">
                        {"Sender name "}<span class="text-primary">{"*"}</span>
                    </label>
                    <input
                        class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-body-sm text-body-sm rounded-lg"
                        placeholder="e.g. My App Support"
                        type="text"
                        value={p.sender_name.clone()}
                        oninput={{
                            let cb = p.on_sender_name_change.clone();
                            Callback::from(move |e: InputEvent| {
                                if let Some(t) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                    cb.emit(t.value());
                                }
                            })
                        }}
                    />
                </div>
                <div class="space-y-1.5">
                    <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider flex items-center gap-1">
                        {"Sender address "}<span class="text-primary">{"*"}</span>
                    </label>
                    <div class="relative">
                        <input
                            class="w-full bg-surface-container p-3 pr-10 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-body-sm text-body-sm rounded-lg"
                            placeholder="support@example.com"
                            type="email"
                            value={p.sender_address.clone()}
                            oninput={{
                                let cb = p.on_sender_address_change.clone();
                                Callback::from(move |e: InputEvent| {
                                    if let Some(t) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                        cb.emit(t.value());
                                    }
                                })
                            }}
                        />
                        <span class="material-symbols-outlined absolute right-3 top-1/2 -translate-y-1/2 text-outline-variant">{"mail"}</span>
                    </div>
                </div>
            </div>

            <div class="space-y-6">
                <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                    <div class="md:col-span-3 space-y-1.5">
                        <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider">
                            {"SMTP server host "}<span class="text-primary">{"*"}</span>
                        </label>
                        <input
                            class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-code-md text-code-md rounded-lg"
                            placeholder="smtp.postmarkapp.com"
                            type="text"
                            value={p.smtp_host.clone()}
                            oninput={{
                                let cb = p.on_smtp_host_change.clone();
                                Callback::from(move |e: InputEvent| {
                                    if let Some(t) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                        cb.emit(t.value());
                                    }
                                })
                            }}
                        />
                    </div>
                    <div class="space-y-1.5">
                        <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider">
                            {"Port "}<span class="text-primary">{"*"}</span>
                        </label>
                        <input
                            class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-code-md text-code-md rounded-lg"
                            placeholder="587"
                            type="text"
                            value={p.smtp_port.clone()}
                            oninput={{
                                let cb = p.on_smtp_port_change.clone();
                                Callback::from(move |e: InputEvent| {
                                    if let Some(t) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                        cb.emit(t.value());
                                    }
                                })
                            }}
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
                            value={p.smtp_username.clone()}
                            oninput={{
                                let cb = p.on_smtp_username_change.clone();
                                Callback::from(move |e: InputEvent| {
                                    if let Some(t) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                        cb.emit(t.value());
                                    }
                                })
                            }}
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
                                type={if p.show_password { "text" } else { "password" }}
                                value={p.smtp_password.clone()}
                                oninput={{
                                    let cb = p.on_smtp_password_change.clone();
                                    Callback::from(move |e: InputEvent| {
                                        if let Some(t) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                            cb.emit(t.value());
                                        }
                                    })
                                }}
                            />
                            <button
                                type="button"
                                onclick={p.on_toggle_password.clone()}
                                class="material-symbols-outlined absolute right-3 top-1/2 -translate-y-1/2 text-outline-variant cursor-pointer hover:text-outline transition-colors"
                            >
                                {if p.show_password { "visibility_off" } else { "visibility" }}
                            </button>
                        </div>
                    </div>
                </div>
            </div>

            <div class="flex items-center justify-end gap-4 pt-6 border-t border-outline-variant/30">
                <button
                    type="button"
                    onclick={p.on_cancel.clone()}
                    class="px-6 py-2.5 font-label-xs text-label-xs font-bold text-on-surface-variant hover:bg-surface-container-high transition-colors rounded-lg active:scale-95 cursor-pointer"
                >
                    {"Cancel"}
                </button>
                <button
                    type="button"
                    onclick={p.on_save.clone()}
                    disabled={p.is_saving}
                    class={classes!(
                        "px-8", "py-2.5", "font-label-xs", "text-label-xs", "font-bold",
                        "rounded-lg", "shadow-sm", "transition-all",
                        if p.is_saving {
                            "bg-primary/60 text-on-primary cursor-not-allowed"
                        } else {
                            "bg-primary text-on-primary active:scale-[0.98] hover:bg-on-primary-fixed-variant cursor-pointer"
                        }
                    )}
                >
                    { if p.is_saving { "Saving..." } else { "Save changes" } }
                </button>
            </div>
        </div>
    }
}

// ─── Templates Panel ──────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct TemplatesPanelProps {
    password_reset: TemplateState,
    user_registration: TemplateState,
    expanded_card: Option<&'static str>,
    is_saving: bool,
    on_password_reset_change: Callback<TemplateState>,
    on_user_registration_change: Callback<TemplateState>,
    on_expand: Callback<Option<&'static str>>,
    on_save: Callback<MouseEvent>,
}

#[function_component(TemplatesPanel)]
fn templates_panel(p: &TemplatesPanelProps) -> Html {
    let pr_expanded = p.expanded_card == Some("password_reset");
    let ur_expanded = p.expanded_card == Some("user_registration");

    let on_expand_pr = {
        let cb = p.on_expand.clone();
        Callback::from(move |_: MouseEvent| {
            cb.emit(if pr_expanded {
                None
            } else {
                Some("password_reset")
            });
        })
    };
    let on_expand_ur = {
        let cb = p.on_expand.clone();
        Callback::from(move |_: MouseEvent| {
            cb.emit(if ur_expanded {
                None
            } else {
                Some("user_registration")
            });
        })
    };

    html! {
        <div class="space-y-4">
            <p class="font-body-sm text-body-sm text-on-surface-variant mb-2">
                {"Customize the email content sent to users for each operation. Use "}
                <code class="bg-surface-container px-1.5 py-0.5 rounded text-primary font-code-sm text-code-sm">{"{{variable}}"}</code>
                {" placeholders for dynamic values."}
            </p>

            <TemplateCard
                title="Password Reset"
                icon="lock_reset"
                description="Sent when a user requests a password reset."
                template={p.password_reset.clone()}
                is_expanded={pr_expanded}
                on_expand={on_expand_pr}
                on_change={p.on_password_reset_change.clone()}
            />

            <TemplateCard
                title="User Registration"
                icon="person_add"
                description="Sent when a new user successfully creates an account."
                template={p.user_registration.clone()}
                is_expanded={ur_expanded}
                on_expand={on_expand_ur}
                on_change={p.on_user_registration_change.clone()}
            />

            <div class="flex items-center justify-end gap-4 pt-4">
                <button
                    type="button"
                    onclick={p.on_save.clone()}
                    disabled={p.is_saving}
                    class={classes!(
                        "px-8", "py-2.5", "font-label-xs", "text-label-xs", "font-bold",
                        "rounded-lg", "shadow-sm", "transition-all",
                        if p.is_saving {
                            "bg-primary/60 text-on-primary cursor-not-allowed"
                        } else {
                            "bg-primary text-on-primary active:scale-[0.98] hover:bg-on-primary-fixed-variant cursor-pointer"
                        }
                    )}
                >
                    { if p.is_saving { "Saving..." } else { "Save templates" } }
                </button>
            </div>
        </div>
    }
}

// ─── Template Card ────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct TemplateCardProps {
    title: &'static str,
    icon: &'static str,
    description: &'static str,
    template: TemplateState,
    is_expanded: bool,
    on_expand: Callback<MouseEvent>,
    on_change: Callback<TemplateState>,
}

#[function_component(TemplateCard)]
fn template_card(p: &TemplateCardProps) -> Html {
    let body_tab = use_state(|| "html");
    let is_html = *body_tab == "html";
    let is_preview = *body_tab == "preview";

    let switch_html = {
        let bt = body_tab.clone();
        Callback::from(move |_: MouseEvent| bt.set("html"))
    };
    let switch_plain = {
        let bt = body_tab.clone();
        Callback::from(move |_: MouseEvent| bt.set("plain"))
    };
    let switch_preview = {
        let bt = body_tab.clone();
        Callback::from(move |_: MouseEvent| bt.set("preview"))
    };

    let template = p.template.clone();
    let on_change = p.on_change.clone();

    let on_subject = {
        let t = template.clone();
        let cb = on_change.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(el) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                cb.emit(TemplateState {
                    subject: el.value(),
                    body_html: t.body_html.clone(),
                    body_text: t.body_text.clone(),
                });
            }
        })
    };

    let on_body_html = {
        let t = template.clone();
        let cb = on_change.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(el) = e.target_dyn_into::<web_sys::HtmlTextAreaElement>() {
                cb.emit(TemplateState {
                    subject: t.subject.clone(),
                    body_html: el.value(),
                    body_text: t.body_text.clone(),
                });
            }
        })
    };

    let on_body_text = {
        let t = template.clone();
        let cb = on_change.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(el) = e.target_dyn_into::<web_sys::HtmlTextAreaElement>() {
                cb.emit(TemplateState {
                    subject: t.subject.clone(),
                    body_html: t.body_html.clone(),
                    body_text: el.value(),
                });
            }
        })
    };

    html! {
        <div class="bg-surface-container-lowest border border-outline-variant rounded-xl shadow-sm overflow-hidden">
            // Card header
            <button
                type="button"
                onclick={p.on_expand.clone()}
                class="w-full flex items-center justify-between p-5 hover:bg-surface-container/40 transition-colors cursor-pointer"
            >
                <div class="flex items-center gap-3">
                    <div class="w-9 h-9 rounded-lg bg-primary/10 flex items-center justify-center">
                        <span class="material-symbols-outlined text-primary text-[18px]">{p.icon}</span>
                    </div>
                    <div class="text-left">
                        <p class="font-label-sm text-label-sm font-bold text-on-surface">{p.title}</p>
                        <p class="font-body-xs text-body-xs text-on-surface-variant mt-0.5">{p.description}</p>
                    </div>
                </div>
                <span class={classes!(
                    "material-symbols-outlined", "text-outline-variant", "transition-transform", "duration-200",
                    if p.is_expanded { "rotate-180" } else { "" }
                )}>
                    {"expand_more"}
                </span>
            </button>

            // Expanded editor
            { if p.is_expanded {
                html! {
                    <div class="border-t border-outline-variant/40 p-5 space-y-5">
                        // Subject
                        <div class="space-y-1.5">
                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider">
                                {"Email subject"}
                            </label>
                            <input
                                class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-body-sm text-body-sm rounded-lg"
                                placeholder="e.g. Reset your password"
                                type="text"
                                value={template.subject.clone()}
                                oninput={on_subject}
                            />
                        </div>

                        // Body with HTML / Plain text / Preview sub-tabs
                        <div class="space-y-2">
                            <div class="flex items-center justify-between">
                                <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider">
                                    {"Email body"}
                                </label>
                                <div class="flex gap-1 bg-surface-container rounded-lg p-0.5">
                                    <button
                                        type="button"
                                        onclick={switch_html}
                                        class={classes!(
                                            "px-3", "py-1", "font-label-xs", "text-label-xs", "rounded-md", "transition-colors", "cursor-pointer",
                                            if is_html { "bg-surface-container-highest text-on-surface font-bold shadow-sm" } else { "text-on-surface-variant" }
                                        )}
                                    >{"HTML"}</button>
                                    <button
                                        type="button"
                                        onclick={switch_plain}
                                        class={classes!(
                                            "px-3", "py-1", "font-label-xs", "text-label-xs", "rounded-md", "transition-colors", "cursor-pointer",
                                            if *body_tab == "plain" { "bg-surface-container-highest text-on-surface font-bold shadow-sm" } else { "text-on-surface-variant" }
                                        )}
                                    >{"Plain text"}</button>
                                    <button
                                        type="button"
                                        onclick={switch_preview}
                                        class={classes!(
                                            "flex", "items-center", "gap-1",
                                            "px-3", "py-1", "font-label-xs", "text-label-xs", "rounded-md", "transition-colors", "cursor-pointer",
                                            if is_preview { "bg-primary/10 text-primary font-bold shadow-sm" } else { "text-on-surface-variant" }
                                        )}
                                    >
                                        <span class="material-symbols-outlined text-[14px]">{"visibility"}</span>
                                        {"Preview"}
                                    </button>
                                </div>
                            </div>

                            { if is_html {
                                html! {
                                    <div class="relative">
                                        <textarea
                                            class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-code-sm text-code-sm rounded-lg resize-y min-h-[200px]"
                                            placeholder="<html><body>...</body></html>"
                                            value={template.body_html.clone()}
                                            oninput={on_body_html}
                                        />
                                        <span class="absolute top-2 right-3 font-label-xs text-label-xs text-outline-variant select-none">{"HTML"}</span>
                                    </div>
                                }
                            } else if is_preview {
                                html! {
                                    <div class="rounded-lg border border-outline-variant overflow-hidden">
                                        // Preview toolbar
                                        <div class="flex items-center gap-2 px-4 py-2 bg-surface-container border-b border-outline-variant/40">
                                            <span class="material-symbols-outlined text-[14px] text-outline-variant">{"visibility"}</span>
                                            <span class="font-label-xs text-label-xs text-on-surface-variant">{"HTML Preview"}</span>
                                            <span class="ml-auto font-label-xs text-label-xs text-outline-variant italic">{"Scripts and external requests are blocked"}</span>
                                        </div>
                                        // Sandboxed iframe rendering the HTML
                                        { if template.body_html.is_empty() {
                                            html! {
                                                <div class="flex flex-col items-center justify-center h-48 gap-2 text-on-surface-variant bg-surface-container/40">
                                                    <span class="material-symbols-outlined text-[32px] opacity-40">{"mail"}</span>
                                                    <p class="font-body-sm text-body-sm opacity-60">{"No HTML content yet. Write some HTML to see the preview."}</p>
                                                </div>
                                            }
                                        } else {
                                            html! {
                                                <iframe
                                                    srcdoc={template.body_html.clone()}
                                                    sandbox="allow-same-origin"
                                                    class="w-full border-0 bg-white"
                                                    style="min-height: 480px; display: block;"
                                                    title="Email preview"
                                                />
                                            }
                                        }}
                                    </div>
                                }
                            } else {
                                html! {
                                    <div class="relative">
                                        <textarea
                                            class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-body-sm text-body-sm rounded-lg resize-y min-h-[160px]"
                                            placeholder="Plain text fallback for email clients that don't support HTML."
                                            value={template.body_text.clone()}
                                            oninput={on_body_text}
                                        />
                                        <span class="absolute top-2 right-3 font-label-xs text-label-xs text-outline-variant select-none">{"PLAIN"}</span>
                                    </div>
                                }
                            }}
                        </div>

                        // Variable hints
                        <div class="flex flex-wrap gap-2 items-center">
                            <span class="font-label-xs text-label-xs text-on-surface-variant">{"Available variables:"}</span>
                            { for ["{{name}}", "{{email}}", "{{app_name}}"].iter().map(|v| html! {
                                <span class="px-2 py-0.5 bg-surface-container rounded text-primary font-code-sm text-code-sm">{*v}</span>
                            })}
                        </div>
                    </div>
                }
            } else {
                html! {}
            }}
        </div>
    }
}
