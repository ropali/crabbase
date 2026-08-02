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
    let smtp_enabled = use_state(|| false);
    let smtp_host = use_state(|| String::new());
    let smtp_port = use_state(|| "587".to_string());
    let smtp_username = use_state(|| String::new());
    let smtp_password = use_state(|| String::new());
    let show_password = use_state(|| false);
    let show_more_options = use_state(|| false);
    let is_saved = use_state(|| false);

    let encryption = use_state(|| "TLS".to_string());
    let auth_method = use_state(|| "PLAIN".to_string());
    let timeout_seconds = use_state(|| "30".to_string());

    let toggle_smtp = {
        let smtp_enabled = smtp_enabled.clone();
        Callback::from(move |_| {
            smtp_enabled.set(!*smtp_enabled);
        })
    };

    let toggle_show_password = {
        let show_password = show_password.clone();
        Callback::from(move |_| {
            show_password.set(!*show_password);
        })
    };

    let toggle_show_more = {
        let show_more_options = show_more_options.clone();
        Callback::from(move |_| {
            show_more_options.set(!*show_more_options);
        })
    };

    let on_save_click = {
        let is_saved = is_saved.clone();
        let on_save = props.on_save.clone();
        Callback::from(move |_| {
            is_saved.set(true);
            if let Some(ref cb) = on_save {
                cb.emit(());
            }
        })
    };

    let on_cancel_click = {
        let sender_name = sender_name.clone();
        let sender_address = sender_address.clone();
        let smtp_enabled = smtp_enabled.clone();
        let smtp_host = smtp_host.clone();
        let smtp_port = smtp_port.clone();
        let smtp_username = smtp_username.clone();
        let smtp_password = smtp_password.clone();
        let show_more_options = show_more_options.clone();
        let is_saved = is_saved.clone();
        Callback::from(move |_| {
            sender_name.set(String::new());
            sender_address.set(String::new());
            smtp_enabled.set(false);
            smtp_host.set(String::new());
            smtp_port.set("587".to_string());
            smtp_username.set(String::new());
            smtp_password.set(String::new());
            show_more_options.set(false);
            is_saved.set(false);
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
                    if *is_saved {
                        html! {
                            <div class="mb-6 p-4 bg-primary-container/10 border border-primary-container/30 text-primary rounded-lg flex items-center justify-between transition-all">
                                <div class="flex items-center gap-2">
                                    <span class="material-symbols-outlined">{"check_circle"}</span>
                                    <span class="font-body-sm text-body-sm font-bold">{"Mail settings saved successfully!"}</span>
                                </div>
                                <button onclick={
                                    let is_saved = is_saved.clone();
                                    move |_| is_saved.set(false)
                                } class="text-primary hover:opacity-80">
                                    <span class="material-symbols-outlined text-sm">{"close"}</span>
                                </button>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }

                <div class="bg-surface-container-lowest border border-outline-variant p-8 rounded-xl space-y-10 shadow-sm">
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

                    /* SMTP Toggle */
                    <div class="flex items-center gap-4 py-4 border-y border-outline-variant/30">
                        <button
                            id="smtp-toggle"
                            type="button"
                            onclick={toggle_smtp}
                            class={classes!(
                                "relative", "inline-flex", "h-6", "w-11", "shrink-0", "cursor-pointer",
                                "rounded-full", "border-2", "border-transparent", "transition-colors",
                                "duration-200", "ease-in-out", "focus:outline-none",
                                if *smtp_enabled { "bg-primary-container" } else { "bg-outline-variant" }
                            )}
                        >
                            <span
                                id="smtp-toggle-thumb"
                                class={classes!(
                                    "pointer-events-none", "inline-block", "h-5", "w-5", "transform",
                                    "rounded-full", "bg-white", "shadow", "ring-0", "transition",
                                    "duration-200", "ease-in-out",
                                    if *smtp_enabled { "translate-x-5" } else { "translate-x-0" }
                                )}
                            />
                        </button>
                        <div class="flex items-center gap-1.5">
                            <span class="font-body-md text-body-md font-bold">{"Use SMTP mail server (recommended)"}</span>
                            <span
                                class="material-symbols-outlined text-[16px] text-outline cursor-help"
                                title="Using an external SMTP server is highly recommended for production apps."
                            >
                                {"info"}
                            </span>
                        </div>
                    </div>

                    /* SMTP Form Grid */
                    <div
                        id="smtp-settings"
                        class={classes!(
                            "space-y-6", "transition-all", "duration-300",
                            if *smtp_enabled { "opacity-100" } else { "opacity-50 pointer-events-none" }
                        )}
                    >
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
                                    disabled={!*smtp_enabled}
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
                                    disabled={!*smtp_enabled}
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
                                    disabled={!*smtp_enabled}
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
                                        disabled={!*smtp_enabled}
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

                        /* Show more options button & panel */
                        <div class="flex flex-col pt-2 gap-4">
                            <div>
                                <button
                                    type="button"
                                    onclick={toggle_show_more}
                                    class="flex items-center gap-2 px-3 py-1.5 border border-outline-variant rounded-lg font-label-xs text-label-xs text-on-surface-variant hover:bg-surface-container-high transition-colors active:scale-95 cursor-pointer"
                                >
                                    {if *show_more_options { "Hide options" } else { "Show more options" }}
                                    <span class="material-symbols-outlined text-[14px]">
                                        {if *show_more_options { "expand_less" } else { "expand_more" }}
                                    </span>
                                </button>
                            </div>

                            {
                                if *show_more_options {
                                    html! {
                                        <div class="p-4 bg-surface-container-low border border-outline-variant/60 rounded-lg space-y-4">
                                            <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                                                <div class="space-y-1.5">
                                                    <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider">
                                                        {"Encryption"}
                                                    </label>
                                                    <select
                                                        class="w-full bg-surface-container p-2.5 border border-outline-variant rounded-lg font-body-sm text-body-sm outline-none focus:border-primary"
                                                        value={(*encryption).clone()}
                                                        onchange={
                                                            let encryption = encryption.clone();
                                                            Callback::from(move |e: Event| {
                                                                if let Some(target) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                                                                    encryption.set(target.value());
                                                                }
                                                            })
                                                        }
                                                    >
                                                        <option value="TLS">{"TLS (STARTTLS)"}</option>
                                                        <option value="SSL">{"SSL/TLS"}</option>
                                                        <option value="NONE">{"None (Plain)"}</option>
                                                    </select>
                                                </div>
                                                <div class="space-y-1.5">
                                                    <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider">
                                                        {"Auth Method"}
                                                    </label>
                                                    <select
                                                        class="w-full bg-surface-container p-2.5 border border-outline-variant rounded-lg font-body-sm text-body-sm outline-none focus:border-primary"
                                                        value={(*auth_method).clone()}
                                                        onchange={
                                                            let auth_method = auth_method.clone();
                                                            Callback::from(move |e: Event| {
                                                                if let Some(target) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                                                                    auth_method.set(target.value());
                                                                }
                                                            })
                                                        }
                                                    >
                                                        <option value="PLAIN">{"PLAIN"}</option>
                                                        <option value="LOGIN">{"LOGIN"}</option>
                                                        <option value="CRAM-MD5">{"CRAM-MD5"}</option>
                                                    </select>
                                                </div>
                                                <div class="space-y-1.5">
                                                    <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider">
                                                        {"Timeout (sec)"}
                                                    </label>
                                                    <input
                                                        type="number"
                                                        class="w-full bg-surface-container p-2.5 border border-outline-variant rounded-lg font-code-md text-code-md outline-none focus:border-primary"
                                                        value={(*timeout_seconds).clone()}
                                                        oninput={
                                                            let timeout_seconds = timeout_seconds.clone();
                                                            Callback::from(move |e: InputEvent| {
                                                                if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                                    timeout_seconds.set(target.value());
                                                                }
                                                            })
                                                        }
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }
                            }
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
                            class="bg-primary text-on-primary px-8 py-2.5 font-label-xs text-label-xs font-bold rounded-lg shadow-sm active:scale-[0.98] transition-all hover:bg-on-primary-fixed-variant cursor-pointer"
                        >
                            {"Save changes"}
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
