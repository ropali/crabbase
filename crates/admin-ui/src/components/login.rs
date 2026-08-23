use gloo_timers::callback::Timeout;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    Login,
    ForgotPassword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginStatus {
    Idle,
    Authenticating,
    Success,
}

#[derive(Properties, PartialEq, Clone)]
pub struct LoginProps {
    pub on_login_success: Callback<()>,
}

#[function_component(Login)]
pub fn login(props: &LoginProps) -> Html {
    let navigator = use_navigator();
    let route = use_route::<crate::routes::Route>();
    let initial_mode = match route {
        Some(crate::routes::Route::ForgotPassword) => AuthMode::ForgotPassword,
        _ => AuthMode::Login,
    };

    let mode = use_state(|| initial_mode);
    #[cfg(debug_assertions)]
    let email = use_state(|| "admin@crabbase.local".to_string());
    #[cfg(not(debug_assertions))]
    let email = use_state(|| String::new());

    #[cfg(debug_assertions)]
    let password = use_state(|| "admin123".to_string());
    #[cfg(not(debug_assertions))]
    let password = use_state(|| String::new());
    let show_password = use_state(|| false);
    let status = use_state(|| LoginStatus::Idle);
    let error_msg = use_state(|| None::<String>);
    let success_msg = use_state(|| None::<String>);

    let on_switch_mode = {
        let mode = mode.clone();
        let status = status.clone();
        let error_msg = error_msg.clone();
        let success_msg = success_msg.clone();
        let navigator = navigator.clone();
        Callback::from(move |target_mode: AuthMode| {
            mode.set(target_mode);
            status.set(LoginStatus::Idle);
            error_msg.set(None);
            success_msg.set(None);
            if let Some(ref nav) = navigator {
                match target_mode {
                    AuthMode::Login => nav.push(&crate::routes::Route::Login),
                    AuthMode::ForgotPassword => nav.push(&crate::routes::Route::ForgotPassword),
                }
            }
        })
    };

    let on_click_forgot = {
        let on_switch_mode = on_switch_mode.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            on_switch_mode.emit(AuthMode::ForgotPassword);
        })
    };

    let on_click_login = {
        let on_switch_mode = on_switch_mode.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            on_switch_mode.emit(AuthMode::Login);
        })
    };

    let on_input_email = {
        let email = email.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            email.set(input.value());
        })
    };

    let on_input_password = {
        let password = password.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            password.set(input.value());
        })
    };

    let on_toggle_show_password = {
        let show_password = show_password.clone();
        Callback::from(move |_| {
            show_password.set(!*show_password);
        })
    };

    let on_submit = {
        let mode = mode.clone();
        let status = status.clone();
        let error_msg = error_msg.clone();
        let success_msg = success_msg.clone();
        let email = email.clone();
        let password = password.clone();
        let on_login_success = props.on_login_success.clone();
        let navigator = navigator.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if *status != LoginStatus::Idle {
                return;
            }

            status.set(LoginStatus::Authenticating);
            error_msg.set(None);
            success_msg.set(None);

            let status_clone = status.clone();
            let error_msg_clone = error_msg.clone();
            let email_val = (*email).clone();
            let password_val = (*password).clone();
            let on_login_success_clone = on_login_success.clone();
            let navigator_clone = navigator.clone();
            let current_mode = *mode;

            wasm_bindgen_futures::spawn_local(async move {
                let client = crate::api::client::ApiClient::default();
                match current_mode {
                    AuthMode::Login => {
                        match client.login("_superusers", &email_val, &password_val).await {
                            Ok(_) => {
                                status_clone.set(LoginStatus::Success);
                                let timeout = Timeout::new(800, move || {
                                    if let Some(ref nav) = navigator_clone {
                                        nav.push(&crate::routes::Route::Home);
                                    }
                                    on_login_success_clone.emit(());
                                });
                                timeout.forget();
                            }
                            Err(e) => {
                                status_clone.set(LoginStatus::Idle);
                                let err_text = match e {
                                    gloo_net::Error::GlooError(msg) => msg,
                                    _ => format!("{}", e),
                                };
                                let user_friendly_msg = if err_text.contains("HTTP Status 401")
                                    || err_text.contains("Unauthorized")
                                {
                                    "Invalid email or password. Please try again.".to_string()
                                } else {
                                    format!("Authentication failed: {}", err_text)
                                };
                                error_msg_clone.set(Some(user_friendly_msg));
                            }
                        }
                    }
                    AuthMode::ForgotPassword => {
                        match client.forget_password("_superusers", &email_val).await {
                            Ok(_) => {
                                status_clone.set(LoginStatus::Success);
                                // Navigate to the OTP + new-password screen
                                gloo_timers::callback::Timeout::new(600, move || {
                                    if let Some(ref nav) = navigator_clone {
                                        // Pass email as query param so the reset screen can pre-fill it
                                        let _ = nav.push_with_query(
                                            &crate::routes::Route::ResetPassword,
                                            &[("email", email_val.as_str())],
                                        );
                                    }
                                })
                                .forget();
                            }
                            Err(e) => {
                                status_clone.set(LoginStatus::Idle);
                                let err_text = match e {
                                    gloo_net::Error::GlooError(msg) => msg,
                                    _ => format!("{}", e),
                                };
                                error_msg_clone
                                    .set(Some(format!("Failed to send reset link: {}", err_text)));
                            }
                        }
                    }
                }
            });
        })
    };

    // Styling logic
    let password_type = if *show_password { "text" } else { "password" };
    let visibility_icon = if *show_password {
        "visibility_off"
    } else {
        "visibility"
    };

    let button_content = match (*mode, *status) {
        (AuthMode::Login, LoginStatus::Idle) => html! {
            <>
                <span>{"Login"}</span>
                <span class="material-symbols-outlined text-[20px]">{"arrow_forward"}</span>
            </>
        },
        (AuthMode::Login, LoginStatus::Authenticating) => html! {
            <>
                <svg class="animate-spin -ml-1 mr-3 h-5 w-5 text-on-primary-container" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                {"Authenticating..."}
            </>
        },
        (AuthMode::Login, LoginStatus::Success) => html! {
            <>
                <span class="material-symbols-outlined">{"check_circle"}</span>
                {"Success"}
            </>
        },
        (AuthMode::ForgotPassword, LoginStatus::Idle) => html! {
            <>
                <span>{"Send Reset Link"}</span>
                <span class="material-symbols-outlined text-[20px]">{"send"}</span>
            </>
        },
        (AuthMode::ForgotPassword, LoginStatus::Authenticating) => html! {
            <>
                <svg class="animate-spin -ml-1 mr-3 h-5 w-5 text-on-primary-container" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                {"Sending..."}
            </>
        },
        (AuthMode::ForgotPassword, LoginStatus::Success) => html! {
            <>
                <span class="material-symbols-outlined">{"check_circle"}</span>
                {"Reset Link Sent"}
            </>
        },
    };

    let button_class = match *status {
        LoginStatus::Idle => {
            "w-full bg-primary-container text-on-primary-container h-12 rounded-lg font-headline-md text-headline-md hover:bg-primary transition-all active:scale-[0.98] flex items-center justify-center gap-2 shadow-sm cursor-pointer"
        }
        LoginStatus::Authenticating => {
            "w-full bg-primary-container text-on-primary-container h-12 rounded-lg font-headline-md text-headline-md opacity-75 cursor-not-allowed flex items-center justify-center gap-2 shadow-sm"
        }
        LoginStatus::Success => {
            "w-full bg-[#16a34a] text-white h-12 rounded-lg font-headline-md text-headline-md flex items-center justify-center gap-2 shadow-sm transition-colors duration-300"
        }
    };

    let subtitle = match *mode {
        AuthMode::Login => {
            "Welcome back. Please enter your credentials to access the management schema."
        }
        AuthMode::ForgotPassword => {
            "Enter your email address below and we will send you a password reset link."
        }
    };

    html! {
        <div class="min-h-screen w-full flex flex-col font-body-md text-body-md bg-background pb-20 overflow-x-hidden relative">
            <div class="fixed inset-0 bg-pattern opacity-20 pointer-events-none"></div>

            <main class="flex-grow flex items-center justify-center p-gutter relative z-10">
                <div class="w-full max-w-[440px]">
                    <div class="text-center mb-8 relative">
                        <div class="flex flex-col items-center gap-4">
                            <div class="w-full h-24 relative overflow-hidden flex items-center justify-center">
                                <div class="w-32 h-24">
                                    <div class="w-full h-full" id="animated-svg-ANIMATION_32" style="display:block;">
                                        <svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
                                            <style>
                                                {r#"
                                                    @keyframes scuttle-back-and-forth {
                                                        0%, 100% { transform: translateX(-40px) rotate(-2deg); }
                                                        50% { transform: translateX(40px) rotate(2deg); }
                                                    }
                                                    @keyframes wave-claw {
                                                        0%, 100% { transform: rotate(0deg); }
                                                        50% { transform: rotate(-15deg); }
                                                    }
                                                    @keyframes blink {
                                                        0%, 90%, 100% { transform: scaleY(1); }
                                                        95% { transform: scaleY(0.1); }
                                                    }
                                                    .crab-body { fill: #ce412b; }
                                                    .crab-eye { fill: white; }
                                                    .crab-pupil { fill: black; }
                                                    .crab-leg { fill: #ce412b; transform-origin: center; }
                                                    .animate-crab {
                                                        animation: scuttle-back-and-forth 6s ease-in-out infinite;
                                                        transform-origin: center;
                                                    }
                                                    .claw-l { animation: wave-claw 2s ease-in-out infinite; transform-origin: 30px 35px; }
                                                    .eye-blink { animation: blink 4s infinite; transform-origin: center; }
                                                "#}
                                            </style>
                                            <g class="animate-crab">
                                                <path class="crab-leg" d="M20,50 L5,40 M20,60 L5,70 M20,70 L10,85" stroke="#ce412b" stroke-linecap="round" stroke-width="4"></path>
                                                <path class="crab-leg" d="M80,50 L95,40 M80,60 L95,70 M80,70 L90,85" stroke="#ce412b" stroke-linecap="round" stroke-width="4"></path>
                                                <path class="crab-body claw-l" d="M30,30 Q20,10 10,30 Q20,40 30,35 Z"></path>
                                                <path class="crab-body" d="M70,30 Q80,10 90,30 Q80,40 70,35 Z"></path>
                                                <ellipse class="crab-body" cx="50" cy="55" rx="35" ry="25"></ellipse>
                                                <g class="eye-blink">
                                                    <circle class="crab-eye" cx="40" cy="35" r="5"></circle>
                                                    <circle class="crab-pupil" cx="40" cy="35" r="2"></circle>
                                                    <circle class="crab-eye" cx="60" cy="35" r="5"></circle>
                                                    <circle class="crab-pupil" cx="60" cy="35" r="2"></circle>
                                                </g>
                                            </g>
                                        </svg>
                                    </div>
                                </div>
                            </div>
                            <div>
                                <h1 class="font-headline-lg text-headline-lg text-primary tracking-tight mb-1">{"Crabbase"}</h1>
                                <p class="font-body-sm text-body-sm text-on-surface-variant max-w-[280px] mx-auto">
                                    {subtitle}
                                </p>
                            </div>
                        </div>
                    </div>
                    <div class="login-card bg-surface-container-lowest rounded-xl p-8 shadow-sm">
                        <form class="space-y-6" id="login-form" onsubmit={on_submit}>
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
                            {
                                if let Some(ref msg) = *success_msg {
                                    html! {
                                        <div class="p-3 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 rounded-lg text-body-sm flex items-center gap-2 border border-emerald-500/20">
                                            <span class="material-symbols-outlined text-[16px]">{"check_circle"}</span>
                                            <span>{msg}</span>
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }
                            }
                            <div class="space-y-1.5">
                                <label class="font-label-xs text-label-xs uppercase tracking-wider text-on-surface-variant flex items-center gap-2" for="email">
                                    <span class="material-symbols-outlined text-[14px]">{"mail"}</span>
                                    {"Email Address"}
                                </label>
                                <input
                                    class="w-full px-4 py-3 bg-surface border border-outline-variant rounded-lg font-code-md text-code-md transition-all placeholder:text-on-surface-variant/40 focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/20"
                                    id="email"
                                    name="email"
                                    placeholder="admin@crabbase.io"
                                    required=true
                                    type="email"
                                    value={(*email).clone()}
                                    oninput={on_input_email}
                                    disabled={*status != LoginStatus::Idle}
                                />
                            </div>

                            {
                                if *mode == AuthMode::Login {
                                    html! {
                                        <div class="space-y-1.5">
                                            <div class="flex justify-between items-center">
                                                <label class="font-label-xs text-label-xs uppercase tracking-wider text-on-surface-variant flex items-center gap-2" for="password">
                                                    <span class="material-symbols-outlined text-[14px]">{"lock"}</span>
                                                    {"Password"}
                                                </label>
                                                <a class="font-label-xs text-label-xs text-primary hover:underline transition-colors cursor-pointer" href="#" onclick={on_click_forgot}>
                                                    {"Forgot password?"}
                                                </a>
                                            </div>
                                            <div class="relative">
                                                <input
                                                    class="w-full px-4 py-3 bg-surface border border-outline-variant rounded-lg font-code-md text-code-md transition-all placeholder:text-on-surface-variant/40 focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/20"
                                                    id="password"
                                                    name="password"
                                                    placeholder="••••••••••••"
                                                    required=true
                                                    type={password_type}
                                                    value={(*password).clone()}
                                                    oninput={on_input_password}
                                                    disabled={*status != LoginStatus::Idle}
                                                />
                                                <button
                                                    class="absolute right-3 top-1/2 -translate-y-1/2 text-on-surface-variant/60 hover:text-primary transition-colors cursor-pointer"
                                                    type="button"
                                                    onclick={on_toggle_show_password}
                                                    disabled={*status != LoginStatus::Idle}
                                                >
                                                    <span class="material-symbols-outlined text-[20px]">{visibility_icon}</span>
                                                </button>
                                            </div>
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }
                            }

                            <button
                                class={button_class}
                                type="submit"
                                disabled={*status != LoginStatus::Idle}
                            >
                                {button_content}
                            </button>
                        </form>

                        {
                            if *mode == AuthMode::ForgotPassword {
                                html! {
                                    <div class="text-center mt-6">
                                        <a class="font-label-xs text-label-xs text-primary hover:underline inline-flex items-center gap-1 transition-colors cursor-pointer" href="#" onclick={on_click_login}>
                                            <span class="material-symbols-outlined text-[14px]">{"arrow_back"}</span>
                                            {"Back to Login"}
                                        </a>
                                    </div>
                                }
                            } else {
                                html! {}
                            }
                        }
                    </div>
                    <p class="text-center mt-6 font-label-xs text-label-xs text-on-surface-variant/60">
                        {"New to Crabbase? "}
                        <a class="text-primary font-bold hover:underline" href="#">{"Contact System Admin"}</a>
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
