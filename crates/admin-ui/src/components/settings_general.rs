use yew::prelude::*;

#[function_component(SettingsGeneral)]
pub fn settings_general() -> Html {
    let app_name = use_state(|| "Crabbase Engine".to_string());
    let app_url = use_state(|| "http://localhost:8989".to_string());
    let admin_email = use_state(|| "admin@crabbase.io".to_string());
    let public_registration = use_state(|| false);
    let is_saved = use_state(|| false);

    let toggle_registration = {
        let public_registration = public_registration.clone();
        Callback::from(move |_| {
            public_registration.set(!*public_registration);
        })
    };

    let on_save = {
        let is_saved = is_saved.clone();
        Callback::from(move |_| {
            is_saved.set(true);
        })
    };

    let on_cancel = {
        let app_name = app_name.clone();
        let app_url = app_url.clone();
        let admin_email = admin_email.clone();
        let public_registration = public_registration.clone();
        let is_saved = is_saved.clone();
        Callback::from(move |_| {
            app_name.set("Crabbase Engine".to_string());
            app_url.set("http://localhost:8989".to_string());
            admin_email.set("admin@crabbase.io".to_string());
            public_registration.set(false);
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
                        <span class="font-bold text-on-surface">{"General"}</span>
                    </div>
                    <h2 class="font-headline-lg text-headline-lg text-on-surface">{"General settings"}</h2>
                    <p class="font-body-md text-body-md text-on-surface-variant">
                        {"Manage your Crabbase engine app configuration, identity, and access controls."}
                    </p>
                </div>

                {
                    if *is_saved {
                        html! {
                            <div class="mb-6 p-4 bg-primary-container/10 border border-primary-container/30 text-primary rounded-lg flex items-center justify-between transition-all">
                                <div class="flex items-center gap-2">
                                    <span class="material-symbols-outlined">{"check_circle"}</span>
                                    <span class="font-body-sm text-body-sm font-bold">{"General settings saved successfully!"}</span>
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

                <div class="bg-surface-container-lowest border border-outline-variant p-8 rounded-xl space-y-8 shadow-sm">
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                        <div class="space-y-1.5">
                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider flex items-center gap-1">
                                {"Application Name "} <span class="text-primary">{"*"}</span>
                            </label>
                            <input
                                class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-body-sm text-body-sm rounded-lg"
                                type="text"
                                value={(*app_name).clone()}
                                oninput={
                                    let app_name = app_name.clone();
                                    Callback::from(move |e: InputEvent| {
                                        if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                            app_name.set(target.value());
                                        }
                                    })
                                }
                            />
                        </div>

                        <div class="space-y-1.5">
                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider flex items-center gap-1">
                                {"Application URL "} <span class="text-primary">{"*"}</span>
                            </label>
                            <input
                                class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-body-sm text-body-sm rounded-lg"
                                type="text"
                                value={(*app_url).clone()}
                                oninput={
                                    let app_url = app_url.clone();
                                    Callback::from(move |e: InputEvent| {
                                        if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                            app_url.set(target.value());
                                        }
                                    })
                                }
                            />
                        </div>
                    </div>

                    <div class="space-y-1.5">
                        <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider flex items-center gap-1">
                            {"Admin Contact Email "} <span class="text-primary">{"*"}</span>
                        </label>
                        <input
                            class="w-full bg-surface-container p-3 border border-outline-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-body-sm text-body-sm rounded-lg"
                            type="email"
                            value={(*admin_email).clone()}
                            oninput={
                                let admin_email = admin_email.clone();
                                Callback::from(move |e: InputEvent| {
                                    if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                        admin_email.set(target.value());
                                    }
                                })
                            }
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
                            onclick={on_save}
                            class="bg-primary text-on-primary px-8 py-2.5 font-label-xs text-label-xs font-bold rounded-lg shadow-sm active:scale-[0.98] transition-all hover:bg-on-primary-fixed-variant cursor-pointer"
                        >
                            {"Save changes"}
                        </button>
                    </div>
                </div>
            </div>
        </main>
    }
}
