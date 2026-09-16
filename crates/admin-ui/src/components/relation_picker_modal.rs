use serde_json::Value;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::{api::client::ApiClient, models::collection::Record};

#[derive(Properties, PartialEq, Clone)]
pub struct RelationPickerModalProps {
    pub field_name: String,
    #[prop_or_default]
    pub target_collection: Option<String>,
    #[prop_or_default]
    pub current_value: String,
    pub on_select: Callback<String>,
    pub on_close: Callback<()>,
}

fn extract_record_summary(record: &Record) -> (Option<String>, Vec<(String, String)>) {
    let mut primary_title = None;
    let mut previews = Vec::new();

    if let Value::Object(map) = &record.data {
        // Look for common identifying keys
        let title_keys = [
            "title",
            "name",
            "email",
            "username",
            "label",
            "slug",
            "code",
            "heading",
            "description",
        ];
        for key in title_keys {
            if let Some(val) = map.get(key) {
                if let Some(s) = val.as_str() {
                    if !s.trim().is_empty() {
                        primary_title = Some(s.to_string());
                        break;
                    }
                }
            }
        }

        // Collect other field previews (up to 4)
        for (k, v) in map.iter() {
            if matches!(k.as_str(), "password" | "token_key" | "token") {
                continue;
            }
            if let Some(ref t) = primary_title {
                if let Some(s) = v.as_str() {
                    if s == t {
                        continue;
                    }
                }
            }
            let display_val = match v {
                Value::String(s) => {
                    if s.len() > 35 {
                        format!("{}...", &s[..32])
                    } else {
                        s.clone()
                    }
                }
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "null".to_string(),
                Value::Array(a) => format!("[{} items]", a.len()),
                Value::Object(_) => "{...}".to_string(),
            };
            if !display_val.is_empty() && previews.len() < 4 {
                previews.push((k.clone(), display_val));
            }
        }
    }

    (primary_title, previews)
}

#[function_component(RelationPickerModal)]
pub fn relation_picker_modal(props: &RelationPickerModalProps) -> Html {
    let collection_name = props.target_collection.clone().unwrap_or_default();

    let records = use_state(Vec::<Record>::new);
    let total_records = use_state(|| 0usize);
    let page = use_state(|| 1usize);
    let per_page = 50usize;
    let is_loading = use_state(|| false);
    let error_msg = use_state(|| None::<String>);
    let search_query = use_state(String::new);
    let refresh_trigger = use_state(|| 0u32);

    // Fetch records when page or refresh_trigger changes
    {
        let records = records.clone();
        let total_records = total_records.clone();
        let is_loading = is_loading.clone();
        let error_msg = error_msg.clone();
        let col_name = collection_name.clone();
        let current_page = *page;
        let refresh_val = *refresh_trigger;

        use_effect_with(
            (col_name.clone(), current_page, refresh_val),
            move |(col, p, _)| {
                if col.is_empty() {
                    records.set(Vec::new());
                    total_records.set(0);
                } else {
                    let col = col.clone();
                    let p = *p;
                    let records = records.clone();
                    let total_records = total_records.clone();
                    let is_loading = is_loading.clone();
                    let error_msg = error_msg.clone();

                    is_loading.set(true);
                    error_msg.set(None);

                    wasm_bindgen_futures::spawn_local(async move {
                        let client = ApiClient::default();
                        match client
                            .get_records(&col, Some(p), Some(50), None, None)
                            .await
                        {
                            Ok(res) => {
                                records.set(res.items);
                                total_records.set(res.total);
                                error_msg.set(None);
                            }
                            Err(e) => {
                                error_msg.set(Some(format!("Failed to load records: {}", e)));
                                records.set(Vec::new());
                                total_records.set(0);
                            }
                        }
                        is_loading.set(false);
                    });
                }

                || ()
            },
        );
    }

    // Filter records client-side across all data fields, ID, and timestamps
    let filtered_records: Vec<Record> = {
        let q = search_query.trim().to_lowercase();
        if q.is_empty() {
            (*records).clone()
        } else {
            records
                .iter()
                .filter(|r| {
                    if r.id.to_lowercase().contains(&q) {
                        return true;
                    }
                    if r.created.to_lowercase().contains(&q) {
                        return true;
                    }
                    if r.updated.to_lowercase().contains(&q) {
                        return true;
                    }
                    if let Value::Object(map) = &r.data {
                        for (k, v) in map {
                            if k.to_lowercase().contains(&q) {
                                return true;
                            }
                            match v {
                                Value::String(s) => {
                                    if s.to_lowercase().contains(&q) {
                                        return true;
                                    }
                                }
                                Value::Number(n) => {
                                    if n.to_string().contains(&q) {
                                        return true;
                                    }
                                }
                                Value::Bool(b) => {
                                    if b.to_string().contains(&q) {
                                        return true;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    false
                })
                .cloned()
                .collect()
        }
    };

    let on_backdrop_click = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| {
            on_close.emit(());
        })
    };

    let on_modal_click = Callback::from(|e: MouseEvent| {
        e.stop_propagation();
    });

    let on_close_btn = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| {
            on_close.emit(());
        })
    };

    let on_search_input = {
        let search_query = search_query.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            search_query.set(input.value());
        })
    };

    let on_clear_search = {
        let search_query = search_query.clone();
        Callback::from(move |_| {
            search_query.set(String::new());
        })
    };

    let on_refresh = {
        let refresh_trigger = refresh_trigger.clone();
        Callback::from(move |_| {
            refresh_trigger.set(*refresh_trigger + 1);
        })
    };

    let total_pages = ((*total_records + per_page - 1) / per_page).max(1);
    let can_prev = *page > 1;
    let can_next = *page < total_pages;

    let on_prev_page = {
        let page = page.clone();
        Callback::from(move |_| {
            if *page > 1 {
                page.set(*page - 1);
            }
        })
    };

    let on_next_page = {
        let page = page.clone();
        let total_pages = total_pages;
        Callback::from(move |_| {
            if *page < total_pages {
                page.set(*page + 1);
            }
        })
    };

    html! {
        <div
            onclick={on_backdrop_click}
            class="fixed inset-0 bg-inverse-surface/40 backdrop-blur-sm z-[60] flex items-center justify-center p-4 animate-fade-in"
        >
            <div
                onclick={on_modal_click}
                class="bg-surface border border-outline-variant rounded-2xl shadow-2xl w-full max-w-3xl flex flex-col max-h-[85vh] overflow-hidden animate-scale-in"
            >
                // Modal Header
                <div class="px-6 py-4 border-b border-outline-variant flex items-center justify-between bg-surface-container-low/50 shrink-0">
                    <div class="flex items-center gap-3">
                        <div class="w-9 h-9 rounded-xl bg-primary/10 text-primary flex items-center justify-center">
                            <span class="material-symbols-outlined text-xl">{"link"}</span>
                        </div>
                        <div>
                            <div class="flex items-center gap-2">
                                <h3 class="font-bold text-on-surface text-lg">{"Select Related Record"}</h3>
                                <span class="bg-primary/10 text-primary px-2 py-0.5 rounded-full font-label-xs text-[10px] uppercase font-bold tracking-wider">
                                    {&props.field_name}
                                </span>
                            </div>
                            <p class="font-label-xs text-label-xs text-on-surface-variant">
                                {format!("Choose a record from '{}' to link to field '{}'", collection_name, props.field_name)}
                            </p>
                        </div>
                    </div>
                    <button
                        type="button"
                        onclick={on_close_btn.clone()}
                        class="p-2 hover:bg-surface-container-high rounded-full transition-colors text-on-surface-variant hover:text-on-surface"
                        title="Close modal"
                    >
                        <span class="material-symbols-outlined text-lg">{"close"}</span>
                    </button>
                </div>

                // Toolbar: Collection Info & Search Bar
                <div class="p-4 border-b border-outline-variant bg-surface-container-lowest flex flex-col gap-3 shrink-0">
                    <div class="flex items-center justify-between gap-3">
                        // Related collection badge (no dropdown)
                        <div class="flex items-center gap-2">
                            <span class="font-label-xs text-label-xs text-on-surface-variant font-medium flex items-center gap-1">
                                <span class="material-symbols-outlined text-[14px]">{"database"}</span>
                                <span>{"Collection:"}</span>
                            </span>
                            <span class="font-mono text-xs font-bold text-primary px-2.5 py-1 bg-surface-container rounded-lg border border-outline-variant/50">
                                {collection_name.clone()}
                            </span>
                        </div>

                        // Refresh button
                        <button
                            type="button"
                            onclick={on_refresh.clone()}
                            class="p-1.5 text-on-surface-variant hover:text-primary hover:bg-surface-container-high rounded-lg transition-colors flex items-center gap-1 text-xs"
                            title="Reload records"
                        >
                            <span class="material-symbols-outlined text-sm">{"refresh"}</span>
                            <span>{"Refresh"}</span>
                        </button>
                    </div>

                    // Search input
                    <div class="relative">
                        <span class="material-symbols-outlined absolute left-3 top-1/2 -translate-y-1/2 text-on-surface-variant/60 text-base">
                            {"search"}
                        </span>
                        <input
                            type="text"
                            placeholder={format!("Search {} records by ID, title, or field values...", collection_name)}
                            value={(*search_query).clone()}
                            oninput={on_search_input}
                            class="w-full bg-surface-container-low border border-outline-variant rounded-lg pl-9 pr-9 py-2 text-body-sm text-on-surface placeholder:text-on-surface-variant/50 focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all"
                        />
                        {
                            if !search_query.is_empty() {
                                html! {
                                    <button
                                        type="button"
                                        onclick={on_clear_search.clone()}
                                        class="absolute right-2.5 top-1/2 -translate-y-1/2 text-on-surface-variant/60 hover:text-on-surface hover:bg-surface-container-high rounded p-0.5 transition-colors"
                                        title="Clear search"
                                    >
                                        <span class="material-symbols-outlined text-sm">{"close"}</span>
                                    </button>
                                }
                            } else {
                                html! {}
                            }
                        }
                    </div>

                    // Search / Count indicator
                    <div class="flex items-center justify-between text-[11px] text-on-surface-variant/70">
                        <div>
                            {
                                if !search_query.is_empty() {
                                    html! {
                                        <span>
                                            {format!("Showing {} matches for \"{}\"", filtered_records.len(), *search_query)}
                                        </span>
                                    }
                                } else {
                                    html! {
                                        <span>{format!("{} records found", *total_records)}</span>
                                    }
                                }
                            }
                        </div>
                        {
                            if !props.current_value.is_empty() {
                                html! {
                                    <div class="flex items-center gap-1">
                                        <span>{"Current selection:"}</span>
                                        <span class="font-mono font-medium text-primary bg-surface-container px-1.5 py-0.2 rounded border border-outline-variant/40">
                                            {props.current_value.clone()}
                                        </span>
                                    </div>
                                }
                            } else {
                                html! {}
                            }
                        }
                    </div>
                </div>

                // Records List (Scrollable)
                <div class="flex-1 overflow-y-auto min-h-[260px] max-h-[420px] divide-y divide-outline-variant/30 custom-scrollbar bg-surface">
                    {
                        if *is_loading {
                            html! {
                                <div class="p-12 flex flex-col items-center justify-center text-on-surface-variant gap-3">
                                    <span class="material-symbols-outlined text-3xl animate-spin text-primary">{"progress_activity"}</span>
                                    <p class="text-xs font-medium">{"Loading records..."}</p>
                                </div>
                            }
                        } else if let Some(err) = &*error_msg {
                            html! {
                                <div class="p-8 flex flex-col items-center justify-center text-center gap-2">
                                    <span class="material-symbols-outlined text-error text-3xl">{"error"}</span>
                                    <p class="text-xs text-error font-medium">{err}</p>
                                    <button
                                        type="button"
                                        onclick={on_refresh}
                                        class="mt-2 px-3 py-1.5 bg-surface-container hover:bg-surface-container-high text-xs rounded-lg font-semibold transition-colors"
                                    >
                                        {"Try Again"}
                                    </button>
                                </div>
                            }
                        } else if filtered_records.is_empty() {
                            if !search_query.is_empty() {
                                html! {
                                    <div class="p-12 flex flex-col items-center justify-center text-center gap-2 text-on-surface-variant">
                                        <span class="material-symbols-outlined text-3xl text-outline">{"search_off"}</span>
                                        <p class="text-xs font-medium">{format!("No records match \"{}\"", *search_query)}</p>
                                        <button
                                            type="button"
                                            onclick={on_clear_search}
                                            class="text-xs text-primary hover:underline font-semibold"
                                        >
                                            {"Clear search filter"}
                                        </button>
                                    </div>
                                }
                            } else {
                                html! {
                                    <div class="p-12 flex flex-col items-center justify-center text-center gap-2 text-on-surface-variant">
                                        <span class="material-symbols-outlined text-3xl text-outline">{"inventory_2"}</span>
                                        <p class="text-xs font-medium">{format!("No records found in collection \"{}\"", collection_name)}</p>
                                        <p class="text-[11px] text-on-surface-variant/60">{"Create records in that collection first to link them."}</p>
                                    </div>
                                }
                            }
                        } else {
                            html! {
                                <>
                                    {
                                        filtered_records.iter().map(|record| {
                                            let is_selected = !props.current_value.is_empty() && record.id == props.current_value;
                                            let (primary_title, previews) = extract_record_summary(record);

                                            let on_select_record = {
                                                let on_select = props.on_select.clone();
                                                let record_id = record.id.clone();
                                                Callback::from(move |_| {
                                                    on_select.emit(record_id.clone());
                                                })
                                            };

                                            let row_bg = if is_selected {
                                                "bg-primary/5 hover:bg-primary/10 border-l-4 border-l-primary"
                                            } else {
                                                "hover:bg-surface-container-high/60 border-l-4 border-l-transparent"
                                            };

                                            html! {
                                                <div
                                                    key={record.id.clone()}
                                                    onclick={on_select_record.clone()}
                                                    class={classes!(
                                                        "p-3.5",
                                                        "flex",
                                                        "items-center",
                                                        "justify-between",
                                                        "gap-4",
                                                        "cursor-pointer",
                                                        "transition-all",
                                                        "group",
                                                        row_bg,
                                                    )}
                                                >
                                                    <div class="flex-1 min-w-0">
                                                        <div class="flex items-center gap-2 flex-wrap mb-1">
                                                            {
                                                                if let Some(ref title) = primary_title {
                                                                    html! {
                                                                        <span class="font-bold text-on-surface text-sm group-hover:text-primary transition-colors">
                                                                            {title}
                                                                        </span>
                                                                    }
                                                                } else {
                                                                    html! {}
                                                                }
                                                            }
                                                            <span class="font-mono text-xs bg-surface-container px-2 py-0.5 rounded text-primary font-medium border border-outline-variant/40">
                                                                {record.id.clone()}
                                                            </span>
                                                            {
                                                                if is_selected {
                                                                    html! {
                                                                        <span class="bg-primary/15 text-primary text-[11px] font-semibold px-2 py-0.5 rounded-full flex items-center gap-0.5">
                                                                            <span class="material-symbols-outlined text-[13px]">{"check"}</span>
                                                                            {"Selected"}
                                                                        </span>
                                                                    }
                                                                } else {
                                                                    html! {}
                                                                }
                                                            }
                                                        </div>

                                                        {
                                                            if !previews.is_empty() {
                                                                html! {
                                                                    <div class="flex items-center gap-2 flex-wrap text-xs text-on-surface-variant mt-1">
                                                                        {
                                                                            previews.iter().map(|(k, v)| {
                                                                                html! {
                                                                                    <span class="inline-flex items-center gap-1 bg-surface-container-low px-2 py-0.5 rounded text-[11px] border border-outline-variant/30">
                                                                                        <span class="font-medium text-on-surface-variant/80">{format!("{}:", k)}</span>
                                                                                        <span class="text-on-surface truncate max-w-[180px]">{v}</span>
                                                                                    </span>
                                                                                }
                                                                            }).collect::<Html>()
                                                                        }
                                                                    </div>
                                                                }
                                                            } else {
                                                                html! {}
                                                            }
                                                        }
                                                    </div>

                                                    <div class="shrink-0">
                                                        {
                                                            if is_selected {
                                                                html! {
                                                                    <button
                                                                        type="button"
                                                                        class="px-3.5 py-1.5 bg-primary/15 text-primary rounded-lg text-xs font-bold flex items-center gap-1 cursor-pointer"
                                                                    >
                                                                        <span class="material-symbols-outlined text-sm">{"done"}</span>
                                                                        {"Selected"}
                                                                    </button>
                                                                }
                                                            } else {
                                                                html! {
                                                                    <button
                                                                        type="button"
                                                                        onclick={on_select_record}
                                                                        class="px-3.5 py-1.5 bg-primary text-on-primary rounded-lg text-xs font-bold hover:bg-primary-container hover:text-on-primary-container shadow-sm flex items-center gap-1 transition-all active:scale-95"
                                                                    >
                                                                        <span>{"Choose"}</span>
                                                                        <span class="material-symbols-outlined text-sm">{"arrow_forward"}</span>
                                                                    </button>
                                                                }
                                                            }
                                                        }
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Html>()
                                    }
                                </>
                            }
                        }
                    }
                </div>

                // Modal Footer: Pagination & Actions
                <div class="px-6 py-3.5 border-t border-outline-variant bg-surface-container-low flex items-center justify-between shrink-0">
                    <div class="flex items-center gap-2">
                        {
                            if total_pages > 1 {
                                html! {
                                    <div class="flex items-center gap-2">
                                        <button
                                            type="button"
                                            disabled={!can_prev}
                                            onclick={on_prev_page}
                                            class="px-2.5 py-1 bg-surface border border-outline-variant rounded-lg text-xs font-semibold text-on-surface disabled:opacity-40 disabled:cursor-not-allowed hover:bg-surface-container transition-colors flex items-center gap-1"
                                        >
                                            <span class="material-symbols-outlined text-sm">{"chevron_left"}</span>
                                            <span>{"Prev"}</span>
                                        </button>
                                        <span class="text-xs text-on-surface-variant font-medium">
                                            {format!("Page {} of {}", *page, total_pages)}
                                        </span>
                                        <button
                                            type="button"
                                            disabled={!can_next}
                                            onclick={on_next_page}
                                            class="px-2.5 py-1 bg-surface border border-outline-variant rounded-lg text-xs font-semibold text-on-surface disabled:opacity-40 disabled:cursor-not-allowed hover:bg-surface-container transition-colors flex items-center gap-1"
                                        >
                                            <span>{"Next"}</span>
                                            <span class="material-symbols-outlined text-sm">{"chevron_right"}</span>
                                        </button>
                                    </div>
                                }
                            } else {
                                html! {
                                    <span class="text-xs text-on-surface-variant font-medium">
                                        {format!("Total: {} items", *total_records)}
                                    </span>
                                }
                            }
                        }
                    </div>

                    <div class="flex items-center gap-2">
                        <button
                            type="button"
                            onclick={on_close_btn}
                            class="px-5 py-2 border border-outline-variant rounded-lg text-xs font-bold text-on-surface hover:bg-surface-container-high transition-colors"
                        >
                            {"Cancel"}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}
