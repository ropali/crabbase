use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::api::client::ApiClient;
use crate::models::backup::Backup;

#[function_component(SettingsBackups)]
pub fn settings_backups() -> Html {
    let backups = use_state(Vec::<Backup>::new);
    let loading = use_state(|| true);
    let is_initializing = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let auto_backups_enabled = use_state(|| false);
    let backup_storage = use_state(|| "local".to_string());
    let is_saving_storage = use_state(|| false);
    let is_backup_options_open = use_state(|| true);

    let reload = {
        let backups = backups.clone();
        let loading = loading.clone();
        let error = error.clone();
        let auto_backups_enabled = auto_backups_enabled.clone();
        let backup_storage = backup_storage.clone();
        Callback::from(move |_| {
            let backups = backups.clone();
            let loading = loading.clone();
            let error = error.clone();
            let auto_backups_enabled = auto_backups_enabled.clone();
            let backup_storage = backup_storage.clone();
            loading.set(true);
            spawn_local(async move {
                let client = ApiClient::default();
                match client.list_backups().await {
                    Ok(resp) => {
                        backups.set(resp.items);
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                if let Ok(crons) = client.list_crons().await {
                    if let Some(cron) = crons.iter().find(|c| c.name == "db.backup") {
                        auto_backups_enabled.set(cron.enabled);
                    }
                }
                if let Ok(settings) = client.get_backup_settings().await {
                    if let Some(storage) = settings.get("storageType").and_then(|v| v.as_str()) {
                        backup_storage.set(storage.to_string());
                    }
                }
                loading.set(false);
            });
        })
    };

    {
        let reload = reload.clone();
        use_effect_with((), move |_| {
            reload.emit(());
            || ()
        });
    }

    let initialize_backup = {
        let is_initializing = is_initializing.clone();
        let error = error.clone();
        let backups = backups.clone();
        let loading = loading.clone();
        Callback::from(move |_| {
            let is_initializing = is_initializing.clone();
            let error = error.clone();
            let backups = backups.clone();
            let loading = loading.clone();
            is_initializing.set(true);
            error.set(None);
            spawn_local(async move {
                let client = ApiClient::default();
                if let Err(e) = client.create_backup().await {
                    error.set(Some(e.to_string()));
                    is_initializing.set(false);
                    return;
                }

                // Poll while the worker is dumping.
                for _ in 0..60 {
                    gloo_timers::future::TimeoutFuture::new(2000).await;
                    if let Ok(resp) = client.list_backups().await {
                        let active = resp
                            .items
                            .iter()
                            .any(|b| b.status == "pending" || b.status == "running");
                        backups.set(resp.items);
                        if !active {
                            break;
                        }
                    }
                }

                loading.set(false);
                is_initializing.set(false);
            });
        })
    };

    let toggle_auto_backups = {
        let auto_backups_enabled = auto_backups_enabled.clone();
        let error = error.clone();
        Callback::from(move |_| {
            let next = !*auto_backups_enabled;
            auto_backups_enabled.set(next);
            let error = error.clone();
            spawn_local(async move {
                let client = ApiClient::default();
                match client.list_crons().await {
                    Ok(crons) => {
                        if let Some(cron) = crons.iter().find(|c| c.name == "db.backup") {
                            if let Err(e) = client.update_cron(&cron.id, None, Some(next)).await {
                                error.set(Some(e.to_string()));
                            }
                        } else {
                            error.set(Some("db.backup cron is missing".to_string()));
                        }
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
            });
        })
    };

    let change_backup_storage = {
        let backup_storage = backup_storage.clone();
        let is_saving_storage = is_saving_storage.clone();
        let error = error.clone();
        Callback::from(move |e: Event| {
            let Some(target) = e.target_dyn_into::<web_sys::HtmlSelectElement>() else {
                return;
            };
            let next = target.value();
            if next == *backup_storage {
                return;
            }

            let previous = (*backup_storage).clone();
            backup_storage.set(next.clone());
            is_saving_storage.set(true);
            error.set(None);

            let backup_storage = backup_storage.clone();
            let is_saving_storage = is_saving_storage.clone();
            let error = error.clone();
            spawn_local(async move {
                let client = ApiClient::default();
                match client.get_backup_settings().await {
                    Ok(mut settings) => {
                        settings["storageType"] = serde_json::Value::String(next);
                        if let Err(e) = client.save_backup_settings(settings).await {
                            error.set(Some(e.to_string()));
                            backup_storage.set(previous);
                        }
                    }
                    Err(e) => {
                        error.set(Some(e.to_string()));
                        backup_storage.set(previous);
                    }
                }
                is_saving_storage.set(false);
            });
        })
    };

    let is_cloud_storage = *backup_storage == "cloud";

    let toggle_options_panel = {
        let is_backup_options_open = is_backup_options_open.clone();
        Callback::from(move |_| is_backup_options_open.set(!*is_backup_options_open))
    };

    let manual_count = backups
        .iter()
        .filter(|b| b.triggered_by == "manual")
        .count();
    let scheduled_count = backups
        .iter()
        .filter(|b| b.triggered_by != "manual")
        .count();
    let last_snapshot = backups
        .iter()
        .find(|b| b.status == "completed")
        .map(|b| short_time(&b.created))
        .unwrap_or_else(|| "--/--/--".to_string());
    let has_active = backups
        .iter()
        .any(|b| b.status == "pending" || b.status == "running");

    html! {
        <main class="flex-1 overflow-y-auto p-margin_page bg-surface">
            <div class="max-w-5xl mx-auto space-y-8">
                <div class="flex items-center gap-2 font-label-xs text-label-xs text-outline">
                    <a class="hover:text-primary transition-colors" href="#">{"Settings"}</a>
                    <span class="material-symbols-outlined text-[14px]">{"chevron_right"}</span>
                    <span class="text-on-surface">{"Backups"}</span>
                </div>

                <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
                    <div>
                        <h2 class="font-headline-lg text-headline-lg text-on-surface mb-1">{"Backup and restore your data"}</h2>
                        <p class="font-body-md text-body-md text-on-surface-variant">{"Manage snapshot schedules and local persistence for your Crabbase cluster."}</p>
                    </div>
                    <div class="flex items-center gap-2">
                        <button
                            onclick={let reload = reload.clone(); move |_| reload.emit(())}
                            class="flex items-center gap-2 px-3 py-2 bg-surface-container-low border border-outline-variant rounded-lg text-on-surface-variant hover:bg-surface-container transition-all text-body-sm cursor-pointer active:scale-95"
                        >
                            <span class="material-symbols-outlined text-[18px]">{"refresh"}</span>
                            <span>{"Refresh"}</span>
                        </button>
                    </div>
                </div>

                {
                    if let Some(message) = (*error).clone() {
                        html! {
                            <div class="p-4 bg-red-50 border border-red-200 text-red-800 rounded-lg flex items-center gap-2">
                                <span class="material-symbols-outlined">{"error"}</span>
                                <span class="font-body-sm text-body-sm font-bold">{message}</span>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }

                <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
                    <div class="md:col-span-2 bg-surface-container-lowest border border-outline-variant rounded-xl p-6 flex flex-col justify-center min-h-[300px] relative overflow-hidden shadow-sm">
                        <div class="absolute -right-12 -top-12 opacity-5 pointer-events-none">
                            <span class="material-symbols-outlined text-[240px]">{"database"}</span>
                        </div>
                        <div class="relative z-10 flex flex-col items-center text-center">
                            <div class="w-16 h-16 bg-surface-container rounded-full flex items-center justify-center mb-4 text-on-surface-variant">
                                <span class="material-symbols-outlined text-[32px]">
                                    {if has_active { "sync" } else { "database" }}
                                </span>
                            </div>
                            <h3 class="font-headline-md text-headline-md mb-2 text-on-surface">
                                {if has_active { "Backup in progress..." } else { "Create a new backup" }}
                            </h3>
                            <p class="font-body-sm text-body-sm text-on-surface-variant mb-6 max-w-xs">
                                {if is_cloud_storage {
                                    "A pg_dump custom-format snapshot will be generated locally and uploaded to your S3 storage."
                                } else {
                                    "A pg_dump custom-format snapshot will be generated and stored in the local backup directory."
                                }}
                            </p>
                            <button
                                onclick={initialize_backup}
                                disabled={*is_initializing || has_active}
                                class={classes!(
                                    "bg-primary", "text-white", "font-body-md", "text-body-md", "px-8", "py-3",
                                    "rounded-lg", "font-bold", "flex", "items-center", "gap-2",
                                    "transition-all", "shadow-sm", "cursor-pointer",
                                    if *is_initializing || has_active { "opacity-70 cursor-not-allowed" } else { "hover:opacity-90 active:scale-95" }
                                )}
                            >
                                <span class={classes!("material-symbols-outlined", "text-[20px]", if *is_initializing || has_active { "animate-spin" } else { "" })}>
                                    {if *is_initializing || has_active { "sync" } else { "play_circle" }}
                                </span>
                                {if *is_initializing || has_active { "Creating snapshot..." } else { "Initialize new backup" }}
                            </button>
                        </div>
                    </div>

                    <div class="space-y-6">
                        <div class="bg-surface-container-lowest border border-outline-variant rounded-xl p-5 shadow-sm">
                            <h4 class="font-label-xs text-label-xs text-outline uppercase mb-4 tracking-wider">{"Health Status"}</h4>
                            <div class="flex items-center justify-between mb-4">
                                <span class="font-body-sm text-body-sm">{"Database Engine"}</span>
                                <span class="px-2 py-0.5 bg-green-100 text-green-800 rounded font-label-xs text-label-xs font-bold flex items-center gap-1">
                                    <span class="w-1.5 h-1.5 rounded-full bg-green-600"></span>
                                    {"Healthy"}
                                </span>
                            </div>
                            <div class="flex items-center justify-between">
                                <span class="font-body-sm text-body-sm">{"Last Snapshot"}</span>
                                <span class="font-code-md text-code-md text-on-surface-variant">{last_snapshot}</span>
                            </div>
                        </div>

                        <div class="bg-surface-container-lowest border border-outline-variant rounded-xl p-5 shadow-sm">
                            <h4 class="font-label-xs text-label-xs text-outline uppercase mb-3 tracking-wider">{"Totals"}</h4>
                            <div class="flex items-center justify-between text-body-sm">
                                <span>{"Manual"}</span>
                                <span class="font-bold">{manual_count}</span>
                            </div>
                            <div class="flex items-center justify-between text-body-sm mt-2">
                                <span>{"Automatic"}</span>
                                <span class="font-bold">{scheduled_count}</span>
                            </div>
                        </div>
                    </div>
                </div>

                <div class="bg-surface-container-lowest border border-outline-variant rounded-xl overflow-hidden shadow-sm">
                    <div
                        onclick={toggle_options_panel}
                        class="border-b border-outline-variant p-4 bg-surface-container-low flex items-center justify-between cursor-pointer group hover:bg-surface-container-high transition-colors"
                    >
                        <div class="flex items-center gap-3">
                            <span class="material-symbols-outlined text-on-surface-variant">{"tune"}</span>
                            <span class="font-headline-md text-headline-md font-bold text-on-surface">{"Backup options"}</span>
                        </div>
                        <span class="material-symbols-outlined text-on-surface-variant group-hover:translate-y-0.5 transition-transform">
                            {if *is_backup_options_open { "expand_less" } else { "expand_more" }}
                        </span>
                    </div>

                    {
                        if *is_backup_options_open {
                            html! {
                                <div class="p-6 space-y-8">
                                    <div class="flex items-center justify-between">
                                        <div class="max-w-md">
                                            <h5 class="font-body-md text-body-md font-bold mb-1">{"Enable auto backups"}</h5>
                                            <p class="font-body-sm text-body-sm text-on-surface-variant">{"Runs the db.backup cron (02:00 UTC by default). You can change the schedule from the Crons page."}</p>
                                        </div>
                                        <button
                                            type="button"
                                            onclick={toggle_auto_backups}
                                            class={classes!(
                                                "relative", "inline-flex", "h-6", "w-11", "shrink-0", "cursor-pointer",
                                                "rounded-full", "border-2", "border-transparent", "transition-colors",
                                                "duration-200", "ease-in-out", "focus:outline-none",
                                                if *auto_backups_enabled { "bg-primary" } else { "bg-outline-variant" }
                                            )}
                                        >
                                            <span
                                                class={classes!(
                                                    "pointer-events-none", "inline-block", "h-5", "w-5", "transform",
                                                    "rounded-full", "bg-white", "shadow", "ring-0", "transition",
                                                    "duration-200", "ease-in-out",
                                                    if *auto_backups_enabled { "translate-x-5" } else { "translate-x-0" }
                                                )}
                                            />
                                        </button>
                                    </div>

                                    <div class="h-[1px] bg-outline-variant w-full"></div>

                                    <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
                                        <div class="max-w-md">
                                            <h5 class="font-body-md text-body-md font-bold mb-1">{"Backup destination"}</h5>
                                            <p class="font-body-sm text-body-sm text-on-surface-variant">
                                                {"Choose where completed snapshots are stored. Cloud uploads use the S3 configuration defined in "}
                                                <a class="text-primary font-bold hover:underline" href="/settings/storage">{"File storage"}</a>
                                                {"."}
                                            </p>
                                        </div>
                                        <div class="flex items-center gap-3">
                                            {
                                                if *is_saving_storage {
                                                    html! {
                                                        <span class="material-symbols-outlined animate-spin text-on-surface-variant text-[18px]">{"sync"}</span>
                                                    }
                                                } else {
                                                    html! {}
                                                }
                                            }
                                            <div class="relative">
                                                <select
                                                    value={(*backup_storage).clone()}
                                                    onchange={change_backup_storage}
                                                    disabled={*is_saving_storage}
                                                    class="bg-surface border border-outline-variant px-4 py-2.5 font-body-sm text-body-sm text-on-surface focus:ring-2 focus:ring-primary focus:border-primary transition-all rounded cursor-pointer appearance-none pr-10 disabled:opacity-60"
                                                >
                                                    <option value="local" selected={!is_cloud_storage}>{"Local disk"}</option>
                                                    <option value="cloud" selected={is_cloud_storage}>{"Cloud (S3)"}</option>
                                                </select>
                                                <div class="pointer-events-none absolute inset-y-0 right-0 flex items-center px-3 text-on-surface-variant">
                                                    <span class="material-symbols-outlined text-sm">{"unfold_more"}</span>
                                                </div>
                                            </div>
                                        </div>
                                    </div>

                                    {
                                        if is_cloud_storage {
                                            html! {
                                                <div class="flex gap-3 p-4 bg-surface-container-highest border-l-4 border-primary text-on-surface-variant font-body-sm text-body-sm rounded-r-md">
                                                    <span class="material-symbols-outlined text-primary shrink-0">{"info"}</span>
                                                    <div>
                                                        {"Backups will be uploaded to the S3 bucket configured in "}
                                                        <a class="text-primary font-bold hover:underline" href="/settings/storage">{"File storage"}</a>
                                                        {". The local copy is kept on disk until the retention policy prunes it."}
                                                    </div>
                                                </div>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                </div>
                            }
                        } else {
                            html! {}
                        }
                    }
                </div>

                <div class="mt-8">
                    <div class="flex items-center justify-between mb-4">
                        <h3 class="font-headline-md text-headline-md font-bold text-on-surface">{"Backup History"}</h3>
                        <div class="flex items-center gap-4 text-outline font-label-xs text-label-xs">
                            <span class="flex items-center gap-1">
                                <span class="w-2 h-2 rounded-full bg-primary"></span>
                                {format!("{} Manual", manual_count)}
                            </span>
                            <span class="flex items-center gap-1">
                                <span class="w-2 h-2 rounded-full bg-secondary"></span>
                                {format!("{} Automated", scheduled_count)}
                            </span>
                        </div>
                    </div>

                    <div class="bg-surface-container-lowest border border-outline-variant rounded-xl overflow-hidden shadow-sm">
                        <table class="w-full text-left border-collapse">
                            <thead>
                                <tr class="bg-surface-container-low border-b border-outline-variant">
                                    <th class="px-cell_padding_h py-cell_padding_v font-label-xs text-label-xs uppercase text-outline tracking-wider">{"Filename"}</th>
                                    <th class="px-cell_padding_h py-cell_padding_v font-label-xs text-label-xs uppercase text-outline tracking-wider">{"Created"}</th>
                                    <th class="px-cell_padding_h py-cell_padding_v font-label-xs text-label-xs uppercase text-outline tracking-wider">{"Size"}</th>
                                    <th class="px-cell_padding_h py-cell_padding_v font-label-xs text-label-xs uppercase text-outline tracking-wider">{"Type"}</th>
                                    <th class="px-cell_padding_h py-cell_padding_v font-label-xs text-label-xs uppercase text-outline tracking-wider">{"Storage"}</th>
                                    <th class="px-cell_padding_h py-cell_padding_v font-label-xs text-label-xs uppercase text-outline tracking-wider">{"Status"}</th>
                                    <th class="px-cell_padding_h py-cell_padding_v font-label-xs text-label-xs uppercase text-outline tracking-wider text-right">{"Actions"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {
                                    if *loading && backups.is_empty() {
                                        html! {
                                            <tr>
                                                <td class="px-cell_padding_h py-12 text-center" colspan="7">
                                                    <span class="material-symbols-outlined animate-spin text-outline text-[32px]">{"sync"}</span>
                                                </td>
                                            </tr>
                                        }
                                    } else if backups.is_empty() {
                                        html! {
                                            <tr>
                                                <td class="px-cell_padding_h py-12 text-center" colspan="7">
                                                    <div class="opacity-40 flex flex-col items-center">
                                                        <span class="material-symbols-outlined text-[48px] mb-2 text-outline">{"history"}</span>
                                                        <p class="font-body-sm text-body-sm italic">{"No backup records to display"}</p>
                                                    </div>
                                                </td>
                                            </tr>
                                        }
                                    } else {
                                        html! {
                                            <>
                                                { for backups.iter().map(|backup| {
                                                    let id_download = backup.id.clone();
                                                    let filename = backup.filename.clone();
                                                    let error_download = error.clone();
                                                    let on_download = Callback::from(move |_| {
                                                        let id = id_download.clone();
                                                        let filename = filename.clone();
                                                        let error = error_download.clone();
                                                        spawn_local(async move {
                                                            let client = ApiClient::default();
                                                            match client.download_backup(&id).await {
                                                                Ok(bytes) => trigger_download(&filename, bytes),
                                                                Err(e) => error.set(Some(e.to_string())),
                                                            }
                                                        });
                                                    });

                                                    let id_delete = backup.id.clone();
                                                    let error_delete = error.clone();
                                                    let reload = reload.clone();
                                                    let on_delete = Callback::from(move |_| {
                                                        let id = id_delete.clone();
                                                        let error = error_delete.clone();
                                                        let reload = reload.clone();
                                                        spawn_local(async move {
                                                            let client = ApiClient::default();
                                                            match client.delete_backup(&id).await {
                                                                Ok(()) => reload.emit(()),
                                                                Err(e) => error.set(Some(e.to_string())),
                                                            }
                                                        });
                                                    });

                                                    let completed = backup.status == "completed";

                                                    html! {
                                                        <tr class="border-b border-outline-variant/30 hover:bg-surface-container-low transition-colors">
                                                            <td class="px-cell_padding_h py-3 font-code-md text-code-md font-bold text-on-surface">{&backup.filename}</td>
                                                            <td class="px-cell_padding_h py-3 font-body-sm text-body-sm text-on-surface-variant">{short_time(&backup.created)}</td>
                                                            <td class="px-cell_padding_h py-3 font-code-md text-code-md text-on-surface-variant">{format_size(backup.size_bytes)}</td>
                                                            <td class="px-cell_padding_h py-3 font-body-sm text-body-sm">
                                                                <span class="px-2 py-0.5 bg-primary-container/10 text-primary rounded font-label-xs text-label-xs font-bold">
                                                                    {display_trigger(&backup.triggered_by)}
                                                                </span>
                                                            </td>
                                                            <td class="px-cell_padding_h py-3 font-body-sm text-body-sm">
                                                                <span class={classes!("px-2", "py-0.5", "rounded", "font-label-xs", "text-label-xs", "font-bold", storage_class(&backup.storage_backend))}>
                                                                    {display_storage(&backup.storage_backend)}
                                                                </span>
                                                            </td>
                                                            <td class="px-cell_padding_h py-3 font-body-sm text-body-sm">
                                                                <span class={classes!("px-2", "py-0.5", "rounded", "font-label-xs", "text-label-xs", "font-bold", status_class(&backup.status))}>
                                                                    {&backup.status}
                                                                </span>
                                                            </td>
                                                            <td class="px-cell_padding_h py-3 text-right space-x-2">
                                                                <button
                                                                    onclick={on_download}
                                                                    disabled={!completed}
                                                                    class={classes!(
                                                                        "px-3", "py-1", "rounded", "font-label-xs", "text-label-xs", "font-bold", "transition-colors",
                                                                        if completed { "bg-surface-container hover:bg-outline-variant text-on-surface-variant cursor-pointer" } else { "bg-surface-container text-outline cursor-not-allowed opacity-50" }
                                                                    )}
                                                                >
                                                                    {"Download"}
                                                                </button>
                                                                <button
                                                                    onclick={on_delete}
                                                                    class="px-3 py-1 bg-red-50 hover:bg-red-100 text-red-700 rounded font-label-xs text-label-xs font-bold transition-colors cursor-pointer"
                                                                >
                                                                    {"Delete"}
                                                                </button>
                                                            </td>
                                                        </tr>
                                                    }
                                                }) }
                                            </>
                                        }
                                    }
                                }
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>
        </main>
    }
}

fn short_time(value: &str) -> String {
    value
        .replace('T', " ")
        .split('.')
        .next()
        .map(|s| s.trim_end_matches('Z').to_string())
        .unwrap_or_else(|| value.to_string())
}

fn format_size(bytes: Option<i64>) -> String {
    let Some(bytes) = bytes else {
        return "--".to_string();
    };
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn display_trigger(value: &str) -> &str {
    match value {
        "manual" => "Manual",
        "scheduled" => "Automated",
        "cli" => "CLI",
        other => other,
    }
}

fn display_storage(value: &str) -> &'static str {
    match value {
        "s3" | "cloud" => "Cloud",
        _ => "Local",
    }
}

fn storage_class(value: &str) -> &'static str {
    match value {
        "s3" | "cloud" => "bg-blue-100 text-blue-800",
        _ => "bg-surface-container text-on-surface-variant",
    }
}

fn status_class(status: &str) -> &'static str {
    match status {
        "completed" => "bg-green-100 text-green-800",
        "failed" => "bg-red-100 text-red-800",
        "running" => "bg-blue-100 text-blue-800",
        _ => "bg-yellow-100 text-yellow-800",
    }
}

fn trigger_download(filename: &str, bytes: Vec<u8>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };

    let parts = js_sys::Array::new();
    let data = js_sys::Uint8Array::from(bytes.as_slice());
    parts.push(&data);

    let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence(parts.as_ref()) else {
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };

    if let Some(anchor) = document
        .create_element("a")
        .ok()
        .and_then(|el| el.dyn_into::<web_sys::HtmlAnchorElement>().ok())
    {
        anchor.set_href(&url);
        anchor.set_download(filename);
        anchor.click();
    }

    let _ = web_sys::Url::revoke_object_url(&url);
}
