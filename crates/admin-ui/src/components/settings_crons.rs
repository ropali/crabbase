use std::collections::HashSet;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
struct CronJob {
    id: &'static str,
    schedule: &'static str,
}

#[function_component(SettingsCrons)]
pub fn settings_crons() -> Html {
    let running_jobs = use_state(HashSet::<String>::new);
    let completed_jobs = use_state(HashSet::<String>::new);

    let jobs = vec![
        CronJob {
            id: "__pbDBOptimize__",
            schedule: "0 0 * * *",
        },
        CronJob {
            id: "__pbMFACleanup__",
            schedule: "0 * * * *",
        },
        CronJob {
            id: "__pbOTPCleanup__",
            schedule: "0 * * * *",
        },
        CronJob {
            id: "__pbLogsCleanup__",
            schedule: "0 */6 * * *",
        },
        CronJob {
            id: "__pbRateLimitersCleanup__",
            schedule: "2 * * * *",
        },
    ];

    let run_job = |job_id: &'static str| {
        let running_jobs = running_jobs.clone();
        let completed_jobs = completed_jobs.clone();
        Callback::from(move |_| {
            let mut running = (*running_jobs).clone();
            running.insert(job_id.to_string());
            running_jobs.set(running);

            let running_jobs = running_jobs.clone();
            let completed_jobs = completed_jobs.clone();
            let job_id_str = job_id.to_string();

            gloo_timers::callback::Timeout::new(1500, move || {
                let mut running = (*running_jobs).clone();
                running.remove(&job_id_str);
                running_jobs.set(running);

                let mut completed = (*completed_jobs).clone();
                completed.insert(job_id_str.clone());
                completed_jobs.set(completed);

                let completed_jobs = completed_jobs.clone();
                gloo_timers::callback::Timeout::new(2000, move || {
                    let mut completed = (*completed_jobs).clone();
                    completed.remove(&job_id_str);
                    completed_jobs.set(completed);
                })
                .forget();
            })
            .forget();
        })
    };

    html! {
        <main class="flex-1 overflow-y-auto bg-surface p-margin_page">
            <div class="max-w-4xl mx-auto space-y-8">
                /* Breadcrumbs */
                <nav class="flex items-center gap-2 mb-4 text-on-surface-variant font-label-xs text-label-xs">
                    <span class="hover:text-primary cursor-pointer transition-colors">{"Settings"}</span>
                    <span class="material-symbols-outlined text-[14px]">{"chevron_right"}</span>
                    <span class="font-bold text-on-surface">{"Crons"}</span>
                </nav>

                /* Bento Style Header */
                <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-8">
                    <div class="md:col-span-2 bg-surface-container-lowest border border-outline-variant p-6 rounded-xl flex flex-col justify-between shadow-sm">
                        <div>
                            <div class="flex items-center justify-between mb-2">
                                <h1 class="font-headline-lg text-headline-lg text-on-surface">{"Scheduled Tasks"}</h1>
                                <button class="p-2 hover:bg-surface-container rounded-full transition-all group cursor-pointer">
                                    <span class="material-symbols-outlined text-primary group-active:rotate-180 transition-transform duration-500">{"refresh"}</span>
                                </button>
                            </div>
                            <p class="font-body-md text-body-md text-on-surface-variant max-w-lg">
                                {"Manage and monitor system-level background jobs. These tasks ensure database health, log rotation, and security cleanup protocols."}
                            </p>
                        </div>
                        <div class="mt-6 flex items-center gap-4">
                            <div class="flex items-center gap-2 px-3 py-1 bg-surface-container rounded-full">
                                <span class="w-2 h-2 rounded-full bg-green-500 animate-pulse"></span>
                                <span class="text-label-xs font-label-xs text-on-surface-variant">{"Scheduler: Active"}</span>
                            </div>
                            <div class="text-label-xs font-label-xs text-on-surface-variant">{"Next run in: "} <span class="font-bold text-on-surface">{"12m 4s"}</span></div>
                        </div>
                    </div>

                    <div class="bg-primary-container p-6 rounded-xl text-on-primary-container relative overflow-hidden flex flex-col justify-between shadow-sm">
                        <div class="z-10">
                            <span class="material-symbols-outlined text-white/70 text-[32px] mb-2" style="font-variation-settings: 'FILL' 1;">{"terminal"}</span>
                            <h3 class="font-headline-md text-headline-md mb-1 font-bold text-white">{"Total Jobs"}</h3>
                            <p class="text-[40px] font-bold leading-none text-white">{"05"}</p>
                        </div>
                        <p class="text-label-xs font-label-xs text-white/80 z-10 font-medium">{"All tasks reporting healthy status"}</p>
                        <div class="absolute -right-4 -bottom-4 w-32 h-32 border-[20px] border-white/10 rounded-full"></div>
                    </div>
                </div>

                /* Cron List Table */
                <div class="bg-surface-container-lowest border border-outline-variant rounded-xl overflow-hidden shadow-sm">
                    <div class="bg-surface-container-low px-cell_padding_h py-3 border-b border-outline-variant grid grid-cols-12 items-center">
                        <div class="col-span-6 font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Job Identifier"}</div>
                        <div class="col-span-4 font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Schedule (Cron)"}</div>
                        <div class="col-span-2 text-right font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Action"}</div>
                    </div>

                    <div class="divide-y divide-outline-variant/40">
                        {
                            for jobs.iter().map(|job| {
                                let is_running = running_jobs.contains(job.id);
                                let is_complete = completed_jobs.contains(job.id);

                                html! {
                                    <div class="grid grid-cols-12 items-center px-cell_padding_h py-3 hover:bg-surface-container-low transition-colors group">
                                        <div class="col-span-6 flex items-center gap-3">
                                            <span class="font-code-md text-code-md font-bold text-on-surface">{job.id}</span>
                                        </div>
                                        <div class="col-span-4">
                                            <span class="font-code-md text-code-md text-on-surface-variant bg-surface-container px-2 py-0.5 rounded border border-outline-variant/30">
                                                {job.schedule}
                                            </span>
                                        </div>
                                        <div class="col-span-2 text-right">
                                            <button
                                                onclick={run_job(job.id)}
                                                disabled={is_running}
                                                class={classes!(
                                                    "p-2", "rounded", "transition-all", "cursor-pointer",
                                                    if is_running {
                                                        "text-primary bg-primary-container/10"
                                                    } else if is_complete {
                                                        "text-green-700 bg-green-100"
                                                    } else {
                                                        "text-on-surface-variant hover:text-primary hover:bg-primary-container/10 active:scale-90"
                                                    }
                                                )}
                                                title="Run task manually"
                                            >
                                                <span class={classes!(
                                                    "material-symbols-outlined",
                                                    if is_running { "animate-spin" } else { "" }
                                                )}>
                                                    {
                                                        if is_running {
                                                            "sync"
                                                        } else if is_complete {
                                                            "check_circle"
                                                        } else {
                                                            "play_arrow"
                                                        }
                                                    }
                                                </span>
                                            </button>
                                        </div>
                                    </div>
                                }
                            })
                        }
                    </div>
                </div>

                /* Informational Footer Info */
                <div class="mt-8 p-6 bg-surface-container rounded-lg border border-dashed border-outline-variant flex items-start gap-4">
                    <span class="material-symbols-outlined text-primary">{"info"}</span>
                    <div>
                        <p class="font-body-sm text-body-sm text-on-surface-variant">
                            {"App cron jobs can be registered only programmatically with "}
                            <span class="font-code-md text-primary font-bold hover:underline cursor-pointer">{"Go"}</span>
                            {" or "}
                            <span class="font-code-md text-primary font-bold hover:underline cursor-pointer">{"JavaScript"}</span>
                            {". The manual trigger button is used for testing purposes and will not disrupt the regular schedule."}
                        </p>
                    </div>
                </div>
            </div>
        </main>
    }
}
