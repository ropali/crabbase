use yew::prelude::*;

#[function_component(SettingsBackups)]
pub fn settings_backups() -> Html {
    let auto_backups_enabled = use_state(|| true);
    let s3_backups_enabled = use_state(|| false);
    let is_backup_options_open = use_state(|| true);
    let is_initializing = use_state(|| false);
    let is_backup_complete = use_state(|| false);
    let is_saved = use_state(|| false);

    let toggle_auto_backups = {
        let auto_backups_enabled = auto_backups_enabled.clone();
        Callback::from(move |_| {
            auto_backups_enabled.set(!*auto_backups_enabled);
        })
    };

    let toggle_s3_backups = {
        let s3_backups_enabled = s3_backups_enabled.clone();
        Callback::from(move |_| {
            s3_backups_enabled.set(!*s3_backups_enabled);
        })
    };

    let toggle_options_panel = {
        let is_backup_options_open = is_backup_options_open.clone();
        Callback::from(move |_| {
            is_backup_options_open.set(!*is_backup_options_open);
        })
    };

    let initialize_backup = {
        let is_initializing = is_initializing.clone();
        let is_backup_complete = is_backup_complete.clone();
        Callback::from(move |_| {
            is_initializing.set(true);
            let is_initializing = is_initializing.clone();
            let is_backup_complete = is_backup_complete.clone();
            gloo_timers::callback::Timeout::new(1500, move || {
                is_initializing.set(false);
                is_backup_complete.set(true);
            })
            .forget();
        })
    };

    let on_save = {
        let is_saved = is_saved.clone();
        Callback::from(move |_| {
            is_saved.set(true);
        })
    };

    html! {
        <main class="flex-1 overflow-y-auto p-margin_page bg-surface">
            <div class="max-w-5xl mx-auto space-y-8">
                /* Breadcrumbs */
                <div class="flex items-center gap-2 font-label-xs text-label-xs text-outline">
                    <a class="hover:text-primary transition-colors" href="#">{"Settings"}</a>
                    <span class="material-symbols-outlined text-[14px]">{"chevron_right"}</span>
                    <span class="text-on-surface">{"Backups"}</span>
                </div>

                /* Page Title Area */
                <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
                    <div>
                        <h2 class="font-headline-lg text-headline-lg text-on-surface mb-1">{"Backup and restore your data"}</h2>
                        <p class="font-body-md text-body-md text-on-surface-variant">{"Manage snapshot schedules and cloud persistence for your Crabbase clusters."}</p>
                    </div>
                    <div class="flex items-center gap-2">
                        <button class="flex items-center gap-2 px-3 py-2 bg-surface-container-low border border-outline-variant rounded-lg text-on-surface-variant hover:bg-surface-container transition-all text-body-sm cursor-pointer active:scale-95">
                            <span class="material-symbols-outlined text-[18px]">{"refresh"}</span>
                            <span>{"Refresh"}</span>
                        </button>
                        <button class="flex items-center gap-2 px-3 py-2 bg-surface-container-low border border-outline-variant rounded-lg text-on-surface-variant hover:bg-surface-container transition-all text-body-sm cursor-pointer active:scale-95">
                            <span class="material-symbols-outlined text-[18px]">{"cloud"}</span>
                            <span>{"Connect Storage"}</span>
                        </button>
                    </div>
                </div>

                {
                    if *is_saved {
                        html! {
                            <div class="p-4 bg-primary-container/10 border border-primary-container/30 text-primary rounded-lg flex items-center justify-between transition-all">
                                <div class="flex items-center gap-2">
                                    <span class="material-symbols-outlined">{"check_circle"}</span>
                                    <span class="font-body-sm text-body-sm font-bold">{"Backup settings saved successfully!"}</span>
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

                /* Bento Grid - Backup Overview */
                <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
                    /* Main Backup Interface Card */
                    <div class="md:col-span-2 bg-surface-container-lowest border border-outline-variant rounded-xl p-6 flex flex-col justify-center min-h-[300px] relative overflow-hidden shadow-sm">
                        <div class="absolute -right-12 -top-12 opacity-5 pointer-events-none">
                            <span class="material-symbols-outlined text-[240px]">{"database"}</span>
                        </div>
                        <div class="relative z-10 flex flex-col items-center text-center">
                            {
                                if *is_backup_complete {
                                    html! {
                                        <>
                                            <div class="w-16 h-16 bg-green-100 rounded-full flex items-center justify-center mb-4 text-green-700">
                                                <span class="material-symbols-outlined text-[32px]">{"check_circle"}</span>
                                            </div>
                                            <h3 class="font-headline-md text-headline-md mb-2 text-on-surface">{"Snapshot Created Successfully!"}</h3>
                                            <p class="font-body-sm text-body-sm text-on-surface-variant mb-6 max-w-xs">
                                                {"System snapshot snapshot_latest.zip has been generated and saved locally."}
                                            </p>
                                            <button
                                                onclick={initialize_backup}
                                                class="bg-primary text-white font-body-md text-body-md px-8 py-3 rounded-lg font-bold flex items-center gap-2 hover:opacity-90 active:scale-95 transition-all shadow-sm cursor-pointer"
                                            >
                                                <span class="material-symbols-outlined text-[20px]">{"refresh"}</span>
                                                {"Create another backup"}
                                            </button>
                                        </>
                                    }
                                } else {
                                    html! {
                                        <>
                                            <div class="w-16 h-16 bg-surface-container rounded-full flex items-center justify-center mb-4 text-on-surface-variant">
                                                <span class="material-symbols-outlined text-[32px]">{"folder_off"}</span>
                                            </div>
                                            <h3 class="font-headline-md text-headline-md mb-2 text-on-surface">{"No backups found."}</h3>
                                            <p class="font-body-sm text-body-sm text-on-surface-variant mb-6 max-w-xs">
                                                {"You haven't created any manual or scheduled backups yet. Start by initializing your first system snapshot."}
                                            </p>
                                            <button
                                                onclick={initialize_backup}
                                                disabled={*is_initializing}
                                                class={classes!(
                                                    "bg-primary", "text-white", "font-body-md", "text-body-md", "px-8", "py-3",
                                                    "rounded-lg", "font-bold", "flex", "items-center", "gap-2",
                                                    "transition-all", "shadow-sm", "cursor-pointer",
                                                    if *is_initializing { "opacity-70 cursor-not-allowed" } else { "hover:opacity-90 active:scale-95" }
                                                )}
                                            >
                                                <span class={classes!("material-symbols-outlined", "text-[20px]", if *is_initializing { "animate-spin" } else { "" })}>
                                                    {if *is_initializing { "sync" } else { "play_circle" }}
                                                </span>
                                                {if *is_initializing { "Creating snapshot..." } else { "Initialize new backup" }}
                                            </button>
                                        </>
                                    }
                                }
                            }
                        </div>
                    </div>

                    /* Stats/Quick Info Sidebar */
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
                                <span class="font-code-md text-code-md text-on-surface-variant">
                                    {if *is_backup_complete { "Just now" } else { "--/--/--" }}
                                </span>
                            </div>
                        </div>

                        <div class="bg-primary-container/10 border border-primary-container/20 rounded-xl p-5 shadow-sm">
                            <h4 class="font-label-xs text-label-xs text-primary-container uppercase mb-2 tracking-wider font-bold">{"System Recommendation"}</h4>
                            <p class="font-body-sm text-body-sm text-on-surface mb-3 leading-relaxed">
                                {"Consider enabling automated S3 backups to prevent data loss during hardware failures."}
                            </p>
                            <a class="font-label-xs text-label-xs text-primary font-bold hover:underline" href="#">{"Learn more about redundancy"}</a>
                        </div>
                    </div>
                </div>

                /* Configuration Section */
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
                                    /* Toggle 1: Auto Backups */
                                    <div class="flex items-center justify-between">
                                        <div class="max-w-md">
                                            <h5 class="font-body-md text-body-md font-bold mb-1">{"Enable auto backups"}</h5>
                                            <p class="font-body-sm text-body-sm text-on-surface-variant">{"Automatically create a system snapshot every 24 hours at midnight UTC. Includes all collections and schema definitions."}</p>
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

                                    /* Toggle 2: S3 Backups */
                                    <div class="flex items-center justify-between">
                                        <div class="max-w-md">
                                            <h5 class="font-body-md text-body-md font-bold mb-1">{"Store backups in S3 storage"}</h5>
                                            <p class="font-body-sm text-body-sm text-on-surface-variant">{"Encrypt and upload completed backups to a remote Amazon S3, MinIO, or Google Cloud bucket for geographic redundancy."}</p>
                                        </div>
                                        <button
                                            type="button"
                                            onclick={toggle_s3_backups}
                                            class={classes!(
                                                "relative", "inline-flex", "h-6", "w-11", "shrink-0", "cursor-pointer",
                                                "rounded-full", "border-2", "border-transparent", "transition-colors",
                                                "duration-200", "ease-in-out", "focus:outline-none",
                                                if *s3_backups_enabled { "bg-primary" } else { "bg-outline-variant" }
                                            )}
                                        >
                                            <span
                                                class={classes!(
                                                    "pointer-events-none", "inline-block", "h-5", "w-5", "transform",
                                                    "rounded-full", "bg-white", "shadow", "ring-0", "transition",
                                                    "duration-200", "ease-in-out",
                                                    if *s3_backups_enabled { "translate-x-5" } else { "translate-x-0" }
                                                )}
                                            />
                                        </button>
                                    </div>

                                    {
                                        if *s3_backups_enabled {
                                            html! {
                                                <div class="bg-surface-container p-4 rounded-lg flex items-start gap-3 border border-outline-variant/30">
                                                    <span class="material-symbols-outlined text-primary text-[20px]">{"info"}</span>
                                                    <div>
                                                        <p class="font-body-sm text-body-sm font-bold text-on-surface">{"Configuration Required"}</p>
                                                        <p class="font-body-sm text-body-sm text-on-surface-variant">
                                                            {"S3 storage requires credentials (Access Key, Secret Key, Endpoint) to be configured in the File storage section before activation."}
                                                        </p>
                                                    </div>
                                                </div>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }

                                    /* Footer Actions */
                                    <div class="pt-4 border-t border-outline-variant flex justify-end">
                                        <button
                                            type="button"
                                            onclick={on_save}
                                            class="bg-primary text-white font-body-md text-body-md px-6 py-2 rounded-lg font-bold hover:opacity-90 active:scale-95 transition-all cursor-pointer"
                                        >
                                            {"Save changes"}
                                        </button>
                                    </div>
                                </div>
                            }
                        } else {
                            html! {}
                        }
                    }
                </div>

                /* Backup History Table Section */
                <div class="mt-8">
                    <div class="flex items-center justify-between mb-4">
                        <h3 class="font-headline-md text-headline-md font-bold text-on-surface">{"Backup History"}</h3>
                        <div class="flex items-center gap-4 text-outline font-label-xs text-label-xs">
                            <span class="flex items-center gap-1">
                                <span class="w-2 h-2 rounded-full bg-primary"></span>
                                {if *is_backup_complete { "1 Manual" } else { "0 Manual" }}
                            </span>
                            <span class="flex items-center gap-1">
                                <span class="w-2 h-2 rounded-full bg-secondary"></span>
                                {"0 Automated"}
                            </span>
                        </div>
                    </div>

                    <div class="bg-surface-container-lowest border border-outline-variant rounded-xl overflow-hidden shadow-sm">
                        <table class="w-full text-left border-collapse">
                            <thead>
                                <tr class="bg-surface-container-low border-b border-outline-variant">
                                    <th class="px-cell_padding_h py-cell_padding_v font-label-xs text-label-xs uppercase text-outline tracking-wider">{"ID / Filename"}</th>
                                    <th class="px-cell_padding_h py-cell_padding_v font-label-xs text-label-xs uppercase text-outline tracking-wider">{"Created"}</th>
                                    <th class="px-cell_padding_h py-cell_padding_v font-label-xs text-label-xs uppercase text-outline tracking-wider">{"Size"}</th>
                                    <th class="px-cell_padding_h py-cell_padding_v font-label-xs text-label-xs uppercase text-outline tracking-wider">{"Type"}</th>
                                    <th class="px-cell_padding_h py-cell_padding_v font-label-xs text-label-xs uppercase text-outline tracking-wider">{"Status"}</th>
                                    <th class="px-cell_padding_h py-cell_padding_v font-label-xs text-label-xs uppercase text-outline tracking-wider text-right">{"Actions"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {
                                    if *is_backup_complete {
                                        html! {
                                            <tr class="border-b border-outline-variant/30 hover:bg-surface-container-low transition-colors">
                                                <td class="px-cell_padding_h py-3 font-code-md text-code-md font-bold text-on-surface">{"snapshot_manual_latest.zip"}</td>
                                                <td class="px-cell_padding_h py-3 font-body-sm text-body-sm text-on-surface-variant">{"Just now"}</td>
                                                <td class="px-cell_padding_h py-3 font-code-md text-code-md text-on-surface-variant">{"1.2 MB"}</td>
                                                <td class="px-cell_padding_h py-3 font-body-sm text-body-sm">
                                                    <span class="px-2 py-0.5 bg-primary-container/10 text-primary rounded font-label-xs text-label-xs font-bold">{"Manual"}</span>
                                                </td>
                                                <td class="px-cell_padding_h py-3 font-body-sm text-body-sm">
                                                    <span class="px-2 py-0.5 bg-green-100 text-green-800 rounded font-label-xs text-label-xs font-bold">{"Completed"}</span>
                                                </td>
                                                <td class="px-cell_padding_h py-3 text-right">
                                                    <button class="px-3 py-1 bg-surface-container hover:bg-outline-variant rounded font-label-xs text-label-xs font-bold text-on-surface-variant transition-colors cursor-pointer">
                                                        {"Download"}
                                                    </button>
                                                </td>
                                            </tr>
                                        }
                                    } else {
                                        html! {
                                            <tr>
                                                <td class="px-cell_padding_h py-12 text-center" colspan="6">
                                                    <div class="opacity-40 flex flex-col items-center">
                                                        <span class="material-symbols-outlined text-[48px] mb-2 text-outline">{"history"}</span>
                                                        <p class="font-body-sm text-body-sm italic">{"No backup records to display"}</p>
                                                    </div>
                                                </td>
                                            </tr>
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
