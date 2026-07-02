use crate::api::client::ApiClient;
use crate::models::collection::CreateCollectionRequest;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct CustomField {
    pub id: usize,
    pub name: String,
    pub data_type: String,
    pub required: bool,
    pub expanded: bool,
    pub min_len: Option<usize>,
    pub max_len: Option<usize>,
    pub validation_pattern: String,
    pub autogenerate_pattern: String,
    pub min_val: Option<f64>,
    pub max_val: Option<f64>,
    pub help_text: String,
    pub presentable: bool,
    pub hidden: bool,
    pub related_to: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CustomIndex {
    pub id: usize,
    pub fields: String,
    pub unique: bool,
}

#[derive(Properties, PartialEq)]
pub struct CreateCollectionDrawerProps {
    pub on_close: Callback<()>,
    pub on_success: Callback<()>,
}

#[function_component(CreateCollectionDrawer)]
pub fn create_collection_drawer(props: &CreateCollectionDrawerProps) -> Html {
    let name = use_state(|| "".to_string());
    let collection_type = use_state(|| "Base".to_string());
    let show_type_dropdown = use_state(|| false);
    let active_tab = use_state(|| "fields".to_string());
    let error_msg = use_state(|| None::<String>);

    let fields = use_state(|| Vec::<CustomField>::new());
    let next_field_id = use_state(|| 0usize);

    let indexes = use_state(|| Vec::<CustomIndex>::new());
    let next_index_id = use_state(|| 0usize);

    let available_collections = use_state(Vec::<crate::models::collection::Collection>::new);

    // API Rules states
    let list_rule = use_state(|| "public".to_string());
    let list_expr = use_state(|| "".to_string());
    let view_rule = use_state(|| "public".to_string());
    let view_expr = use_state(|| "".to_string());
    let create_rule = use_state(|| "admin".to_string());
    let create_expr = use_state(|| "".to_string());
    let update_rule = use_state(|| "admin".to_string());
    let update_expr = use_state(|| "".to_string());
    let delete_rule = use_state(|| "admin".to_string());
    let delete_expr = use_state(|| "".to_string());

    {
        let available_collections = available_collections.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let client = ApiClient::new("/api".to_string(), None);
                if let Ok(res) = client.get_collections().await {
                    available_collections.set(res.items);
                }
            });
            || ()
        });
    }

    let on_close_click = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| {
            on_close.emit(());
        })
    };

    let on_drawer_click = Callback::from(|e: MouseEvent| {
        e.stop_propagation();
    });

    let on_input_name = {
        let name = name.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            name.set(input.value());
        })
    };

    let toggle_type_dropdown = {
        let show_type_dropdown = show_type_dropdown.clone();
        Callback::from(move |_| {
            show_type_dropdown.set(!*show_type_dropdown);
        })
    };

    let select_type = {
        let collection_type = collection_type.clone();
        let show_type_dropdown = show_type_dropdown.clone();
        Callback::from(move |t: String| {
            collection_type.set(t);
            show_type_dropdown.set(false);
        })
    };

    let select_tab = {
        let active_tab = active_tab.clone();
        Callback::from(move |tab: String| {
            active_tab.set(tab);
        })
    };

    // Fields Actions
    let add_field = {
        let fields = fields.clone();
        let next_field_id = next_field_id.clone();
        Callback::from(move |_| {
            let mut current = (*fields).clone();
            current.push(CustomField {
                id: *next_field_id,
                name: "".to_string(),
                data_type: "Text".to_string(),
                required: false,
                expanded: false,
                min_len: None,
                max_len: None,
                validation_pattern: "".to_string(),
                autogenerate_pattern: "".to_string(),
                min_val: None,
                max_val: None,
                help_text: "".to_string(),
                presentable: true,
                hidden: false,
                related_to: None,
            });
            fields.set(current);
            next_field_id.set(*next_field_id + 1);
        })
    };

    let remove_field = {
        let fields = fields.clone();
        Callback::from(move |id: usize| {
            let current: Vec<CustomField> =
                (*fields).iter().filter(|f| f.id != id).cloned().collect();
            fields.set(current);
        })
    };

    let update_field_name = {
        let fields = fields.clone();
        Callback::from(move |(id, new_name): (usize, String)| {
            let current: Vec<CustomField> = (*fields)
                .iter()
                .map(|f| {
                    if f.id == id {
                        CustomField {
                            name: new_name.clone(),
                            ..f.clone()
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            fields.set(current);
        })
    };

    let update_field_type = {
        let fields = fields.clone();
        Callback::from(move |(id, new_type): (usize, String)| {
            let current: Vec<CustomField> = (*fields)
                .iter()
                .map(|f| {
                    if f.id == id {
                        CustomField {
                            data_type: new_type.clone(),
                            ..f.clone()
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            fields.set(current);
        })
    };

    let update_field_required = {
        let fields = fields.clone();
        Callback::from(move |(id, req): (usize, bool)| {
            let current: Vec<CustomField> = (*fields)
                .iter()
                .map(|f| {
                    if f.id == id {
                        CustomField {
                            required: req,
                            ..f.clone()
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            fields.set(current);
        })
    };

    let toggle_field_expand = {
        let fields = fields.clone();
        Callback::from(move |id: usize| {
            let current: Vec<CustomField> = (*fields)
                .iter()
                .map(|f| {
                    if f.id == id {
                        CustomField {
                            expanded: !f.expanded,
                            ..f.clone()
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            fields.set(current);
        })
    };

    let update_field_min_len = {
        let fields = fields.clone();
        Callback::from(move |(id, val): (usize, Option<usize>)| {
            let current: Vec<CustomField> = (*fields)
                .iter()
                .map(|f| {
                    if f.id == id {
                        CustomField {
                            min_len: val,
                            ..f.clone()
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            fields.set(current);
        })
    };

    let update_field_max_len = {
        let fields = fields.clone();
        Callback::from(move |(id, val): (usize, Option<usize>)| {
            let current: Vec<CustomField> = (*fields)
                .iter()
                .map(|f| {
                    if f.id == id {
                        CustomField {
                            max_len: val,
                            ..f.clone()
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            fields.set(current);
        })
    };

    let update_field_validation = {
        let fields = fields.clone();
        Callback::from(move |(id, val): (usize, String)| {
            let current: Vec<CustomField> = (*fields)
                .iter()
                .map(|f| {
                    if f.id == id {
                        CustomField {
                            validation_pattern: val.clone(),
                            ..f.clone()
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            fields.set(current);
        })
    };

    let update_field_autogenerate = {
        let fields = fields.clone();
        Callback::from(move |(id, val): (usize, String)| {
            let current: Vec<CustomField> = (*fields)
                .iter()
                .map(|f| {
                    if f.id == id {
                        CustomField {
                            autogenerate_pattern: val.clone(),
                            ..f.clone()
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            fields.set(current);
        })
    };

    let update_field_min_val = {
        let fields = fields.clone();
        Callback::from(move |(id, val): (usize, Option<f64>)| {
            let current: Vec<CustomField> = (*fields)
                .iter()
                .map(|f| {
                    if f.id == id {
                        CustomField {
                            min_val: val,
                            ..f.clone()
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            fields.set(current);
        })
    };

    let update_field_max_val = {
        let fields = fields.clone();
        Callback::from(move |(id, val): (usize, Option<f64>)| {
            let current: Vec<CustomField> = (*fields)
                .iter()
                .map(|f| {
                    if f.id == id {
                        CustomField {
                            max_val: val,
                            ..f.clone()
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            fields.set(current);
        })
    };

    let update_field_help_text = {
        let fields = fields.clone();
        Callback::from(move |(id, val): (usize, String)| {
            let current: Vec<CustomField> = (*fields)
                .iter()
                .map(|f| {
                    if f.id == id {
                        CustomField {
                            help_text: val.clone(),
                            ..f.clone()
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            fields.set(current);
        })
    };

    let update_field_presentable = {
        let fields = fields.clone();
        Callback::from(move |(id, val): (usize, bool)| {
            let current: Vec<CustomField> = (*fields)
                .iter()
                .map(|f| {
                    if f.id == id {
                        CustomField {
                            presentable: val,
                            ..f.clone()
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            fields.set(current);
        })
    };

    let update_field_hidden = {
        let fields = fields.clone();
        Callback::from(move |(id, val): (usize, bool)| {
            let current: Vec<CustomField> = (*fields)
                .iter()
                .map(|f| {
                    if f.id == id {
                        CustomField {
                            hidden: val,
                            ..f.clone()
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            fields.set(current);
        })
    };

    let update_field_related_to = {
        let fields = fields.clone();
        Callback::from(move |(id, val): (usize, Option<String>)| {
            let current: Vec<CustomField> = (*fields)
                .iter()
                .map(|f| {
                    if f.id == id {
                        CustomField {
                            related_to: val.clone(),
                            ..f.clone()
                        }
                    } else {
                        f.clone()
                    }
                })
                .collect();
            fields.set(current);
        })
    };

    // Indexes Actions
    let add_index = {
        let indexes = indexes.clone();
        let next_index_id = next_index_id.clone();
        Callback::from(move |_| {
            let mut current = (*indexes).clone();
            current.push(CustomIndex {
                id: *next_index_id,
                fields: "".to_string(),
                unique: false,
            });
            indexes.set(current);
            next_index_id.set(*next_index_id + 1);
        })
    };

    let remove_index = {
        let indexes = indexes.clone();
        Callback::from(move |id: usize| {
            let current: Vec<CustomIndex> = (*indexes)
                .iter()
                .filter(|idx| idx.id != id)
                .cloned()
                .collect();
            indexes.set(current);
        })
    };

    let update_index_fields = {
        let indexes = indexes.clone();
        Callback::from(move |(id, val): (usize, String)| {
            let current: Vec<CustomIndex> = (*indexes)
                .iter()
                .map(|idx| {
                    if idx.id == id {
                        CustomIndex {
                            fields: val.clone(),
                            ..idx.clone()
                        }
                    } else {
                        idx.clone()
                    }
                })
                .collect();
            indexes.set(current);
        })
    };

    let update_index_unique = {
        let indexes = indexes.clone();
        Callback::from(move |(id, val): (usize, bool)| {
            let current: Vec<CustomIndex> = (*indexes)
                .iter()
                .map(|idx| {
                    if idx.id == id {
                        CustomIndex {
                            unique: val,
                            ..idx.clone()
                        }
                    } else {
                        idx.clone()
                    }
                })
                .collect();
            indexes.set(current);
        })
    };

    let on_submit = {
        let name = name.clone();
        let fields = fields.clone();
        let indexes = indexes.clone();
        let on_success = props.on_success.clone();
        let error_msg = error_msg.clone();
        let collection_type = collection_type.clone();

        Callback::from(move |_| {
            let name_val = (*name).clone();
            let fields_val = (*fields).clone();
            let indexes_val = (*indexes).clone();
            let on_success = on_success.clone();
            let error_msg = error_msg.clone();
            let collection_type_val = (*collection_type).clone();

            if name_val.is_empty() {
                error_msg.set(Some("Collection name is required".to_string()));
                return;
            }

            wasm_bindgen_futures::spawn_local(async move {
                let client = ApiClient::new("/api".to_string(), None);

                let index_field_names: std::collections::HashSet<String> = indexes_val
                    .iter()
                    .flat_map(|idx| idx.fields.split(',').map(|s| s.trim().to_string()))
                    .collect();

                let mut columns = fields_val
                    .into_iter()
                    .filter(|f| !f.name.is_empty())
                    .map(|f| {
                        let is_indexed = index_field_names.contains(&f.name);
                        crate::models::collection::Field {
                            name: f.name,
                            data_type: f.data_type,
                            index: is_indexed,
                            related_to: f.related_to,
                        }
                    })
                    .collect::<Vec<_>>();

                if collection_type_val == "Auth" {
                    let auth_fields = vec![
                        ("password", "Text"),
                        ("tokenKey", "Text"),
                        ("email", "Text"),
                        ("emailVisibility", "Bool"),
                        ("verified", "Bool"),
                    ];
                    for (af_name, af_type) in auth_fields {
                        if !columns.iter().any(|c| c.name == af_name) {
                            columns.push(crate::models::collection::Field {
                                name: af_name.to_string(),
                                data_type: af_type.to_string(),
                                index: false,
                                related_to: None,
                            });
                        }
                    }
                }

                let payload = CreateCollectionRequest {
                    name: name_val,
                    columns,
                    collection_type: Some(collection_type_val.to_lowercase()),
                };

                match client.create_collection(payload).await {
                    Ok(_) => {
                        error_msg.set(None);
                        on_success.emit(());
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("API Error: {}", e)));
                    }
                }
            });
        })
    };

    let rules = vec![
        (
            "List / Search Rule",
            list_rule.clone(),
            list_expr.clone(),
            "Anyone can search and list the records of this collection.",
        ),
        (
            "View Rule",
            view_rule.clone(),
            view_expr.clone(),
            "Anyone can view individual record detail.",
        ),
        (
            "Create Rule",
            create_rule.clone(),
            create_expr.clone(),
            "Restricted to administrators by default.",
        ),
        (
            "Update Rule",
            update_rule.clone(),
            update_expr.clone(),
            "Restricted to administrators by default.",
        ),
        (
            "Delete Rule",
            delete_rule.clone(),
            delete_expr.clone(),
            "Restricted to administrators by default.",
        ),
    ];

    html! {
        <div onclick={on_close_click.clone()} class="absolute inset-0 drawer-mask z-40 flex justify-end">
            <section onclick={on_drawer_click} class="w-drawer_width bg-white h-full shadow-2xl flex flex-col animate-slide-in-right duration-300 relative border-l border-outline-variant">
                <header class="px-6 py-4 flex justify-between items-center border-b border-outline-variant">
                    <h2 class="font-headline-md text-headline-md text-on-surface font-bold">{"Create collection"}</h2>
                    <button onclick={on_close_click.clone()} class="p-1 hover:bg-surface-container rounded transition-colors text-on-surface-variant">
                        <span class="material-symbols-outlined">{"close"}</span>
                    </button>
                </header>

                <div class="flex-1 overflow-y-auto px-6 py-6 flex flex-col gap-8 custom-scrollbar">
                    {
                        if let Some(err) = &*error_msg {
                            html! {
                                <div class="bg-error-container/20 border border-error text-error px-4 py-3 rounded text-xs font-semibold">
                                    { err }
                                </div>
                            }
                        } else {
                            html! {}
                        }
                    }
                    <div class="bg-surface-container-low p-4 rounded-lg industrial-border relative">
                        <div class="flex items-center justify-between mb-2">
                            <label class="font-label-xs text-label-xs uppercase tracking-wider text-on-surface-variant flex items-center gap-1">
                                {"Name"} <span class="text-error">{"*"}</span>
                            </label>

                            <div class="relative">
                                <div onclick={toggle_type_dropdown} class="flex items-center gap-2 bg-white industrial-border px-3 py-1 rounded text-xs font-bold text-on-surface cursor-pointer hover:bg-surface-container-lowest">
                                    <span>{format!("Type: {}", *collection_type)}</span>
                                    <span class="material-symbols-outlined text-[14px]">{"expand_more"}</span>
                                </div>
                                {
                                    if *show_type_dropdown {
                                        html! {
                                            <div class="absolute right-0 mt-1 bg-white border border-outline-variant rounded shadow-lg z-50 py-1 w-32">
                                                <div onclick={select_type.reform(|_| "Base".to_string())} class="px-3 py-1.5 hover:bg-surface-container-low text-xs font-medium cursor-pointer text-on-surface">{"Base"}</div>
                                                <div onclick={select_type.reform(|_| "Auth".to_string())} class="px-3 py-1.5 hover:bg-surface-container-low text-xs font-medium cursor-pointer text-on-surface">{"Auth"}</div>
                                            </div>
                                        }
                                    } else {
                                        html! {}
                                    }
                                }
                            </div>
                        </div>
                        <input oninput={on_input_name} value={(*name).clone()} class="w-full bg-white border border-outline-variant rounded p-3 font-code-md text-code-md focus:ring-primary focus:border-primary placeholder:text-on-surface-variant/40 outline-none" placeholder="e.g. posts" type="text"/>
                    </div>

                    <div>
                        <div class="flex gap-1 border-b border-outline-variant">
                            <button onclick={select_tab.reform(|_| "fields".to_string())} class={classes!("px-8", "py-3", "font-label-xs", "text-label-xs", "uppercase", "tracking-widest", if *active_tab == "fields" { "text-primary border-b-2 border-primary bg-surface-container-lowest font-bold" } else { "text-on-surface-variant hover:bg-surface-container-low transition-colors" })}>
                                {"Fields"}
                            </button>
                            <button onclick={select_tab.reform(|_| "rules".to_string())} class={classes!("px-8", "py-3", "font-label-xs", "text-label-xs", "uppercase", "tracking-widest", if *active_tab == "rules" { "text-primary border-b-2 border-primary bg-surface-container-lowest font-bold" } else { "text-on-surface-variant hover:bg-surface-container-low transition-colors" })}>
                                {"API rules"}
                            </button>
                        </div>

                        {
                            if *active_tab == "fields" {
                                html! {
                                    <>
                                        <div class="mt-6 flex flex-col gap-2">
                                            <div class="flex items-center gap-4 bg-surface-container-low p-3 rounded industrial-border opacity-70 group">
                                                <span class="material-symbols-outlined text-on-surface-variant/60">{"text_fields"}</span>
                                                <div class="flex-1">
                                                    <span class="font-code-md text-code-md">{"id"}</span>
                                                    <span class="ml-2 px-1.5 py-0.5 bg-surface-container-high text-on-surface-variant rounded text-[9px] uppercase font-bold">{"Required"}</span>
                                                </div>
                                                <div class="flex items-center gap-2">
                                                    <div class="text-[11px] font-bold text-on-surface-variant bg-white px-2 py-1 border border-outline-variant rounded flex items-center gap-1">
                                                        {"System"}
                                                    </div>
                                                    <span class="material-symbols-outlined text-on-surface-variant/40">{"settings"}</span>
                                                </div>
                                            </div>

                                            <div class="flex items-center gap-4 bg-white p-3 rounded industrial-border opacity-70">
                                                <span class="material-symbols-outlined text-on-surface-variant/60">{"calendar_today"}</span>
                                                <div class="flex-1">
                                                    <span class="font-code-md text-code-md">{"created"}</span>
                                                </div>
                                                <div class="flex items-center gap-2">
                                                    <div class="text-[11px] font-bold text-on-surface-variant bg-white px-2 py-1 border border-outline-variant rounded flex items-center gap-1">
                                                        {"System"}
                                                    </div>
                                                    <span class="material-symbols-outlined text-on-surface-variant/40">{"settings"}</span>
                                                </div>
                                            </div>

                                            <div class="flex items-center gap-4 bg-white p-3 rounded industrial-border opacity-70">
                                                <span class="material-symbols-outlined text-on-surface-variant/60">{"calendar_today"}</span>
                                                <div class="flex-1">
                                                    <span class="font-code-md text-code-md">{"updated"}</span>
                                                </div>
                                                <div class="flex items-center gap-2">
                                                    <div class="text-[11px] font-bold text-on-surface-variant bg-white px-2 py-1 border border-outline-variant rounded flex items-center gap-1">
                                                        {"System"}
                                                    </div>
                                                    <span class="material-symbols-outlined text-on-surface-variant/40">{"settings"}</span>
                                                </div>
                                            </div>

                                            {
                                                if *collection_type == "Auth" {
                                                    html! {
                                                        <>
                                                            <div class="flex items-center gap-3 bg-white p-3 rounded industrial-border opacity-70">
                                                                <span class="material-symbols-outlined text-on-surface-variant/60">{"text_fields"}</span>
                                                                <input type="text" value="password" disabled=true class="flex-1 min-w-0 bg-transparent border-b border-dashed border-outline-variant py-0.5 font-code-md text-code-md text-on-surface-variant/60" />
                                                                <div class="flex items-center gap-3">
                                                                    <select disabled=true class="bg-surface-container-low border border-outline-variant px-2 py-1 rounded text-[11px] font-bold text-on-surface-variant/60 cursor-not-allowed focus:outline-none">
                                                                        <option selected=true>{"Text"}</option>
                                                                    </select>
                                                                    <button disabled=true class="px-2 py-1 border rounded text-[10px] font-bold bg-primary-container/20 border-primary text-primary cursor-not-allowed">
                                                                        {"REQ"}
                                                                    </button>
                                                                    <div class="text-[11px] font-bold text-on-surface-variant bg-white px-2 py-1 border border-outline-variant rounded">
                                                                        {"Auth"}
                                                                    </div>
                                                                </div>
                                                            </div>
                                                            <div class="flex items-center gap-3 bg-white p-3 rounded industrial-border opacity-70">
                                                                <span class="material-symbols-outlined text-on-surface-variant/60">{"text_fields"}</span>
                                                                <input type="text" value="tokenKey" disabled=true class="flex-1 min-w-0 bg-transparent border-b border-dashed border-outline-variant py-0.5 font-code-md text-code-md text-on-surface-variant/60" />
                                                                <div class="flex items-center gap-3">
                                                                    <select disabled=true class="bg-surface-container-low border border-outline-variant px-2 py-1 rounded text-[11px] font-bold text-on-surface-variant/60 cursor-not-allowed focus:outline-none">
                                                                        <option selected=true>{"Text"}</option>
                                                                    </select>
                                                                    <button disabled=true class="px-2 py-1 border rounded text-[10px] font-bold bg-primary-container/20 border-primary text-primary cursor-not-allowed">
                                                                        {"REQ"}
                                                                    </button>
                                                                    <div class="text-[11px] font-bold text-on-surface-variant bg-white px-2 py-1 border border-outline-variant rounded">
                                                                        {"Auth"}
                                                                    </div>
                                                                </div>
                                                            </div>
                                                            <div class="flex items-center gap-3 bg-white p-3 rounded industrial-border opacity-70">
                                                                <span class="material-symbols-outlined text-on-surface-variant/60">{"text_fields"}</span>
                                                                <input type="text" value="email" disabled=true class="flex-1 min-w-0 bg-transparent border-b border-dashed border-outline-variant py-0.5 font-code-md text-code-md text-on-surface-variant/60" />
                                                                <div class="flex items-center gap-3">
                                                                    <select disabled=true class="bg-surface-container-low border border-outline-variant px-2 py-1 rounded text-[11px] font-bold text-on-surface-variant/60 cursor-not-allowed focus:outline-none">
                                                                        <option selected=true>{"Text"}</option>
                                                                    </select>
                                                                    <button disabled=true class="px-2 py-1 border rounded text-[10px] font-bold bg-primary-container/20 border-primary text-primary cursor-not-allowed">
                                                                        {"REQ"}
                                                                    </button>
                                                                    <div class="text-[11px] font-bold text-on-surface-variant bg-white px-2 py-1 border border-outline-variant rounded">
                                                                        {"Auth"}
                                                                    </div>
                                                                </div>
                                                            </div>
                                                            <div class="flex items-center gap-3 bg-white p-3 rounded industrial-border opacity-70">
                                                                <span class="material-symbols-outlined text-on-surface-variant/60">{"check_box"}</span>
                                                                <input type="text" value="emailVisibility" disabled=true class="flex-1 min-w-0 bg-transparent border-b border-dashed border-outline-variant py-0.5 font-code-md text-code-md text-on-surface-variant/60" />
                                                                <div class="flex items-center gap-3">
                                                                    <select disabled=true class="bg-surface-container-low border border-outline-variant px-2 py-1 rounded text-[11px] font-bold text-on-surface-variant/60 cursor-not-allowed focus:outline-none">
                                                                        <option selected=true>{"Bool"}</option>
                                                                    </select>
                                                                    <button disabled=true class="px-2 py-1 border rounded text-[10px] font-bold bg-transparent border-outline-variant text-on-surface-variant/60 cursor-not-allowed">
                                                                        {"REQ"}
                                                                    </button>
                                                                    <div class="text-[11px] font-bold text-on-surface-variant bg-white px-2 py-1 border border-outline-variant rounded">
                                                                        {"Auth"}
                                                                    </div>
                                                                </div>
                                                            </div>
                                                            <div class="flex items-center gap-3 bg-white p-3 rounded industrial-border opacity-70">
                                                                <span class="material-symbols-outlined text-on-surface-variant/60">{"check_box"}</span>
                                                                <input type="text" value="verified" disabled=true class="flex-1 min-w-0 bg-transparent border-b border-dashed border-outline-variant py-0.5 font-code-md text-code-md text-on-surface-variant/60" />
                                                                <div class="flex items-center gap-3">
                                                                    <select disabled=true class="bg-surface-container-low border border-outline-variant px-2 py-1 rounded text-[11px] font-bold text-on-surface-variant/60 cursor-not-allowed focus:outline-none">
                                                                        <option selected=true>{"Bool"}</option>
                                                                    </select>
                                                                    <button disabled=true class="px-2 py-1 border rounded text-[10px] font-bold bg-transparent border-outline-variant text-on-surface-variant/60 cursor-not-allowed">
                                                                        {"REQ"}
                                                                    </button>
                                                                    <div class="text-[11px] font-bold text-on-surface-variant bg-white px-2 py-1 border border-outline-variant rounded">
                                                                        {"Auth"}
                                                                    </div>
                                                                </div>
                                                            </div>
                                                        </>
                                                    }
                                                } else {
                                                    html! {}
                                                }
                                            }

                                            {
                                                fields.iter().map(|f| {
                                                    let f_id = f.id;
                                                    let f_name = f.name.clone();
                                                    let f_type = f.data_type.clone();
                                                    let f_req = f.required;

                                                    let on_name_change = {
                                                        let update_field_name = update_field_name.clone();
                                                        Callback::from(move |ev: InputEvent| {
                                                            let input: HtmlInputElement = ev.target_unchecked_into();
                                                            update_field_name.emit((f_id, input.value()));
                                                        })
                                                    };

                                                    let on_type_change = {
                                                        let update_field_type = update_field_type.clone();
                                                        Callback::from(move |ev: Event| {
                                                            let select: web_sys::HtmlSelectElement = ev.target_unchecked_into();
                                                            update_field_type.emit((f_id, select.value()));
                                                        })
                                                    };

                                                    let on_req_toggle = {
                                                        let update_field_required = update_field_required.clone();
                                                        let f_req = f_req;
                                                        Callback::from(move |_: MouseEvent| {
                                                            update_field_required.emit((f_id, !f_req));
                                                        })
                                                    };

                                                    let on_req_change = {
                                                        let update_field_required = update_field_required.clone();
                                                        let f_req = f_req;
                                                        Callback::from(move |_: Event| {
                                                            update_field_required.emit((f_id, !f_req));
                                                        })
                                                    };

                                                    let on_remove = {
                                                        let remove_field = remove_field.clone();
                                                        Callback::from(move |_| {
                                                            remove_field.emit(f_id);
                                                        })
                                                    };

                                                    let on_toggle_expand = {
                                                        let toggle_field_expand = toggle_field_expand.clone();
                                                        Callback::from(move |_| {
                                                            toggle_field_expand.emit(f_id);
                                                        })
                                                    };

                                                    let on_min_len_change = {
                                                        let update_field_min_len = update_field_min_len.clone();
                                                        Callback::from(move |ev: InputEvent| {
                                                            let input: HtmlInputElement = ev.target_unchecked_into();
                                                            let val = input.value().parse::<usize>().ok();
                                                            update_field_min_len.emit((f_id, val));
                                                        })
                                                    };

                                                    let on_max_len_change = {
                                                        let update_field_max_len = update_field_max_len.clone();
                                                        Callback::from(move |ev: InputEvent| {
                                                            let input: HtmlInputElement = ev.target_unchecked_into();
                                                            let val = input.value().parse::<usize>().ok();
                                                            update_field_max_len.emit((f_id, val));
                                                        })
                                                    };

                                                    let on_validation_change = {
                                                        let update_field_validation = update_field_validation.clone();
                                                        Callback::from(move |ev: InputEvent| {
                                                            let input: HtmlInputElement = ev.target_unchecked_into();
                                                            update_field_validation.emit((f_id, input.value()));
                                                        })
                                                    };

                                                    let on_autogenerate_change = {
                                                        let update_field_autogenerate = update_field_autogenerate.clone();
                                                        Callback::from(move |ev: InputEvent| {
                                                            let input: HtmlInputElement = ev.target_unchecked_into();
                                                            update_field_autogenerate.emit((f_id, input.value()));
                                                        })
                                                    };

                                                    let on_min_val_change = {
                                                        let update_field_min_val = update_field_min_val.clone();
                                                        Callback::from(move |ev: InputEvent| {
                                                            let input: HtmlInputElement = ev.target_unchecked_into();
                                                            let val = input.value().parse::<f64>().ok();
                                                            update_field_min_val.emit((f_id, val));
                                                        })
                                                    };

                                                    let on_max_val_change = {
                                                        let update_field_max_val = update_field_max_val.clone();
                                                        Callback::from(move |ev: InputEvent| {
                                                            let input: HtmlInputElement = ev.target_unchecked_into();
                                                            let val = input.value().parse::<f64>().ok();
                                                            update_field_max_val.emit((f_id, val));
                                                        })
                                                    };

                                                    let on_help_text_change = {
                                                        let update_field_help_text = update_field_help_text.clone();
                                                        Callback::from(move |ev: InputEvent| {
                                                            let input: HtmlInputElement = ev.target_unchecked_into();
                                                            update_field_help_text.emit((f_id, input.value()));
                                                        })
                                                    };

                                                    let on_presentable_change = {
                                                        let update_field_presentable = update_field_presentable.clone();
                                                        let f_presentable = f.presentable;
                                                        Callback::from(move |_| {
                                                            update_field_presentable.emit((f_id, !f_presentable));
                                                        })
                                                    };

                                                    let on_hidden_change = {
                                                        let update_field_hidden = update_field_hidden.clone();
                                                        let f_hidden = f.hidden;
                                                        Callback::from(move |_| {
                                                            update_field_hidden.emit((f_id, !f_hidden));
                                                        })
                                                    };

                                                    let on_related_to_change = {
                                                        let update_field_related_to = update_field_related_to.clone();
                                                        Callback::from(move |ev: Event| {
                                                            let select: web_sys::HtmlSelectElement = ev.target_unchecked_into();
                                                            let val = select.value();
                                                            let val_opt = if val.is_empty() { None } else { Some(val) };
                                                            update_field_related_to.emit((f_id, val_opt));
                                                        })
                                                    };

                                                    let icon = match f_type.to_lowercase().as_str() {
                                                        "number" => "123",
                                                        "bool" => "check_box",
                                                        "json" => "data_object",
                                                        "relation" => "link",
                                                        _ => "text_fields"
                                                    };

                                                    let config_grid = match f_type.as_str() {
                                                        "Text" => html! {
                                                            <div class="grid grid-cols-2 gap-4">
                                                                <div class="bg-surface-container-low p-3 rounded-lg industrial-border">
                                                                    <div class="flex items-center gap-1 mb-1">
                                                                        <span class="font-label-xs text-label-xs text-on-surface-variant uppercase">{"Min length"}</span>
                                                                        <span class="material-symbols-outlined text-[14px] text-on-surface-variant/60">{"info"}</span>
                                                                    </div>
                                                                    <input type="number" min="0" value={f.min_len.map(|v| v.to_string()).unwrap_or_default()} oninput={on_min_len_change} placeholder="No min limit" class="w-full bg-transparent border-none p-0 focus:ring-0 text-body-sm text-on-surface outline-none" />
                                                                </div>
                                                                <div class="bg-surface-container-low p-3 rounded-lg industrial-border">
                                                                    <div class="flex items-center gap-1 mb-1">
                                                                        <span class="font-label-xs text-label-xs text-on-surface-variant uppercase">{"Max length"}</span>
                                                                        <span class="material-symbols-outlined text-[14px] text-on-surface-variant/60">{"info"}</span>
                                                                    </div>
                                                                    <input type="number" min="0" value={f.max_len.map(|v| v.to_string()).unwrap_or_default()} oninput={on_max_len_change} placeholder="Default to max 5000 characters" class="w-full bg-transparent border-none p-0 focus:ring-0 text-body-sm text-on-surface outline-none" />
                                                                </div>
                                                                <div class="bg-surface-container-low p-3 rounded-lg industrial-border">
                                                                    <div class="flex items-center gap-1 mb-1">
                                                                        <span class="font-label-xs text-label-xs text-on-surface-variant uppercase">{"Validation pattern"}</span>
                                                                    </div>
                                                                    <input type="text" value={f.validation_pattern.clone()} oninput={on_validation_change} placeholder="Ex. ^[a-z0-9]+$" class="w-full bg-transparent border-none p-0 focus:ring-0 text-body-sm text-on-surface outline-none font-code-md text-code-md" />
                                                                </div>
                                                                <div class="bg-surface-container-low p-3 rounded-lg industrial-border">
                                                                    <div class="flex items-center gap-1 mb-1">
                                                                        <span class="font-label-xs text-label-xs text-on-surface-variant uppercase">{"Autogenerate pattern"}</span>
                                                                        <span class="material-symbols-outlined text-[14px] text-on-surface-variant/60">{"info"}</span>
                                                                    </div>
                                                                    <input type="text" value={f.autogenerate_pattern.clone()} oninput={on_autogenerate_change} placeholder="Ex. [a-z0-9]{30}" class="w-full bg-transparent border-none p-0 focus:ring-0 text-body-sm text-on-surface outline-none font-code-md text-code-md" />
                                                                </div>
                                                            </div>
                                                        },
                                                        "Number" => html! {
                                                            <div class="grid grid-cols-2 gap-4">
                                                                <div class="bg-surface-container-low p-3 rounded-lg industrial-border">
                                                                    <div class="flex items-center gap-1 mb-1">
                                                                        <span class="font-label-xs text-label-xs text-on-surface-variant uppercase">{"Min value"}</span>
                                                                        <span class="material-symbols-outlined text-[14px] text-on-surface-variant/60">{"info"}</span>
                                                                    </div>
                                                                    <input type="number" step="any" value={f.min_val.map(|v| v.to_string()).unwrap_or_default()} oninput={on_min_val_change} placeholder="No min limit" class="w-full bg-transparent border-none p-0 focus:ring-0 text-body-sm text-on-surface outline-none" />
                                                                </div>
                                                                <div class="bg-surface-container-low p-3 rounded-lg industrial-border">
                                                                    <div class="flex items-center gap-1 mb-1">
                                                                        <span class="font-label-xs text-label-xs text-on-surface-variant uppercase">{"Max value"}</span>
                                                                        <span class="material-symbols-outlined text-[14px] text-on-surface-variant/60">{"info"}</span>
                                                                    </div>
                                                                    <input type="number" step="any" value={f.max_val.map(|v| v.to_string()).unwrap_or_default()} oninput={on_max_val_change} placeholder="No max limit" class="w-full bg-transparent border-none p-0 focus:ring-0 text-body-sm text-on-surface outline-none" />
                                                                </div>
                                                            </div>
                                                        },
                                                        "Relation" => {
                                                            let current_rel = f.related_to.clone().unwrap_or_default();
                                                            html! {
                                                                <div class="grid grid-cols-2 gap-4">
                                                                    <div class="bg-surface-container-low p-3 rounded-lg industrial-border col-span-2">
                                                                        <div class="flex items-center gap-1 mb-1">
                                                                            <span class="font-label-xs text-label-xs text-on-surface-variant uppercase">{"Related collection"}</span>
                                                                            <span class="material-symbols-outlined text-[14px] text-on-surface-variant/60">{"link"}</span>
                                                                        </div>
                                                                        <select value={current_rel} onchange={on_related_to_change} class="w-full bg-transparent border-none p-0 focus:ring-0 text-body-sm text-on-surface cursor-pointer focus:outline-none">
                                                                            <option value="">{"Select collection..."}</option>
                                                                            {
                                                                                available_collections.iter().map(|col| {
                                                                                    html! {
                                                                                        <option value={col.name.clone()}>{col.name.clone()}</option>
                                                                                    }
                                                                                }).collect::<Html>()
                                                                            }
                                                                        </select>
                                                                    </div>
                                                                </div>
                                                            }
                                                        },
                                                        _ => html! {}
                                                    };

                                                    let f_help_text = f.help_text.clone();
                                                    let f_presentable = f.presentable;
                                                    let f_hidden = f.hidden;

                                                    let config_panel = if f.expanded {
                                                        html! {
                                                            <div class="p-4 flex flex-col gap-4 bg-white border-t border-outline-variant">
                                                                {config_grid}

                                                                <div class="bg-surface-container-low p-3 rounded-lg industrial-border">
                                                                    <div class="flex items-center gap-1 mb-1">
                                                                        <span class="font-label-xs text-label-xs text-on-surface-variant uppercase">{"Help text"}</span>
                                                                    </div>
                                                                    <input type="text" value={f_help_text} oninput={on_help_text_change} placeholder="Help text" class="w-full bg-transparent border-none p-0 focus:ring-0 text-body-sm text-on-surface outline-none" />
                                                                </div>

                                                                <div class="flex items-center justify-between">
                                                                    <div class="flex items-center gap-6">
                                                                        <label class="flex items-center gap-2 cursor-pointer">
                                                                            <input type="checkbox" checked={f_req} onchange={on_req_change} class="rounded-sm border-outline-variant text-primary focus:ring-primary" />
                                                                            <span class="text-body-sm text-on-surface">{"Required (!='')"}</span>
                                                                            <span class="material-symbols-outlined text-[14px] text-on-surface-variant/60">{"info"}</span>
                                                                        </label>
                                                                        <label class="flex items-center gap-2 cursor-pointer relative">
                                                                            <input type="checkbox" checked={f_presentable} onchange={on_presentable_change} class="rounded-sm border-outline-variant text-primary focus:ring-primary" />
                                                                            <span class="text-body-sm text-on-surface">{"Presentable"}</span>
                                                                            <span class="material-symbols-outlined text-[14px] text-on-surface-variant/60">{"info"}</span>
                                                                        </label>
                                                                        <label class="flex items-center gap-2 cursor-pointer">
                                                                            <input type="checkbox" checked={f_hidden} onchange={on_hidden_change} class="rounded-sm border-outline-variant text-primary focus:ring-primary" />
                                                                            <span class="text-body-sm text-on-surface">{"Hidden"}</span>
                                                                            <span class="material-symbols-outlined text-[14px] text-on-surface-variant/60">{"info"}</span>
                                                                        </label>
                                                                    </div>
                                                                </div>
                                                            </div>
                                                        }
                                                    } else {
                                                        html! {}
                                                    };

                                                    if f.expanded {
                                                        html! {
                                                            <div class="bg-surface-container-low rounded industrial-border overflow-hidden" key={f_id}>
                                                                <div class="flex items-center gap-4 p-3 border-b border-outline-variant bg-surface-container-highest/30">
                                                                    <span class="material-symbols-outlined text-on-surface-variant">{icon}</span>
                                                                    <input type="text" value={f_name} oninput={on_name_change} placeholder="field_name" class="flex-1 min-w-0 bg-transparent border-b border-dashed border-outline-variant hover:border-outline focus:border-primary focus:outline-none py-0.5 font-code-md text-code-md text-on-surface" />

                                                                    <div class="flex items-center gap-3">
                                                                        <select value={f_type.clone()} onchange={on_type_change} class="bg-surface-container-low border border-outline-variant px-2 py-1 rounded text-[11px] font-bold text-on-surface cursor-pointer focus:outline-none">
                                                                            <option value="Text" selected={f_type == "Text"}>{"Text"}</option>
                                                                            <option value="Number" selected={f_type == "Number"}>{"Number"}</option>
                                                                            <option value="Bool" selected={f_type == "Bool"}>{"Bool"}</option>
                                                                            <option value="Json" selected={f_type == "Json"}>{"JSON"}</option>
                                                                            <option value="Relation" selected={f_type == "Relation"}>{"Relation"}</option>
                                                                        </select>

                                                                        <button onclick={on_req_toggle} class={classes!("px-2", "py-1", "border", "rounded", "text-[10px]", "font-bold", "transition-colors", if f_req { "bg-primary-container/20 border-primary text-primary" } else { "bg-transparent border-outline-variant text-on-surface-variant hover:border-outline" })}>
                                                                            {"REQ"}
                                                                        </button>

                                                                        <button onclick={on_toggle_expand} class="text-primary transition-colors p-1">
                                                                            <span class="material-symbols-outlined text-[18px]">{"settings"}</span>
                                                                        </button>

                                                                        <button onclick={on_remove} class="text-on-surface-variant hover:text-error transition-colors p-1">
                                                                            <span class="material-symbols-outlined text-[18px]">{"delete"}</span>
                                                                        </button>
                                                                    </div>
                                                                </div>
                                                                {config_panel}
                                                            </div>
                                                        }
                                                    } else {
                                                        html! {
                                                            <div class="flex items-center gap-3 bg-white p-3 rounded industrial-border hover:border-outline transition-colors" key={f_id}>
                                                                <span class="material-symbols-outlined text-on-surface-variant">{icon}</span>
                                                                <input type="text" value={f_name} oninput={on_name_change} placeholder="field_name" class="flex-1 min-w-0 bg-transparent border-b border-dashed border-outline-variant hover:border-outline focus:border-primary focus:outline-none py-0.5 font-code-md text-code-md text-on-surface" />

                                                                <div class="flex items-center gap-3">
                                                                    <select value={f_type.clone()} onchange={on_type_change} class="bg-surface-container-low border border-outline-variant px-2 py-1 rounded text-[11px] font-bold text-on-surface cursor-pointer focus:outline-none">
                                                                        <option value="Text" selected={f_type == "Text"}>{"Text"}</option>
                                                                        <option value="Number" selected={f_type == "Number"}>{"Number"}</option>
                                                                        <option value="Bool" selected={f_type == "Bool"}>{"Bool"}</option>
                                                                        <option value="Json" selected={f_type == "Json"}>{"JSON"}</option>
                                                                        <option value="Relation" selected={f_type == "Relation"}>{"Relation"}</option>
                                                                    </select>

                                                                    <button onclick={on_req_toggle} class={classes!("px-2", "py-1", "border", "rounded", "text-[10px]", "font-bold", "transition-colors", if f_req { "bg-primary-container/20 border-primary text-primary" } else { "bg-transparent border-outline-variant text-on-surface-variant hover:border-outline" })}>
                                                                        {"REQ"}
                                                                    </button>

                                                                    <button onclick={on_toggle_expand} class="text-on-surface-variant hover:text-primary transition-colors p-1">
                                                                        <span class="material-symbols-outlined text-[18px]">{"settings"}</span>
                                                                    </button>

                                                                    <button onclick={on_remove} class="text-on-surface-variant hover:text-error transition-colors p-1">
                                                                        <span class="material-symbols-outlined text-[18px]">{"delete"}</span>
                                                                    </button>
                                                                </div>
                                                            </div>
                                                        }
                                                    }
                                                }).collect::<Html>()
                                            }

                                            <button onclick={add_field} class="mt-4 w-full py-3 border-2 border-dashed border-outline-variant rounded flex items-center justify-center gap-2 text-on-surface-variant hover:border-primary hover:text-primary transition-all group font-bold">
                                                <span class="material-symbols-outlined group-hover:scale-110 transition-transform">{"add"}</span>
                                                <span class="font-label-xs text-label-xs">{"New field"}</span>
                                            </button>
                                        </div>

                                        <div class="mt-8">
                                            <div class="flex items-center justify-between mb-4">
                                                <h3 class="font-label-xs text-label-xs uppercase tracking-wider text-on-surface-variant font-bold">{format!("Unique constraints and indexes ({})", indexes.len())}</h3>
                                            </div>

                                            <div class="flex flex-col gap-2 mb-4">
                                                {
                                                    indexes.iter().map(|idx| {
                                                        let idx_id = idx.id;
                                                        let idx_fields = idx.fields.clone();
                                                        let idx_unique = idx.unique;

                                                        let on_fields_change = {
                                                            let update_index_fields = update_index_fields.clone();
                                                            Callback::from(move |ev: InputEvent| {
                                                                let input: HtmlInputElement = ev.target_unchecked_into();
                                                                update_index_fields.emit((idx_id, input.value()));
                                                            })
                                                        };

                                                        let on_unique_toggle = {
                                                            let update_index_unique = update_index_unique.clone();
                                                            let idx_unique = idx_unique;
                                                            Callback::from(move |_| {
                                                                update_index_unique.emit((idx_id, !idx_unique));
                                                            })
                                                        };

                                                        let on_remove = {
                                                            let remove_index = remove_index.clone();
                                                            Callback::from(move |_| {
                                                                remove_index.emit(idx_id);
                                                            })
                                                        };

                                                        html! {
                                                            <div class="flex items-center gap-3 bg-white p-3 rounded industrial-border" key={idx_id}>
                                                                <span class="material-symbols-outlined text-on-surface-variant">{"tag"}</span>
                                                                <input type="text" value={idx_fields} oninput={on_fields_change} placeholder="e.g. field_a, field_b" class="flex-1 min-w-0 bg-transparent border-b border-dashed border-outline-variant hover:border-outline focus:border-primary focus:outline-none py-0.5 font-code-md text-code-md text-on-surface" />

                                                                <div class="flex items-center gap-2">
                                                                    <button onclick={on_unique_toggle} class={classes!("px-2", "py-1", "border", "rounded", "text-[10px]", "font-bold", "transition-colors", if idx_unique { "bg-primary-container/20 border-primary text-primary" } else { "bg-transparent border-outline-variant text-on-surface-variant hover:border-outline" })}>
                                                                        {"UNIQUE"}
                                                                    </button>
                                                                    <button onclick={on_remove} class="text-on-surface-variant hover:text-error transition-colors p-1">
                                                                        <span class="material-symbols-outlined text-[18px]">{"delete"}</span>
                                                                    </button>
                                                                </div>
                                                            </div>
                                                        }
                                                    }).collect::<Html>()
                                                }
                                            </div>

                                            <button onclick={add_index} class="flex items-center gap-2 px-4 py-2 bg-surface-container-highest industrial-border rounded text-xs font-bold text-on-surface hover:bg-surface-container-high transition-colors">
                                                <span class="material-symbols-outlined text-[16px]">{"add"}</span>
                                                {"New index"}
                                            </button>
                                        </div>
                                    </>
                                }
                            } else {
                                html! {
                                    <div class="mt-6 flex flex-col gap-6">
                                        {
                                            rules.into_iter().map(|(title, rule_type, rule_expr, desc)| {
                                                let r_type = rule_type.clone();
                                                let r_expr = rule_expr.clone();

                                                let select_rule_type = {
                                                    let rule_type = rule_type.clone();
                                                    Callback::from(move |ev: Event| {
                                                        let select: web_sys::HtmlSelectElement = ev.target_unchecked_into();
                                                        rule_type.set(select.value());
                                                    })
                                                };

                                                let update_expr_val = {
                                                    let rule_expr = rule_expr.clone();
                                                    Callback::from(move |ev: InputEvent| {
                                                        let input: HtmlInputElement = ev.target_unchecked_into();
                                                        rule_expr.set(input.value());
                                                    })
                                                };

                                                html! {
                                                    <div class="bg-white p-4 rounded industrial-border flex flex-col gap-3" key={title}>
                                                        <div class="flex justify-between items-center">
                                                            <div>
                                                                <h4 class="font-bold text-xs text-on-surface uppercase tracking-wider">{title}</h4>
                                                                <p class="text-[11px] text-on-surface-variant/75">{desc}</p>
                                                            </div>
                                                            <select value={(*r_type).clone()} onchange={select_rule_type} class="bg-surface-container-low border border-outline-variant px-3 py-1 rounded text-xs font-bold text-on-surface cursor-pointer focus:outline-none">
                                                                <option value="public">{"Everyone (public)"}</option>
                                                                <option value="admin">{"Admin only"}</option>
                                                                <option value="custom">{"Custom rule"}</option>
                                                            </select>
                                                        </div>

                                                        {
                                                            if *r_type == "custom" {
                                                                html! {
                                                                    <div class="flex flex-col gap-1">
                                                                        <input type="text" value={(*r_expr).clone()} oninput={update_expr_val} placeholder="@request.auth.id != ''" class="w-full bg-surface-container-low border border-outline-variant rounded px-3 py-2 font-code-md text-code-md text-on-surface focus:ring-primary focus:border-primary outline-none" />
                                                                        <span class="text-[10px] text-on-surface-variant/60">{"Use standard filters. E.g. @request.auth.id = id or @request.auth.role = 'editor'"}</span>
                                                                    </div>
                                                                }
                                                            } else {
                                                                html! {}
                                                            }
                                                        }
                                                    </div>
                                                }
                                            }).collect::<Html>()
                                        }
                                    </div>
                                }
                            }
                        }
                    </div>
                </div>

                <footer class="p-6 border-t border-outline-variant bg-surface-container-low flex justify-between items-center">
                    <button onclick={on_close_click.clone()} class="px-6 py-2 text-on-surface-variant hover:text-on-surface font-bold transition-colors font-label-xs text-label-xs uppercase tracking-widest">
                        {"Close"}
                    </button>
                    <div class="flex">
                        <button onclick={on_submit} class="px-10 py-3 bg-primary text-on-primary rounded-l font-bold hover:bg-secondary transition-all active:scale-95">
                            {"Create"}
                        </button>
                        <button class="px-2 bg-primary/90 border-l border-on-primary/20 text-on-primary rounded-r hover:bg-secondary transition-all">
                            <span class="material-symbols-outlined text-[18px]">{"expand_more"}</span>
                        </button>
                    </div>
                </footer>
            </section>
        </div>
    }
}
