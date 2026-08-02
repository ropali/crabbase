use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub struct LogItem {
    pub id: &'static str,
    pub level: &'static str,
    pub level_color: &'static str,
    pub method: Option<&'static str>,
    pub path_or_msg: &'static str,
    pub status: Option<u16>,
    pub duration: &'static str,
    pub actor: &'static str,
    pub timestamp: &'static str,
    pub is_highlighted: bool,
}

#[derive(Clone, PartialEq)]
struct ChartBucket {
    height_pct: &'static str,
    events: u32,
    time: &'static str,
}

#[function_component(ActivityLogs)]
pub fn activity_logs() -> Html {
    let search_query = use_state(|| String::new());
    let selected_level = use_state(|| "ALL".to_string());
    let time_range = use_state(|| "1H".to_string());
    let superuser_only = use_state(|| false);
    let is_live = use_state(|| true);
    let selected_log_id = use_state(|| None::<String>);

    let logs = vec![
        LogItem {
            id: "log_01",
            level: "Error",
            level_color: "bg-primary/10 text-primary border-primary/20",
            method: Some("GET"),
            path_or_msg: "/api/v1/collections/users/undefined",
            status: Some(404),
            duration: "12ms",
            actor: "root",
            timestamp: "2024-05-24 14:22:01.032",
            is_highlighted: false,
        },
        LogItem {
            id: "log_02",
            level: "Info",
            level_color: "bg-green-100 text-green-700 border-green-200",
            method: Some("POST"),
            path_or_msg: "/api/auth/login/success",
            status: Some(200),
            duration: "45ms",
            actor: "auth:jwt",
            timestamp: "2024-05-24 14:21:58.891",
            is_highlighted: true,
        },
        LogItem {
            id: "log_03",
            level: "Debug",
            level_color: "bg-blue-100 text-blue-700 border-blue-200",
            method: None,
            path_or_msg: "Cache revalidation triggered for @crab-store",
            status: None,
            duration: "worker_02",
            actor: "L1_CACHE",
            timestamp: "2024-05-24 14:21:42.110",
            is_highlighted: false,
        },
        LogItem {
            id: "log_04",
            level: "Warn",
            level_color: "bg-yellow-100 text-yellow-700 border-yellow-200",
            method: None,
            path_or_msg: "High memory usage detected in cluster node rust-instance-01",
            status: None,
            duration: "heap:85%",
            actor: "sys",
            timestamp: "2024-05-24 14:20:15.004",
            is_highlighted: false,
        },
        LogItem {
            id: "log_05",
            level: "Info",
            level_color: "bg-green-100 text-green-700 border-green-200",
            method: Some("GET"),
            path_or_msg: "/favicon.ico",
            status: Some(200),
            duration: "2ms",
            actor: "anon",
            timestamp: "2024-05-24 14:19:59.222",
            is_highlighted: false,
        },
    ];

    let chart_buckets = vec![
        ChartBucket {
            height_pct: "h-[30%]",
            events: 124,
            time: "14:00",
        },
        ChartBucket {
            height_pct: "h-[45%]",
            events: 230,
            time: "14:02",
        },
        ChartBucket {
            height_pct: "h-[40%]",
            events: 180,
            time: "14:04",
        },
        ChartBucket {
            height_pct: "h-[60%]",
            events: 420,
            time: "14:06",
        },
        ChartBucket {
            height_pct: "h-[80%]",
            events: 890,
            time: "14:08",
        },
        ChartBucket {
            height_pct: "h-[55%]",
            events: 350,
            time: "14:10",
        },
        ChartBucket {
            height_pct: "h-[70%]",
            events: 610,
            time: "14:12",
        },
        ChartBucket {
            height_pct: "h-[90%]",
            events: 1120,
            time: "14:14",
        },
        ChartBucket {
            height_pct: "h-[65%]",
            events: 540,
            time: "14:16",
        },
        ChartBucket {
            height_pct: "h-[50%]",
            events: 310,
            time: "14:18",
        },
        ChartBucket {
            height_pct: "h-[85%]",
            events: 980,
            time: "14:20",
        },
        ChartBucket {
            height_pct: "h-[75%]",
            events: 750,
            time: "14:22",
        },
        ChartBucket {
            height_pct: "h-[40%]",
            events: 190,
            time: "14:24",
        },
        ChartBucket {
            height_pct: "h-[55%]",
            events: 380,
            time: "14:26",
        },
        ChartBucket {
            height_pct: "h-[65%]",
            events: 510,
            time: "14:28",
        },
        ChartBucket {
            height_pct: "h-[50%]",
            events: 290,
            time: "14:30",
        },
        ChartBucket {
            height_pct: "h-[80%]",
            events: 840,
            time: "14:32",
        },
        ChartBucket {
            height_pct: "h-[60%]",
            events: 460,
            time: "14:34",
        },
        ChartBucket {
            height_pct: "h-[35%]",
            events: 160,
            time: "14:36",
        },
        ChartBucket {
            height_pct: "h-[45%]",
            events: 240,
            time: "14:38",
        },
        ChartBucket {
            height_pct: "h-[30%]",
            events: 110,
            time: "14:40",
        },
        ChartBucket {
            height_pct: "h-[45%]",
            events: 250,
            time: "14:42",
        },
        ChartBucket {
            height_pct: "h-[25%]",
            events: 95,
            time: "14:44",
        },
        ChartBucket {
            height_pct: "h-[40%]",
            events: 210,
            time: "14:46",
        },
        ChartBucket {
            height_pct: "h-[50%]",
            events: 340,
            time: "14:48",
        },
        ChartBucket {
            height_pct: "h-[35%]",
            events: 175,
            time: "14:50",
        },
        ChartBucket {
            height_pct: "h-[45%]",
            events: 260,
            time: "14:52",
        },
        ChartBucket {
            height_pct: "h-[60%]",
            events: 480,
            time: "14:54",
        },
        ChartBucket {
            height_pct: "h-[40%]",
            events: 220,
            time: "14:56",
        },
        ChartBucket {
            height_pct: "h-[30%]",
            events: 130,
            time: "14:58",
        },
    ];

    let toggle_live = {
        let is_live = is_live.clone();
        Callback::from(move |_| {
            is_live.set(!*is_live);
        })
    };

    let toggle_superuser = {
        let superuser_only = superuser_only.clone();
        Callback::from(move |_| {
            superuser_only.set(!*superuser_only);
        })
    };

    // Filter logs dynamically based on search query and level dropdown selection
    let filtered_logs: Vec<LogItem> = logs
        .iter()
        .filter(|item| {
            let matches_level = match selected_level.as_str() {
                "ALL" => true,
                lvl => item.level.eq_ignore_ascii_case(lvl),
            };

            let q = search_query.to_lowercase();
            let matches_query = q.is_empty()
                || item.path_or_msg.to_lowercase().contains(&q)
                || item.level.to_lowercase().contains(&q)
                || item.actor.to_lowercase().contains(&q)
                || item.timestamp.to_lowercase().contains(&q)
                || item
                    .status
                    .map(|s| s.to_string())
                    .unwrap_or_default()
                    .contains(&q);

            matches_level && matches_query
        })
        .cloned()
        .collect();

    html! {
        <div class="flex flex-col flex-1 h-full overflow-hidden bg-surface">
            /* Sticky Sub-header Bar (z-20 so chart tooltips at z-50 appear on top) */
            <div class="sticky top-0 z-20 bg-surface border-b border-outline-variant px-gutter py-3 flex flex-col md:flex-row gap-4 items-center justify-between shadow-sm">
                <div class="flex items-center gap-3 w-full md:w-auto flex-wrap">
                    /* Log Search Input */
                    <div class="relative w-full md:w-80">
                        <span class="material-symbols-outlined absolute left-3 top-1/2 -translate-y-1/2 text-on-surface-variant text-[18px]">{"search"}</span>
                        <input
                            class="w-full bg-surface-container-low border border-outline-variant rounded-lg pl-10 pr-4 py-1.5 font-body-sm text-body-sm focus:ring-2 focus:ring-primary focus:border-primary outline-none transition-all"
                            placeholder="Filter logs by message, status code, or ID..."
                            type="text"
                            value={(*search_query).clone()}
                            oninput={
                                let search_query = search_query.clone();
                                Callback::from(move |e: InputEvent| {
                                    if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                        search_query.set(target.value());
                                    }
                                })
                            }
                        />
                    </div>

                    /* Level Filter Selector Dropdown */
                    <div class="relative flex items-center">
                        <select
                            value={(*selected_level).clone()}
                            onchange={
                                let selected_level = selected_level.clone();
                                Callback::from(move |e: Event| {
                                    if let Some(target) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                                        selected_level.set(target.value());
                                    }
                                })
                            }
                            class="bg-surface-container-low border border-outline-variant rounded-lg px-3 py-1.5 font-body-sm text-body-sm text-on-surface focus:ring-2 focus:ring-primary focus:border-primary outline-none cursor-pointer font-bold"
                        >
                            <option value="ALL">{"All Levels"}</option>
                            <option value="Error">{"Error"}</option>
                            <option value="Warn">{"Warn"}</option>
                            <option value="Info">{"Info"}</option>
                            <option value="Debug">{"Debug"}</option>
                        </select>
                    </div>

                    <div class="h-6 w-[1px] bg-outline-variant hidden md:block mx-1"></div>

                    /* Superuser filter checkbox */
                    <label class="hidden md:flex items-center gap-2 cursor-pointer group select-none">
                        <input
                            type="checkbox"
                            checked={*superuser_only}
                            onclick={toggle_superuser}
                            class="w-4 h-4 text-primary border-outline-variant rounded focus:ring-primary cursor-pointer"
                        />
                        <span class="font-body-sm text-body-sm text-on-surface-variant group-hover:text-on-surface transition-colors whitespace-nowrap">
                            {"Superuser logs"}
                        </span>
                    </label>
                </div>

                <div class="flex items-center gap-4">
                    /* Time Range Switcher */
                    <div class="flex bg-surface-container-low border border-outline-variant rounded-lg p-1">
                        {
                            for ["1H", "6H", "24H"].iter().map(|range| {
                                let range_str = range.to_string();
                                let is_active = *time_range == range_str;
                                let time_range = time_range.clone();
                                html! {
                                    <button
                                        onclick={move |_| time_range.set(range_str.clone())}
                                        class={classes!(
                                            "px-3", "py-1", "rounded-md", "font-label-xs", "text-label-xs", "font-bold", "transition-all", "cursor-pointer",
                                            if is_active { "bg-surface shadow-sm text-primary" } else { "text-on-surface-variant hover:text-on-surface" }
                                        )}
                                    >
                                        {range}
                                    </button>
                                }
                            })
                        }
                    </div>

                    <div class="h-6 w-[1px] bg-outline-variant hidden md:block"></div>

                    /* Live Action Toggle Button */
                    <button
                        onclick={toggle_live}
                        class={classes!(
                            "px-3", "py-1.5", "rounded-lg", "font-bold", "flex", "items-center", "gap-2",
                            "transition-all", "font-body-sm", "text-body-sm", "shadow-sm", "cursor-pointer",
                            if *is_live { "bg-primary text-on-primary" } else { "bg-surface-container text-on-surface-variant border border-outline-variant" }
                        )}
                    >
                        <span class={classes!("material-symbols-outlined", "text-[18px]", if *is_live { "animate-spin" } else { "" })}>
                            {if *is_live { "refresh" } else { "pause" }}
                        </span>
                        {if *is_live { "Live" } else { "Paused" }}
                    </button>
                </div>
            </div>

            /* Telemetry Strip with Increased Height (h-28) & High Z-Index Layering (z-30) */
            <div class="bg-surface border-b border-outline-variant h-28 relative flex items-end shrink-0 z-30">
                <div class="absolute inset-0 bg-gradient-to-b from-primary/10 to-transparent pointer-events-none rounded-b"></div>
                <div class="flex w-full items-end gap-1 px-4 h-20">
                    {
                        for chart_buckets.iter().map(|bucket| {
                            let tooltip_text = format!("{} events at {}", bucket.events, bucket.time);
                            html! {
                                <div class={classes!(
                                    "flex-1", "bg-primary/30", "rounded-t-sm", "transition-all", "duration-150",
                                    "hover:bg-primary", "hover:scale-y-110", "origin-bottom", "cursor-pointer",
                                    "group", "relative", bucket.height_pct
                                )}>
                                    /* Hover Tooltip (z-50 shadow-xl for crystal clear visibility over sub-header) */
                                    <div class="opacity-0 group-hover:opacity-100 absolute -top-9 left-1/2 -translate-x-1/2 bg-inverse-surface text-inverse-on-surface px-2.5 py-1 rounded-md text-[11px] font-bold whitespace-nowrap shadow-xl font-code-md z-50 pointer-events-none transition-all duration-150 border border-outline/20">
                                        {tooltip_text}
                                    </div>
                                </div>
                            }
                        })
                    }
                </div>
                <div class="absolute top-2 left-4 flex gap-6 text-label-xs uppercase tracking-widest font-bold text-on-surface-variant pointer-events-none z-10">
                    <span>{"128,492 Events"}</span>
                    <span class="text-primary font-extrabold">{"14 Errors (24h)"}</span>
                    <span>{"Avg 42ms"}</span>
                </div>
            </div>

            /* Log Table Section */
            <main class="flex-1 overflow-y-auto bg-surface-container-lowest">
                <div class="w-full h-full flex flex-col justify-between">
                    <div class="overflow-x-auto">
                        <table class="w-full text-left border-collapse">
                            <thead class="bg-surface-dim/30 sticky top-0 border-b border-outline-variant z-10 backdrop-blur-sm">
                                <tr>
                                    <th class="py-cell_padding_v px-cell_padding_h w-10">
                                        <input type="checkbox" class="w-4 h-4 text-primary border-outline-variant rounded focus:ring-primary cursor-pointer" />
                                    </th>
                                    <th class="py-cell_padding_v px-cell_padding_h font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Level"}</th>
                                    <th class="py-cell_padding_v px-cell_padding_h font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Event Message"}</th>
                                    <th class="py-cell_padding_v px-cell_padding_h font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Metadata"}</th>
                                    <th class="py-cell_padding_v px-cell_padding_h font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Timestamp"}</th>
                                </tr>
                            </thead>
                            <tbody class="divide-y divide-outline-variant/40">
                                {
                                    if filtered_logs.is_empty() {
                                        html! {
                                            <tr>
                                                <td class="py-12 text-center" colspan="5">
                                                    <div class="opacity-40 flex flex-col items-center">
                                                        <span class="material-symbols-outlined text-[48px] mb-2 text-outline">{"search_off"}</span>
                                                        <p class="font-body-sm text-body-sm italic">{"No activity logs match your filter criteria."}</p>
                                                    </div>
                                                </td>
                                            </tr>
                                        }
                                    } else {
                                        html! {
                                            {
                                                for filtered_logs.iter().map(|item| {
                                                    let item_id = item.id.to_string();
                                                    let is_selected = (*selected_log_id).as_ref() == Some(&item_id);
                                                    let selected_log_id = selected_log_id.clone();

                                                    html! {
                                                        <tr
                                                            onclick={move |_| selected_log_id.set(Some(item_id.clone()))}
                                                            class={classes!(
                                                                "log-row", "hover:bg-surface-container-low", "transition-colors", "group", "cursor-pointer",
                                                                if item.is_highlighted { "border-l-4 border-l-secondary-container" } else { "" },
                                                                if is_selected { "bg-surface-container" } else { "" }
                                                            )}
                                                        >
                                                            <td class="py-cell_padding_v px-cell_padding_h">
                                                                <input type="checkbox" class="w-4 h-4 text-primary border-outline-variant rounded focus:ring-primary cursor-pointer" />
                                                            </td>
                                                            <td class="py-cell_padding_v px-cell_padding_h">
                                                                <span class={classes!("px-2", "py-0.5", "rounded-full", "text-[10px]", "font-bold", "uppercase", "tracking-tight", "border", item.level_color)}>
                                                                    {item.level}
                                                                </span>
                                                            </td>
                                                            <td class="py-cell_padding_v px-cell_padding_h font-code-md text-code-md text-on-background">
                                                                {
                                                                    if let Some(m) = item.method {
                                                                        html! {
                                                                            <>
                                                                                <span class={if m == "GET" { "text-primary font-bold mr-2" } else { "text-on-tertiary-fixed-variant font-bold mr-2" }}>
                                                                                    {m}
                                                                                </span>
                                                                                {item.path_or_msg}
                                                                            </>
                                                                        }
                                                                    } else {
                                                                        html! { item.path_or_msg }
                                                                    }
                                                                }
                                                            </td>
                                                            <td class="py-cell_padding_v px-cell_padding_h">
                                                                <div class="flex gap-2">
                                                                    {
                                                                        if let Some(st) = item.status {
                                                                            html! {
                                                                                <span class={classes!(
                                                                                    "px-1.5", "py-0.5", "rounded", "text-[10px]", "font-medium", "border", "border-outline-variant",
                                                                                    if st >= 400 { "bg-error-container text-on-error-container" } else { "bg-surface-container text-on-surface-variant" }
                                                                                )}>
                                                                                    {st}
                                                                                </span>
                                                                            }
                                                                        } else {
                                                                            html! {}
                                                                        }
                                                                    }
                                                                    <span class="px-1.5 py-0.5 rounded bg-surface-container text-on-surface-variant text-[10px] font-medium border border-outline-variant">
                                                                        {item.duration}
                                                                    </span>
                                                                    <span class="px-1.5 py-0.5 rounded bg-surface-container text-on-surface-variant text-[10px] font-medium border border-outline-variant">
                                                                        {item.actor}
                                                                    </span>
                                                                </div>
                                                            </td>
                                                            <td class="py-cell_padding_v px-cell_padding_h text-on-surface-variant whitespace-nowrap font-code-md text-code-md">
                                                                {item.timestamp}
                                                            </td>
                                                        </tr>
                                                    }
                                                })
                                            }
                                        }
                                    }
                                }
                            </tbody>
                        </table>
                    </div>

                    /* Footer Pagination Bar */
                    <div class="p-3 bg-surface border-t border-outline-variant flex justify-between items-center shrink-0">
                        <div class="flex items-center gap-4">
                            <span class="font-label-xs text-label-xs text-on-surface-variant uppercase font-bold tracking-widest">
                                {format!("Showing {} of 12k logs", filtered_logs.len())}
                            </span>
                        </div>
                        <div class="flex justify-center flex-1">
                            <button class="bg-surface border border-outline-variant px-6 py-1.5 rounded-lg font-bold text-primary hover:bg-surface-container transition-all font-body-sm text-body-sm shadow-sm flex items-center gap-2 cursor-pointer">
                                <span class="material-symbols-outlined text-[18px]">{"expand_more"}</span>
                                {"Load more logs"}
                            </button>
                        </div>
                        <div class="flex items-center gap-2 font-label-xs text-label-xs text-on-surface-variant uppercase font-bold tracking-widest">
                            {"Rows:"}
                            <select class="bg-transparent border-none font-label-xs text-label-xs focus:ring-0 cursor-pointer text-primary font-bold">
                                <option>{"50"}</option>
                                <option>{"100"}</option>
                                <option>{"250"}</option>
                            </select>
                        </div>
                    </div>
                </div>
            </main>
        </div>
    }
}
