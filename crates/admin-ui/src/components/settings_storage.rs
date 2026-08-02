use yew::prelude::*;

#[function_component(SettingsStorage)]
pub fn settings_storage() -> Html {
    let use_s3 = use_state(|| false);
    let endpoint = use_state(|| String::new());
    let bucket = use_state(|| String::new());
    let region = use_state(|| String::new());
    let access_key = use_state(|| String::new());
    let secret = use_state(|| "••••••••••••••••".to_string());
    let show_secret = use_state(|| false);
    let force_path_style = use_state(|| false);
    let is_saved = use_state(|| false);

    let toggle_s3 = {
        let use_s3 = use_s3.clone();
        Callback::from(move |_| {
            use_s3.set(!*use_s3);
        })
    };

    let toggle_show_secret = {
        let show_secret = show_secret.clone();
        Callback::from(move |_| {
            show_secret.set(!*show_secret);
        })
    };

    let toggle_path_style = {
        let force_path_style = force_path_style.clone();
        Callback::from(move |_| {
            force_path_style.set(!*force_path_style);
        })
    };

    let on_save = {
        let is_saved = is_saved.clone();
        Callback::from(move |_| {
            is_saved.set(true);
        })
    };

    let on_cancel = {
        let use_s3 = use_s3.clone();
        let endpoint = endpoint.clone();
        let bucket = bucket.clone();
        let region = region.clone();
        let access_key = access_key.clone();
        let secret = secret.clone();
        let force_path_style = force_path_style.clone();
        let is_saved = is_saved.clone();
        Callback::from(move |_| {
            use_s3.set(false);
            endpoint.set(String::new());
            bucket.set(String::new());
            region.set(String::new());
            access_key.set(String::new());
            secret.set("••••••••••••••••".to_string());
            force_path_style.set(false);
            is_saved.set(false);
        })
    };

    html! {
        <main class="flex-1 overflow-y-auto p-margin_page bg-[#F9F8F7]">
            <div class="max-w-4xl mx-auto space-y-8">
                /* Breadcrumbs & Header */
                <div class="space-y-1">
                    <nav class="flex items-center gap-2 text-on-surface-variant font-label-xs text-label-xs mb-2">
                        <span>{"Settings"}</span>
                        <span class="material-symbols-outlined scale-75">{"chevron_right"}</span>
                        <span class="text-primary font-bold">{"File storage"}</span>
                    </nav>
                    <h1 class="font-headline-lg text-headline-lg text-on-surface">{"Storage Management"}</h1>
                </div>

                {
                    if *is_saved {
                        html! {
                            <div class="p-4 bg-primary-container/10 border border-primary-container/30 text-primary rounded-lg flex items-center justify-between transition-all">
                                <div class="flex items-center gap-2">
                                    <span class="material-symbols-outlined">{"check_circle"}</span>
                                    <span class="font-body-sm text-body-sm font-bold">{"Storage settings saved successfully!"}</span>
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

                /* Info Card */
                <div class="bg-surface-container-low border border-outline-variant p-6 space-y-4 rounded-lg">
                    <div class="space-y-3 text-on-surface-variant font-body-sm text-body-sm leading-relaxed">
                        <p>
                            {"By default, "}
                            <span class="font-bold text-primary">{"Crabbase"}</span>
                            {" uses and recommends the local file system to store uploaded files because it is more performant, easier to manage and backup."}
                        </p>
                        <p>
                            {"Alternatively, if you have limited disk space available, you could opt to an S3 compatible external storage provider (AWS, DigitalOcean Spaces, MinIO)."}
                        </p>
                    </div>

                    <div class="flex items-center gap-3 py-2 group">
                        <button
                            type="button"
                            onclick={toggle_s3}
                            class={classes!(
                                "relative", "inline-flex", "h-6", "w-11", "shrink-0", "cursor-pointer",
                                "rounded-full", "border-2", "border-transparent", "transition-colors",
                                "duration-200", "ease-in-out", "focus:outline-none",
                                if *use_s3 { "bg-primary" } else { "bg-outline-variant" }
                            )}
                        >
                            <span
                                class={classes!(
                                    "pointer-events-none", "inline-block", "h-5", "w-5", "transform",
                                    "rounded-full", "bg-white", "shadow", "ring-0", "transition",
                                    "duration-200", "ease-in-out",
                                    if *use_s3 { "translate-x-5" } else { "translate-x-0" }
                                )}
                            />
                        </button>
                        <span onclick={
                            let use_s3 = use_s3.clone();
                            move |_| use_s3.set(!*use_s3)
                        } class="font-bold text-on-surface group-hover:text-primary transition-colors cursor-pointer">
                            {"Use S3 storage"}
                        </span>
                    </div>

                    /* Migration Warning Alert */
                    {
                        if *use_s3 {
                            html! {
                                <div class="flex gap-3 p-4 bg-surface-container-highest border-l-4 border-primary text-on-surface-variant font-body-sm text-body-sm rounded-r-md">
                                    <span class="material-symbols-outlined text-primary">{"info"}</span>
                                    <div>
                                        {"If you have existing uploaded files, you'll have to migrate them manually from the "}
                                        <span class="font-bold">{"local file system"}</span>
                                        {" to the "}
                                        <span class="font-bold">{"S3 storage"}</span>
                                        {". There are several command line tools that can help you, such as: "}
                                        <code class="font-code-md text-primary bg-surface p-0.5 rounded">{"rclone"}</code>
                                        {", "}
                                        <code class="font-code-md text-primary bg-surface p-0.5 rounded">{"s5cmd"}</code>
                                        {", etc."}
                                    </div>
                                </div>
                            }
                        } else {
                            html! {}
                        }
                    }
                </div>

                /* S3 Config Form Section */
                <div class={classes!(
                    "space-y-6", "transition-all", "duration-300",
                    if *use_s3 { "opacity-100" } else { "opacity-40 pointer-events-none" }
                )}>
                    <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
                        <div class="space-y-2">
                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-tight flex items-center gap-1">
                                {"Endpoint "} <span class="text-primary">{"*"}</span>
                            </label>
                            <input
                                class="w-full bg-surface-container-low border border-outline-variant px-cell_padding_h py-cell_padding_v font-code-md text-code-md focus:ring-2 focus:ring-primary focus:border-primary transition-all rounded"
                                placeholder="e.g. s3.amazonaws.com"
                                type="text"
                                value={(*endpoint).clone()}
                                disabled={!*use_s3}
                                oninput={
                                    let endpoint = endpoint.clone();
                                    Callback::from(move |e: InputEvent| {
                                        if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                            endpoint.set(target.value());
                                        }
                                    })
                                }
                            />
                        </div>
                        <div class="space-y-2">
                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-tight flex items-center gap-1">
                                {"Bucket "} <span class="text-primary">{"*"}</span>
                            </label>
                            <input
                                class="w-full bg-surface-container-low border border-outline-variant px-cell_padding_h py-cell_padding_v font-code-md text-code-md focus:ring-2 focus:ring-primary focus:border-primary transition-all rounded"
                                placeholder="my-storage-bucket"
                                type="text"
                                value={(*bucket).clone()}
                                disabled={!*use_s3}
                                oninput={
                                    let bucket = bucket.clone();
                                    Callback::from(move |e: InputEvent| {
                                        if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                            bucket.set(target.value());
                                        }
                                    })
                                }
                            />
                        </div>
                        <div class="space-y-2">
                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-tight flex items-center gap-1">
                                {"Region "} <span class="text-primary">{"*"}</span>
                            </label>
                            <input
                                class="w-full bg-surface-container-low border border-outline-variant px-cell_padding_h py-cell_padding_v font-code-md text-code-md focus:ring-2 focus:ring-primary focus:border-primary transition-all rounded"
                                placeholder="us-east-1"
                                type="text"
                                value={(*region).clone()}
                                disabled={!*use_s3}
                                oninput={
                                    let region = region.clone();
                                    Callback::from(move |e: InputEvent| {
                                        if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                            region.set(target.value());
                                        }
                                    })
                                }
                            />
                        </div>
                    </div>

                    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                        <div class="space-y-2">
                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-tight flex items-center gap-1">
                                {"Access key "} <span class="text-primary">{"*"}</span>
                            </label>
                            <input
                                class="w-full bg-surface-container-low border border-outline-variant px-cell_padding_h py-cell_padding_v font-code-md text-code-md focus:ring-2 focus:ring-primary focus:border-primary transition-all rounded"
                                placeholder="AKIA..."
                                type="text"
                                value={(*access_key).clone()}
                                disabled={!*use_s3}
                                oninput={
                                    let access_key = access_key.clone();
                                    Callback::from(move |e: InputEvent| {
                                        if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                            access_key.set(target.value());
                                        }
                                    })
                                }
                            />
                        </div>
                        <div class="space-y-2 relative">
                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-tight flex items-center gap-1">
                                {"Secret "} <span class="text-primary">{"*"}</span>
                            </label>
                            <div class="relative">
                                <input
                                    class="w-full bg-surface-container-low border border-outline-variant px-cell_padding_h py-cell_padding_v font-code-md text-code-md focus:ring-2 focus:ring-primary focus:border-primary transition-all rounded pr-10"
                                    type={if *show_secret { "text" } else { "password" }}
                                    value={(*secret).clone()}
                                    disabled={!*use_s3}
                                    oninput={
                                        let secret = secret.clone();
                                        Callback::from(move |e: InputEvent| {
                                            if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                secret.set(target.value());
                                            }
                                        })
                                    }
                                />
                                <button
                                    type="button"
                                    onclick={toggle_show_secret}
                                    class="absolute right-3 top-1/2 -translate-y-1/2 text-on-surface-variant hover:text-primary cursor-pointer"
                                >
                                    <span class="material-symbols-outlined">{if *show_secret { "visibility_off" } else { "visibility" }}</span>
                                </button>
                            </div>
                        </div>
                    </div>

                    <div class="flex items-center gap-3 pt-2">
                        <input
                            type="checkbox"
                            id="path-style"
                            checked={*force_path_style}
                            disabled={!*use_s3}
                            onclick={toggle_path_style}
                            class="w-4 h-4 text-primary border-outline focus:ring-primary rounded cursor-pointer"
                        />
                        <label class="font-body-sm text-body-sm text-on-surface-variant cursor-pointer flex items-center gap-1" for="path-style">
                            {"Force path-style addressing"}
                            <span class="material-symbols-outlined scale-75 text-outline" title="Useful for local development with MinIO">{"help_outline"}</span>
                        </label>
                    </div>
                </div>

                /* Footer Actions */
                <div class="pt-8 border-t border-outline-variant flex justify-end items-center gap-4">
                    <button
                        type="button"
                        onclick={on_cancel}
                        class="px-6 py-2 text-on-surface-variant hover:bg-surface-container transition-all font-bold rounded cursor-pointer"
                    >
                        {"Cancel"}
                    </button>
                    <button
                        type="button"
                        onclick={on_save}
                        class="px-8 py-2.5 bg-primary text-on-primary font-bold rounded shadow-sm hover:shadow-md active:scale-95 transition-all cursor-pointer"
                    >
                        {"Save changes"}
                    </button>
                </div>
            </div>
        </main>
    }
}
