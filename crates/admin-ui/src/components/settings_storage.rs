use crate::api::client::ApiClient;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SettingsStorageProps {
    #[prop_or_default]
    pub on_save: Option<Callback<()>>,
}

#[derive(Clone, PartialEq, Default)]
struct StorageState {
    storage_type: String,
    local_path: String,
    endpoint: String,
    bucket: String,
    region: String,
    access_key: String,
    secret: String,
    force_path_style: bool,
}

#[function_component(SettingsStorage)]
pub fn settings_storage(props: &SettingsStorageProps) -> Html {
    let storage_type = use_state(|| "local".to_string());
    let local_path = use_state(|| "$HOME/crabbase/files/".to_string());
    let endpoint = use_state(|| String::new());
    let bucket = use_state(|| "crabbase_files".to_string());
    let region = use_state(|| "us-east-1".to_string());
    let access_key = use_state(|| String::new());
    let secret = use_state(|| String::new());
    let show_secret = use_state(|| false);
    let force_path_style = use_state(|| false);

    let is_loading = use_state(|| true);
    let is_saving = use_state(|| false);
    let success_msg = use_state(|| Option::<String>::None);
    let error_msg = use_state(|| Option::<String>::None);

    let initial_state = use_state(|| StorageState::default());

    // Fetch storage settings on mount
    {
        let storage_type = storage_type.clone();
        let local_path = local_path.clone();
        let endpoint = endpoint.clone();
        let bucket = bucket.clone();
        let region = region.clone();
        let access_key = access_key.clone();
        let secret = secret.clone();
        let force_path_style = force_path_style.clone();
        let is_loading = is_loading.clone();
        let error_msg = error_msg.clone();
        let initial_state = initial_state.clone();

        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let client = ApiClient::default();
                match client.get_storage_settings().await {
                    Ok(Some(data)) => {
                        let st = data
                            .get("storageType")
                            .or_else(|| data.get("storage_type"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("local")
                            .to_lowercase();
                        storage_type.set(st.clone());

                        let mut lp = "$HOME/crabbase/files/".to_string();
                        if let Some(local) = data.get("local") {
                            if let Some(p) = local
                                .get("local_path")
                                .or_else(|| local.get("localPath"))
                                .or_else(|| local.get("base_path"))
                                .or_else(|| local.get("basePath"))
                                .and_then(|v| v.as_str())
                            {
                                lp = p.to_string();
                            }
                        }
                        local_path.set(lp.clone());

                        let mut ep = String::new();
                        let mut bk = "crabbase_files".to_string();
                        let mut rg = "us-east-1".to_string();
                        let mut ak = String::new();
                        let mut sc = String::new();
                        let mut fps = false;

                        if let Some(s3) = data.get("s3") {
                            if let Some(v) = s3
                                .get("endpoint")
                                .or_else(|| s3.get("endpoint_url"))
                                .or_else(|| s3.get("endpointUrl"))
                                .and_then(|v| v.as_str())
                            {
                                ep = v.to_string();
                            }
                            if let Some(v) = s3.get("bucket").and_then(|v| v.as_str()) {
                                bk = v.to_string();
                            }
                            if let Some(v) = s3.get("region").and_then(|v| v.as_str()) {
                                rg = v.to_string();
                            }
                            if let Some(v) = s3
                                .get("accessKey")
                                .or_else(|| s3.get("access_key"))
                                .or_else(|| s3.get("accessKeyId"))
                                .or_else(|| s3.get("access_key_id"))
                                .and_then(|v| v.as_str())
                            {
                                ak = v.to_string();
                            }
                            if let Some(v) = s3
                                .get("secret")
                                .or_else(|| s3.get("secret_access_key"))
                                .or_else(|| s3.get("secretAccessKey"))
                                .and_then(|v| v.as_str())
                            {
                                sc = v.to_string();
                            }
                            if let Some(v) = s3
                                .get("forcePathStyle")
                                .or_else(|| s3.get("force_path_style"))
                                .and_then(|v| v.as_bool())
                            {
                                fps = v;
                            }
                        }

                        endpoint.set(ep.clone());
                        bucket.set(bk.clone());
                        region.set(rg.clone());
                        access_key.set(ak.clone());
                        secret.set(sc.clone());
                        force_path_style.set(fps);

                        initial_state.set(StorageState {
                            storage_type: st,
                            local_path: lp,
                            endpoint: ep,
                            bucket: bk,
                            region: rg,
                            access_key: ak,
                            secret: sc,
                            force_path_style: fps,
                        });
                    }
                    Ok(None) => {
                        initial_state.set(StorageState {
                            storage_type: (*storage_type).clone(),
                            local_path: (*local_path).clone(),
                            endpoint: (*endpoint).clone(),
                            bucket: (*bucket).clone(),
                            region: (*region).clone(),
                            access_key: (*access_key).clone(),
                            secret: (*secret).clone(),
                            force_path_style: *force_path_style,
                        });
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to load storage settings: {e}")));
                    }
                }
                is_loading.set(false);
            });
            || ()
        });
    }

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
        let storage_type = storage_type.clone();
        let local_path = local_path.clone();
        let endpoint = endpoint.clone();
        let bucket = bucket.clone();
        let region = region.clone();
        let access_key = access_key.clone();
        let secret = secret.clone();
        let force_path_style = force_path_style.clone();
        let is_saving = is_saving.clone();
        let success_msg = success_msg.clone();
        let error_msg = error_msg.clone();
        let initial_state = initial_state.clone();
        let on_save_cb = props.on_save.clone();

        Callback::from(move |_| {
            let st = (*storage_type).clone();
            let lp = (*local_path).clone();
            let ep = (*endpoint).trim().to_string();
            let bk = (*bucket).trim().to_string();
            let rg = (*region).trim().to_string();
            let ak = (*access_key).trim().to_string();
            let sc = (*secret).trim().to_string();
            let fps = *force_path_style;

            if st == "s3" && bk.is_empty() {
                error_msg.set(Some(
                    "Bucket name is required for S3 storage configuration.".to_string(),
                ));
                return;
            }

            is_saving.set(true);
            error_msg.set(None);
            success_msg.set(None);

            let is_saving = is_saving.clone();
            let success_msg = success_msg.clone();
            let error_msg = error_msg.clone();
            let initial_state = initial_state.clone();
            let on_save_cb = on_save_cb.clone();

            let payload_st = st.clone();
            let payload_lp = lp.clone();
            let payload_ep = ep.clone();
            let payload_bk = bk.clone();
            let payload_rg = rg.clone();
            let payload_ak = ak.clone();
            let payload_sc = sc.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let body = serde_json::json!({
                    "storageType": payload_st,
                    "local": {
                        "local_path": payload_lp
                    },
                    "s3": {
                        "endpoint": payload_ep,
                        "bucket": payload_bk,
                        "region": payload_rg,
                        "accessKey": payload_ak,
                        "secret": payload_sc,
                        "forcePathStyle": fps
                    }
                });

                let client = ApiClient::default();
                match client.save_storage_settings(body).await {
                    Ok(_) => {
                        success_msg.set(Some("Storage settings saved successfully!".to_string()));
                        initial_state.set(StorageState {
                            storage_type: payload_st,
                            local_path: payload_lp,
                            endpoint: payload_ep,
                            bucket: payload_bk,
                            region: payload_rg,
                            access_key: payload_ak,
                            secret: payload_sc,
                            force_path_style: fps,
                        });
                        if let Some(ref cb) = on_save_cb {
                            cb.emit(());
                        }
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to save storage settings: {e}")));
                    }
                }
                is_saving.set(false);
            });
        })
    };

    let on_cancel = {
        let storage_type = storage_type.clone();
        let local_path = local_path.clone();
        let endpoint = endpoint.clone();
        let bucket = bucket.clone();
        let region = region.clone();
        let access_key = access_key.clone();
        let secret = secret.clone();
        let force_path_style = force_path_style.clone();
        let initial_state = initial_state.clone();
        let success_msg = success_msg.clone();
        let error_msg = error_msg.clone();

        Callback::from(move |_| {
            storage_type.set(initial_state.storage_type.clone());
            local_path.set(initial_state.local_path.clone());
            endpoint.set(initial_state.endpoint.clone());
            bucket.set(initial_state.bucket.clone());
            region.set(initial_state.region.clone());
            access_key.set(initial_state.access_key.clone());
            secret.set(initial_state.secret.clone());
            force_path_style.set(initial_state.force_path_style);
            success_msg.set(None);
            error_msg.set(None);
        })
    };

    let is_s3 = *storage_type == "s3";

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

                /* Success Alert */
                {
                    if let Some(msg) = (*success_msg).as_ref() {
                        html! {
                            <div class="p-4 bg-primary-container/10 border border-primary-container/30 text-primary rounded-lg flex items-center justify-between transition-all">
                                <div class="flex items-center gap-2">
                                    <span class="material-symbols-outlined">{"check_circle"}</span>
                                    <span class="font-body-sm text-body-sm font-bold">{msg}</span>
                                </div>
                                <button onclick={
                                    let success_msg = success_msg.clone();
                                    move |_| success_msg.set(None)
                                } class="text-primary hover:opacity-80">
                                    <span class="material-symbols-outlined text-sm">{"close"}</span>
                                </button>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }

                /* Error Alert */
                {
                    if let Some(err) = (*error_msg).as_ref() {
                        html! {
                            <div class="p-4 bg-error-container/10 border border-error-container/30 text-error rounded-lg flex items-center justify-between transition-all">
                                <div class="flex items-center gap-2">
                                    <span class="material-symbols-outlined">{"error"}</span>
                                    <span class="font-body-sm text-body-sm font-bold">{err}</span>
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

                /* Loading skeleton */
                {
                    if *is_loading {
                        html! {
                            <div class="p-8 text-center text-on-surface-variant flex flex-col items-center gap-3">
                                <span class="material-symbols-outlined animate-spin text-3xl text-primary">{"progress_activity"}</span>
                                <span class="font-body-sm text-body-sm">{"Loading storage configuration..."}</span>
                            </div>
                        }
                    } else {
                        html! {
                            <>
                                /* Storage Provider Card with Dropdown */
                                <div class="bg-surface-container-low border border-outline-variant p-6 space-y-6 rounded-lg">
                                    <div class="space-y-3 text-on-surface-variant font-body-sm text-body-sm leading-relaxed">
                                        <p>
                                            {"By default, "}
                                            <span class="font-bold text-primary">{"Crabbase"}</span>
                                            {" uses the local file system to store uploaded files. Use S3 as it is recommended."}
                                        </p>
                                    </div>

                                    /* Storage Provider Selector */
                                    <div class="space-y-2 max-w-md">
                                        <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-tight flex items-center gap-1 font-semibold">
                                            {"Storage Provider"} <span class="text-primary">{"*"}</span>
                                        </label>
                                        <div class="relative">
                                            <select
                                                class="w-full bg-surface border border-outline-variant px-cell_padding_h py-2.5 font-body-sm text-body-sm text-on-surface focus:ring-2 focus:ring-primary focus:border-primary transition-all rounded cursor-pointer appearance-none pr-10"
                                                value={(*storage_type).clone()}
                                                onchange={
                                                    let storage_type = storage_type.clone();
                                                    Callback::from(move |e: Event| {
                                                        if let Some(target) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                                                            storage_type.set(target.value());
                                                        }
                                                    })
                                                }
                                            >
                                                <option value="local" selected={!is_s3}>{"Local filesystem"}</option>
                                                <option value="s3" selected={is_s3}>{"S3 compatible storage"}</option>
                                            </select>
                                            <div class="pointer-events-none absolute inset-y-0 right-0 flex items-center px-3 text-on-surface-variant">
                                                <span class="material-symbols-outlined text-sm">{"unfold_more"}</span>
                                            </div>
                                        </div>
                                        <p class="font-body-xs text-body-xs text-on-surface-variant">
                                            {if is_s3 {
                                                "Binary payloads will be stored directly in the specified S3 bucket."
                                            } else {
                                                "Binary payloads will be stored on the local disk of the server instance."
                                            }}
                                        </p>
                                    </div>
                                </div>

                                /* Local Filesystem Configuration (when Local is selected) */
                                {
                                    if !is_s3 {
                                        html! {
                                            <div class="bg-surface-container-low border border-outline-variant p-6 space-y-4 rounded-lg">
                                                <div class="flex items-center gap-2">
                                                    <span class="material-symbols-outlined text-primary">{"folder"}</span>
                                                    <h3 class="font-headline-sm text-headline-sm text-on-surface font-bold">
                                                        {"Local Storage Configuration"}
                                                    </h3>
                                                </div>

                                                <div class="space-y-2">
                                                    <div class="flex items-center justify-between">
                                                        <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-tight font-semibold flex items-center gap-1">
                                                            {"Default Storage Path"}
                                                        </label>
                                                        <span class="inline-flex items-center gap-1 text-[11px] font-semibold text-on-surface-variant bg-surface-container px-2 py-0.5 rounded border border-outline-variant/40">
                                                            <span class="material-symbols-outlined text-xs">{"lock"}</span>
                                                            {"Read-only"}
                                                        </span>
                                                    </div>
                                                    <div class="relative">
                                                        <input
                                                            class="w-full bg-surface-container border border-outline-variant/60 px-cell_padding_h py-cell_padding_v font-code-md text-code-md text-on-surface-variant/80 rounded cursor-not-allowed select-all"
                                                            type="text"
                                                            value={(*local_path).clone()}
                                                            disabled={true}
                                                            readonly={true}
                                                        />
                                                    </div>
                                                    <p class="font-body-xs text-body-xs text-on-surface-variant">
                                                        {"Default directory path provided by server configuration where uploads are stored. This setting cannot be modified from the UI at this time."}
                                                    </p>
                                                </div>
                                            </div>
                                        }
                                    } else {
                                        html! {}
                                    }
                                }

                                /* S3 Configuration (when S3 is selected) */
                                {
                                    if is_s3 {
                                        html! {
                                            <div class="space-y-6">
                                                /* Migration Warning Alert */
                                                <div class="flex gap-3 p-4 bg-surface-container-highest border-l-4 border-primary text-on-surface-variant font-body-sm text-body-sm rounded-r-md">
                                                    <span class="material-symbols-outlined text-primary shrink-0">{"info"}</span>
                                                    <div>
                                                        {"If you have existing uploaded files, you will need to migrate them from the "}
                                                        <span class="font-bold">{"local file system"}</span>
                                                        {" to the "}
                                                        <span class="font-bold">{"S3 bucket"}</span>
                                                        {". Command-line tools such as "}
                                                        <code class="font-code-md text-primary bg-surface p-0.5 rounded">{"rclone"}</code>
                                                        {" or "}
                                                        <code class="font-code-md text-primary bg-surface p-0.5 rounded">{"s5cmd"}</code>
                                                        {" can assist with copying files."}
                                                    </div>
                                                </div>

                                                <div class="bg-surface-container-low border border-outline-variant p-6 space-y-6 rounded-lg">
                                                    <div class="flex items-center gap-2">
                                                        <span class="material-symbols-outlined text-primary">{"cloud"}</span>
                                                        <h3 class="font-headline-sm text-headline-sm text-on-surface font-bold">
                                                            {"S3 Connection Parameters"}
                                                        </h3>
                                                    </div>

                                                    <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
                                                        <div class="space-y-2">
                                                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-tight flex items-center gap-1 font-semibold">
                                                                {"Endpoint"}
                                                            </label>
                                                            <input
                                                                class="w-full bg-surface border border-outline-variant px-cell_padding_h py-cell_padding_v font-code-md text-code-md focus:ring-2 focus:ring-primary focus:border-primary transition-all rounded"
                                                                placeholder="e.g. s3.amazonaws.com"
                                                                type="text"
                                                                value={(*endpoint).clone()}
                                                                oninput={
                                                                    let endpoint = endpoint.clone();
                                                                    Callback::from(move |e: InputEvent| {
                                                                        if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                                                            endpoint.set(target.value());
                                                                        }
                                                                    })
                                                                }
                                                            />
                                                            <span class="text-[11px] text-on-surface-variant">{"Leave empty for AWS S3 standard endpoints"}</span>
                                                        </div>
                                                        <div class="space-y-2">
                                                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-tight flex items-center gap-1 font-semibold">
                                                                {"Bucket"} <span class="text-primary">{"*"}</span>
                                                            </label>
                                                            <input
                                                                class="w-full bg-surface border border-outline-variant px-cell_padding_h py-cell_padding_v font-code-md text-code-md focus:ring-2 focus:ring-primary focus:border-primary transition-all rounded"
                                                                placeholder="crabbase_files"
                                                                type="text"
                                                                value={(*bucket).clone()}
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
                                                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-tight flex items-center gap-1 font-semibold">
                                                                {"Region"} <span class="text-primary">{"*"}</span>
                                                            </label>
                                                            <input
                                                                class="w-full bg-surface border border-outline-variant px-cell_padding_h py-cell_padding_v font-code-md text-code-md focus:ring-2 focus:ring-primary focus:border-primary transition-all rounded"
                                                                placeholder="us-east-1"
                                                                type="text"
                                                                value={(*region).clone()}
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
                                                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-tight flex items-center gap-1 font-semibold">
                                                                {"Access key"}
                                                            </label>
                                                            <input
                                                                class="w-full bg-surface border border-outline-variant px-cell_padding_h py-cell_padding_v font-code-md text-code-md focus:ring-2 focus:ring-primary focus:border-primary transition-all rounded"
                                                                placeholder="AKIA..."
                                                                type="text"
                                                                value={(*access_key).clone()}
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
                                                            <label class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-tight flex items-center gap-1 font-semibold">
                                                                {"Secret key"}
                                                            </label>
                                                            <div class="relative">
                                                                <input
                                                                    class="w-full bg-surface border border-outline-variant px-cell_padding_h py-cell_padding_v font-code-md text-code-md focus:ring-2 focus:ring-primary focus:border-primary transition-all rounded pr-10"
                                                                    type={if *show_secret { "text" } else { "password" }}
                                                                    placeholder="••••••••••••••••"
                                                                    value={(*secret).clone()}
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
                                                            onclick={toggle_path_style}
                                                            class="w-4 h-4 text-primary border-outline focus:ring-primary rounded cursor-pointer"
                                                        />
                                                        <label class="font-body-sm text-body-sm text-on-surface-variant cursor-pointer flex items-center gap-1" for="path-style">
                                                            {"Force path-style addressing"}
                                                            <span class="material-symbols-outlined scale-75 text-outline" title="Useful for local development or MinIO/LocalStack emulation">{"help_outline"}</span>
                                                        </label>
                                                    </div>
                                                </div>
                                            </div>
                                        }
                                    } else {
                                        html! {}
                                    }
                                }

                                /* Footer Actions */
                                <div class="pt-8 border-t border-outline-variant flex justify-end items-center gap-4">
                                    <button
                                        type="button"
                                        onclick={on_cancel}
                                        disabled={*is_saving}
                                        class="px-6 py-2 text-on-surface-variant hover:bg-surface-container transition-all font-bold rounded cursor-pointer disabled:opacity-50"
                                    >
                                        {"Cancel"}
                                    </button>
                                    <button
                                        type="button"
                                        onclick={on_save}
                                        disabled={*is_saving}
                                        class="px-8 py-2.5 bg-primary text-on-primary font-bold rounded shadow-sm hover:shadow-md active:scale-95 transition-all cursor-pointer flex items-center gap-2 disabled:opacity-70"
                                    >
                                        {
                                            if *is_saving {
                                                html! {
                                                    <>
                                                        <span class="material-symbols-outlined animate-spin text-sm">{"progress_activity"}</span>
                                                        <span>{"Saving..."}</span>
                                                    </>
                                                }
                                            } else {
                                                html! {
                                                    <span>{"Save changes"}</span>
                                                }
                                            }
                                        }
                                    </button>
                                </div>
                            </>
                        }
                    }
                }
            </div>
        </main>
    }
}
