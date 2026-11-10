use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::api::client::ApiClient;
use crate::models::backup::Cron;

#[function_component(SettingsCrons)]
pub fn settings_crons() -> Html {
    let crons = use_state(Vec::<Cron>::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);

    let reload = {
        let crons = crons.clone();
        let loading = loading.clone();
        let error = error.clone();
        Callback::from(move |_| {
            let crons = crons.clone();
            let loading = loading.clone();
            let error = error.clone();
            loading.set(true);
            spawn_local(async move {
                let client = ApiClient::default();
                match client.list_crons().await {
                    Ok(items) => {
                        crons.set(items);
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e.to_string())),
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

    let enabled = crons.iter().filter(|c| c.enabled).count();

    html! {
        <main class="flex-1 overflow-y-auto bg-surface p-margin_page">
            <div class="max-w-4xl mx-auto space-y-8">
                <nav class="flex items-center gap-2 mb-4 text-on-surface-variant font-label-xs text-label-xs">
                    <span class="hover:text-primary cursor-pointer transition-colors">{"Settings"}</span>
                    <span class="material-symbols-outlined text-[14px]">{"chevron_right"}</span>
                    <span class="font-bold text-on-surface">{"Crons"}</span>
                </nav>

                <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-8">
                    <div class="md:col-span-2 bg-surface-container-lowest border border-outline-variant p-6 rounded-xl flex flex-col justify-between shadow-sm">
                        <div>
                            <div class="flex items-center justify-between mb-2">
                                <h1 class="font-headline-lg text-headline-lg text-on-surface">{"Scheduled Tasks"}</h1>
                                <button
                                    onclick={let reload = reload.clone(); move |_| reload.emit(())}
                                    class="p-2 hover:bg-surface-container rounded-full transition-all group cursor-pointer"
                                >
                                    <span class="material-symbols-outlined text-primary group-active:rotate-180 transition-transform duration-500">{"refresh"}</span>
                                </button>
                            </div>
                            <p class="font-body-md text-body-md text-on-surface-variant max-w-lg">
                                {"Manage and monitor system-level background jobs. Schedules are stored in the database and run by the job worker."}
                            </p>
                        </div>
                        <div class="mt-6 flex items-center gap-4">
                            <div class="flex items-center gap-2 px-3 py-1 bg-surface-container rounded-full">
                                <span class="w-2 h-2 rounded-full bg-green-500 animate-pulse"></span>
                                <span class="text-label-xs font-label-xs text-on-surface-variant">
                                    {format!("{enabled} of {} enabled", crons.len())}
                                </span>
                            </div>
                        </div>
                    </div>

                    <div class="bg-primary-container p-6 rounded-xl text-on-primary-container relative overflow-hidden flex flex-col justify-between shadow-sm">
                        <div class="z-10">
                            <span class="material-symbols-outlined text-white/70 text-[32px] mb-2" style="font-variation-settings: 'FILL' 1;">{"terminal"}</span>
                            <h3 class="font-headline-md text-headline-md mb-1 font-bold text-white">{"Total Jobs"}</h3>
                            <p class="text-[40px] font-bold leading-none text-white">{format!("{:02}", crons.len())}</p>
                        </div>
                        <p class="text-label-xs font-label-xs text-white/80 z-10 font-medium">{"Durable Postgres queue"}</p>
                        <div class="absolute -right-4 -bottom-4 w-32 h-32 border-[20px] border-white/10 rounded-full"></div>
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

                <div class="bg-surface-container-lowest border border-outline-variant rounded-xl overflow-hidden shadow-sm">
                    <div class="bg-surface-container-low px-cell_padding_h py-3 border-b border-outline-variant grid grid-cols-12 items-center gap-3">
                        <div class="col-span-3 font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Job"}</div>
                        <div class="col-span-4 font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Schedule (UTC)"}</div>
                        <div class="col-span-2 font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Last"}</div>
                        <div class="col-span-3 text-right font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Actions"}</div>
                    </div>

                    <div class="divide-y divide-outline-variant/40">
                        {
                            if *loading && crons.is_empty() {
                                html! {
                                    <div class="py-12 text-center">
                                        <span class="material-symbols-outlined animate-spin text-outline text-[32px]">{"sync"}</span>
                                    </div>
                                }
                            } else {
                                html! {
                                    <>
                                        { for crons.iter().map(|cron| html! {
                                            <CronRow cron={cron.clone()} on_changed={reload.clone()} />
                                        }) }
                                    </>
                                }
                            }
                        }
                    </div>
                </div>

                <div class="mt-8 p-6 bg-surface-container rounded-lg border border-dashed border-outline-variant flex items-start gap-4">
                    <span class="material-symbols-outlined text-primary">{"info"}</span>
                    <div>
                        <p class="font-body-sm text-body-sm text-on-surface-variant">
                            {"A manual run goes through the same dedupe index. If a run is already queued or running, the API returns 409 and nothing is enqueued twice."}
                        </p>
                    </div>
                </div>
            </div>
        </main>
    }
}

#[derive(Properties, PartialEq)]
struct CronRowProps {
    cron: Cron,
    on_changed: Callback<()>,
}

#[function_component(CronRow)]
fn cron_row(props: &CronRowProps) -> Html {
    let schedule = use_state(|| props.cron.schedule.clone());
    let busy = use_state(|| false);
    let message = use_state(|| Option::<String>::None);

    {
        let schedule = schedule.clone();
        let value = props.cron.schedule.clone();
        use_effect_with(value, move |value| {
            schedule.set(value.clone());
            || ()
        });
    }

    let on_input = {
        let schedule = schedule.clone();
        Callback::from(move |e: InputEvent| {
            let input = e.target_unchecked_into::<web_sys::HtmlInputElement>();
            schedule.set(input.value());
        })
    };

    let save_schedule = {
        let schedule = schedule.clone();
        let cron = props.cron.clone();
        let on_changed = props.on_changed.clone();
        let message = message.clone();
        let busy = busy.clone();
        Callback::from(move |_| {
            let schedule = (*schedule).clone();
            let cron = cron.clone();
            let on_changed = on_changed.clone();
            let message = message.clone();
            let busy = busy.clone();
            busy.set(true);
            message.set(None);
            spawn_local(async move {
                let client = ApiClient::default();
                match client.update_cron(&cron.id, Some(&schedule), None).await {
                    Ok(_) => on_changed.emit(()),
                    Err(e) => message.set(Some(e.to_string())),
                }
                busy.set(false);
            });
        })
    };

    let toggle = {
        let cron = props.cron.clone();
        let on_changed = props.on_changed.clone();
        let message = message.clone();
        let busy = busy.clone();
        Callback::from(move |_| {
            let cron = cron.clone();
            let on_changed = on_changed.clone();
            let message = message.clone();
            let busy = busy.clone();
            let next = !cron.enabled;
            busy.set(true);
            message.set(None);
            spawn_local(async move {
                let client = ApiClient::default();
                match client.update_cron(&cron.id, None, Some(next)).await {
                    Ok(_) => on_changed.emit(()),
                    Err(e) => message.set(Some(e.to_string())),
                }
                busy.set(false);
            });
        })
    };

    let run_now = {
        let cron = props.cron.clone();
        let on_changed = props.on_changed.clone();
        let message = message.clone();
        let busy = busy.clone();
        Callback::from(move |_| {
            let cron = cron.clone();
            let on_changed = on_changed.clone();
            let message = message.clone();
            let busy = busy.clone();
            busy.set(true);
            message.set(None);
            spawn_local(async move {
                let client = ApiClient::default();
                match client.run_cron(&cron.id).await {
                    Ok(_) => on_changed.emit(()),
                    Err(e) => message.set(Some(e.to_string())),
                }
                busy.set(false);
            });
        })
    };

    let last = props
        .cron
        .last_run
        .as_deref()
        .map(short_time)
        .unwrap_or_else(|| "--".to_string());
    let next = props
        .cron
        .next_run
        .as_deref()
        .map(short_time)
        .unwrap_or_else(|| "--".to_string());

    html! {
        <div class="grid grid-cols-12 items-center gap-3 px-cell_padding_h py-3 hover:bg-surface-container-low transition-colors group">
            <div class="col-span-3 min-w-0">
                <div class="flex items-center gap-2">
                    <span class={classes!(
                        "w-2", "h-2", "rounded-full", "shrink-0",
                        if props.cron.enabled { "bg-green-500" } else { "bg-outline" }
                    )}></span>
                    <div class="min-w-0">
                        <p class="font-code-md text-code-md font-bold text-on-surface truncate">{&props.cron.name}</p>
                        <p class="font-label-xs text-label-xs text-outline truncate">{&props.cron.kind}</p>
                    </div>
                </div>
                {
                    if let Some(msg) = (*message).clone() {
                        html! { <p class="font-label-xs text-label-xs text-red-600 mt-1">{msg}</p> }
                    } else if let Some(err) = props.cron.last_error.clone() {
                        html! { <p class="font-label-xs text-label-xs text-red-600 mt-1 truncate" title={err.clone()}>{err}</p> }
                    } else {
                        html! {}
                    }
                }
            </div>

            <div class="col-span-4 flex items-center gap-2">
                <input
                    type="text"
                    value={(*schedule).clone()}
                    oninput={on_input}
                    class="w-full font-code-md text-code-md text-on-surface-variant bg-surface-container px-2 py-1 rounded border border-outline-variant/30 focus:outline-none focus:border-primary"
                />
                <button
                    onclick={save_schedule}
                    disabled={*busy}
                    class="p-1.5 rounded text-on-surface-variant hover:text-primary hover:bg-primary-container/10 transition-colors cursor-pointer disabled:opacity-50"
                    title="Save schedule"
                >
                    <span class="material-symbols-outlined text-[18px]">{"save"}</span>
                </button>
            </div>

            <div class="col-span-2">
                <span class={classes!(
                    "px-2", "py-0.5", "rounded", "font-label-xs", "text-label-xs", "font-bold",
                    status_class(props.cron.last_status.as_deref())
                )}>
                    {props.cron.last_status.clone().unwrap_or_else(|| "never".to_string())}
                </span>
                <p class="font-label-xs text-label-xs text-outline mt-1" title={format!("Last run: {last} · Next run: {next}")}>
                    {last}
                </p>
            </div>

            <div class="col-span-3 text-right">
                <button
                    onclick={toggle}
                    disabled={*busy}
                    class={classes!(
                        "px-2", "py-1", "rounded", "font-label-xs", "text-label-xs", "font-bold",
                        "transition-colors", "cursor-pointer", "disabled:opacity-50",
                        if props.cron.enabled { "bg-surface-container text-on-surface-variant hover:bg-outline-variant" } else { "bg-green-50 text-green-700 hover:bg-green-100" }
                    )}
                >
                    {if props.cron.enabled { "Disable" } else { "Enable" }}
                </button>
                <button
                    onclick={run_now}
                    disabled={*busy}
                    class="ml-2 p-1.5 rounded text-on-surface-variant hover:text-primary hover:bg-primary-container/10 active:scale-90 transition-all cursor-pointer disabled:opacity-50"
                    title="Run now"
                >
                    <span class={classes!("material-symbols-outlined", if *busy { "animate-spin" } else { "" })}>
                        {if *busy { "sync" } else { "play_arrow" }}
                    </span>
                </button>
            </div>
        </div>
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

fn status_class(status: Option<&str>) -> &'static str {
    match status {
        Some("succeeded") => "bg-green-100 text-green-800",
        Some("failed") => "bg-red-100 text-red-800",
        Some("running") => "bg-blue-100 text-blue-800",
        Some("queued") => "bg-yellow-100 text-yellow-800",
        _ => "bg-surface-container text-on-surface-variant",
    }
}
