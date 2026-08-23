use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

/// Reads a single query-param value from the current URL, e.g. `?email=foo@bar.com`.
fn get_query_param(key: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    // search looks like "?email=foo%40bar.com&foo=bar"
    let search = search.trim_start_matches('?');
    for pair in search.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                // Percent-decode the value using the browser's built-in decoder
                let decoded = js_sys::decode_uri_component(v).ok()?;
                return decoded.as_string();
            }
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetStatus {
    Idle,
    Submitting,
    Success,
}

#[function_component(ResetPassword)]
pub fn reset_password() -> Html {
    let navigator = use_navigator();

    // Pre-fill email from ?email=... query param if present
    let email = use_state(|| get_query_param("email").unwrap_or_default());
    let otp = use_state(|| String::new());
    let new_password = use_state(|| String::new());
    let confirm_password = use_state(|| String::new());
    let show_password = use_state(|| false);
    let status = use_state(|| ResetStatus::Idle);
    let error_msg = use_state(|| None::<String>);

    let on_input_email = {
        let email = email.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            email.set(input.value());
        })
    };

    let on_input_otp = {
        let otp = otp.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            // Accept digits only, max 6 chars
            let val: String = input
                .value()
                .chars()
                .filter(|c| c.is_ascii_digit())
                .take(6)
                .collect();
            otp.set(val);
        })
    };

    let on_input_new_password = {
        let new_password = new_password.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            new_password.set(input.value());
        })
    };

    let on_input_confirm_password = {
        let confirm_password = confirm_password.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            confirm_password.set(input.value());
        })
    };

    let on_toggle_show_password = {
        let show_password = show_password.clone();
        Callback::from(move |_| show_password.set(!*show_password))
    };

    let on_back_to_login = {
        let navigator = navigator.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            if let Some(ref nav) = navigator {
                nav.push(&crate::routes::Route::Login);
            }
        })
    };

    let on_submit = {
        let email = email.clone();
        let otp = otp.clone();
        let new_password = new_password.clone();
        let confirm_password = confirm_password.clone();
        let status = status.clone();
        let error_msg = error_msg.clone();
        let navigator = navigator.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if *status != ResetStatus::Idle {
                return;
            }

            let email_val = (*email).clone();
            let otp_val = (*otp).clone();
            let pwd_val = (*new_password).clone();
            let confirm_val = (*confirm_password).clone();

            // Client-side validation
            if email_val.trim().is_empty() {
                error_msg.set(Some("Please enter your email address.".to_string()));
                return;
            }
            if otp_val.len() != 6 {
                error_msg.set(Some(
                    "Please enter the 6-digit OTP from your email.".to_string(),
                ));
                return;
            }
            if pwd_val.len() < 8 {
                error_msg.set(Some("Password must be at least 8 characters.".to_string()));
                return;
            }
            if pwd_val != confirm_val {
                error_msg.set(Some("Passwords do not match.".to_string()));
                return;
            }

            error_msg.set(None);
            status.set(ResetStatus::Submitting);

            // TODO: call the reset-password API once the backend endpoint is ready.
            // wasm_bindgen_futures::spawn_local(async move {
            //     let client = crate::api::client::ApiClient::default();
            //     match client.reset_password("_superusers", &email_val, &otp_val, &pwd_val).await {
            //         Ok(_) => { status_clone.set(ResetStatus::Success); ... }
            //         Err(e) => { status_clone.set(ResetStatus::Idle); error_msg_clone.set(Some(...)); }
            //     }
            // });

            // Placeholder: simulate success and redirect to login
            let status_clone = status.clone();
            let navigator_clone = navigator.clone();
            gloo_timers::callback::Timeout::new(800, move || {
                status_clone.set(ResetStatus::Success);
                gloo_timers::callback::Timeout::new(1500, move || {
                    if let Some(ref nav) = navigator_clone {
                        nav.push(&crate::routes::Route::Login);
                    }
                })
                .forget();
            })
            .forget();
        })
    };

    let password_type = if *show_password { "text" } else { "password" };
    let visibility_icon = if *show_password {
        "visibility_off"
    } else {
        "visibility"
    };

    let button_content = match *status {
        ResetStatus::Idle => html! {
            <>
                <span>{"Reset Password"}</span>
                <span class="material-symbols-outlined text-[20px]">{"lock_reset"}</span>
            </>
        },
        ResetStatus::Submitting => html! {
            <>
                <svg class="animate-spin -ml-1 mr-3 h-5 w-5 text-on-primary-container" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                {"Resetting..."}
            </>
        },
        ResetStatus::Success => html! {
            <>
                <span class="material-symbols-outlined">{"check_circle"}</span>
                {"Password Reset!"}
            </>
        },
    };

    let button_class = match *status {
        ResetStatus::Idle => {
            "w-full bg-primary-container text-on-primary-container h-12 rounded-lg font-headline-md text-headline-md hover:bg-primary transition-all active:scale-[0.98] flex items-center justify-center gap-2 shadow-sm cursor-pointer"
        }
        ResetStatus::Submitting => {
            "w-full bg-primary-container text-on-primary-container h-12 rounded-lg font-headline-md text-headline-md opacity-75 cursor-not-allowed flex items-center justify-center gap-2 shadow-sm"
        }
        ResetStatus::Success => {
            "w-full bg-[#16a34a] text-white h-12 rounded-lg font-headline-md text-headline-md flex items-center justify-center gap-2 shadow-sm transition-colors duration-300"
        }
    };

    let is_busy = *status != ResetStatus::Idle;

    html! {
        <div class="min-h-screen w-full flex flex-col font-body-md text-body-md bg-background pb-20 overflow-x-hidden relative">
            <div class="fixed inset-0 bg-pattern opacity-20 pointer-events-none"></div>

            <main class="flex-grow flex items-center justify-center p-gutter relative z-10">
                <div class="w-full max-w-[440px]">

                    // ── Header ────────────────────────────────────────────────
                    <div class="text-center mb-8">
                        <div class="flex flex-col items-center gap-4">
                            <div class="w-16 h-16 rounded-full bg-primary-container flex items-center justify-center">
                                <span class="material-symbols-outlined text-primary text-[32px]">{"lock_reset"}</span>
                            </div>
                            <div>
                                <h1 class="font-headline-lg text-headline-lg text-primary tracking-tight mb-1">{"Reset Password"}</h1>
                                <p class="font-body-sm text-body-sm text-on-surface-variant max-w-[300px] mx-auto">
                                    {"Enter your email, the 6-digit OTP from your inbox, and your new password."}
                                </p>
                            </div>
                        </div>
                    </div>

                    // ── Card ──────────────────────────────────────────────────
                    <div class="login-card bg-surface-container-lowest rounded-xl p-8 shadow-sm">
                        <form class="space-y-6" onsubmit={on_submit}>

                            // ── Error banner ──────────────────────────────────
                            {
                                if let Some(ref err) = *error_msg {
                                    html! {
                                        <div class="p-3 bg-error-container text-on-error-container rounded-lg text-body-sm flex items-center gap-2 border border-error/20">
                                            <span class="material-symbols-outlined text-[16px]">{"error"}</span>
                                            <span>{err}</span>
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }
                            }

                            // ── Success banner ────────────────────────────────
                            {
                                if *status == ResetStatus::Success {
                                    html! {
                                        <div class="p-3 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 rounded-lg text-body-sm flex items-center gap-2 border border-emerald-500/20">
                                            <span class="material-symbols-outlined text-[16px]">{"check_circle"}</span>
                                            <span>{"Password reset successfully! Redirecting to login..."}</span>
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }
                            }

                            // ── Email field ───────────────────────────────────
                            <div class="space-y-1.5">
                                <label class="font-label-xs text-label-xs uppercase tracking-wider text-on-surface-variant flex items-center gap-2" for="reset-email">
                                    <span class="material-symbols-outlined text-[14px]">{"mail"}</span>
                                    {"Email Address"}
                                </label>
                                <input
                                    class="w-full px-4 py-3 bg-surface border border-outline-variant rounded-lg font-code-md text-code-md transition-all placeholder:text-on-surface-variant/40 focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/20"
                                    id="reset-email"
                                    name="email"
                                    placeholder="you@example.com"
                                    required={true}
                                    type="email"
                                    value={(*email).clone()}
                                    oninput={on_input_email}
                                    disabled={is_busy}
                                />
                            </div>

                            // ── OTP field ─────────────────────────────────────
                            <div class="space-y-1.5">
                                <label class="font-label-xs text-label-xs uppercase tracking-wider text-on-surface-variant flex items-center gap-2" for="otp">
                                    <span class="material-symbols-outlined text-[14px]">{"pin"}</span>
                                    {"One-Time Password (OTP)"}
                                </label>
                                <input
                                    class="w-full px-4 py-3 bg-surface border border-outline-variant rounded-lg font-code-md text-code-md tracking-[0.35em] text-center transition-all placeholder:text-on-surface-variant/40 placeholder:tracking-normal focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/20"
                                    id="otp"
                                    name="otp"
                                    placeholder="— — — — — —"
                                    required={true}
                                    type="text"
                                    inputmode="numeric"
                                    maxlength="6"
                                    value={(*otp).clone()}
                                    oninput={on_input_otp}
                                    disabled={is_busy}
                                />
                                <p class="font-body-sm text-body-sm text-on-surface-variant/60 flex items-center gap-1">
                                    <span class="material-symbols-outlined text-[13px]">{"schedule"}</span>
                                    {"OTP is valid for 5 minutes only."}
                                </p>
                            </div>

                            // ── New password ──────────────────────────────────
                            <div class="space-y-1.5">
                                <label class="font-label-xs text-label-xs uppercase tracking-wider text-on-surface-variant flex items-center gap-2" for="new-password">
                                    <span class="material-symbols-outlined text-[14px]">{"lock"}</span>
                                    {"New Password"}
                                </label>
                                <div class="relative">
                                    <input
                                        class="w-full px-4 py-3 bg-surface border border-outline-variant rounded-lg font-code-md text-code-md transition-all placeholder:text-on-surface-variant/40 focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/20"
                                        id="new-password"
                                        name="new_password"
                                        placeholder="Min. 8 characters"
                                        required={true}
                                        type={password_type}
                                        value={(*new_password).clone()}
                                        oninput={on_input_new_password}
                                        disabled={is_busy}
                                    />
                                    <button
                                        class="absolute right-3 top-1/2 -translate-y-1/2 text-on-surface-variant/60 hover:text-primary transition-colors cursor-pointer"
                                        type="button"
                                        onclick={on_toggle_show_password}
                                        disabled={is_busy}
                                    >
                                        <span class="material-symbols-outlined text-[20px]">{visibility_icon}</span>
                                    </button>
                                </div>
                            </div>

                            // ── Confirm password ──────────────────────────────
                            <div class="space-y-1.5">
                                <label class="font-label-xs text-label-xs uppercase tracking-wider text-on-surface-variant flex items-center gap-2" for="confirm-password">
                                    <span class="material-symbols-outlined text-[14px]">{"lock_open"}</span>
                                    {"Confirm New Password"}
                                </label>
                                <input
                                    class="w-full px-4 py-3 bg-surface border border-outline-variant rounded-lg font-code-md text-code-md transition-all placeholder:text-on-surface-variant/40 focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/20"
                                    id="confirm-password"
                                    name="confirm_password"
                                    placeholder="Repeat new password"
                                    required={true}
                                    type={password_type}
                                    value={(*confirm_password).clone()}
                                    oninput={on_input_confirm_password}
                                    disabled={is_busy}
                                />
                            </div>

                            // ── Submit ────────────────────────────────────────
                            <button
                                class={button_class}
                                type="submit"
                                disabled={is_busy}
                            >
                                {button_content}
                            </button>
                        </form>

                        <div class="text-center mt-6">
                            <a
                                class="font-label-xs text-label-xs text-primary hover:underline inline-flex items-center gap-1 transition-colors cursor-pointer"
                                href="#"
                                onclick={on_back_to_login}
                            >
                                <span class="material-symbols-outlined text-[14px]">{"arrow_back"}</span>
                                {"Back to Login"}
                            </a>
                        </div>
                    </div>

                    <p class="text-center mt-6 font-label-xs text-label-xs text-on-surface-variant/60">
                        {"Didn't receive an email? "}
                        <a class="text-primary font-bold hover:underline" href="/forgot-password">{"Try again"}</a>
                    </p>
                </div>
            </main>

            <footer class="bg-surface-container-lowest border-t border-outline-variant py-4 px-gutter flex flex-col md:flex-row justify-between items-center gap-4 fixed bottom-0 w-full left-0 z-20">
                <div class="flex items-center gap-2">
                    <span class="font-label-xs text-label-xs text-on-surface-variant">{"© 2024 Crabbase Admin • v0.39.0-dev"}</span>
                </div>
                <nav class="flex items-center gap-6">
                    <a class="font-label-xs text-label-xs text-on-surface-variant hover:text-primary underline transition-colors" href="#">{"Documentation"}</a>
                    <a class="font-label-xs text-label-xs text-on-surface-variant hover:text-primary underline transition-colors" href="#">{"GitHub"}</a>
                    <a class="font-label-xs text-label-xs text-on-surface-variant hover:text-primary underline transition-colors" href="#">{"Feedback"}</a>
                </nav>
            </footer>
        </div>
    }
}
