use std::collections::HashMap;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::{
    api::client::ApiClient,
    components::data_table::{CellValue, DynamicRow},
    models::{collection::Field, record::UpdateRecordRequest},
};

#[derive(Properties, PartialEq)]
pub struct EditRecordDrawerProps {
    pub collection_name: String,
    pub collection_fields: Vec<Field>,
    pub record: DynamicRow,
    pub on_close: Callback<()>,
    pub on_success: Callback<()>,
}

#[function_component(EditRecordDrawer)]
pub fn edit_record_drawer(props: &EditRecordDrawerProps) -> Html {
    let record_id = props.record.get("id").as_str();
    let created_val = props.record.get("created").as_str();
    let updated_val = props.record.get("updated").as_str();

    let fields = {
        let record = props.record.clone();
        use_state(move || {
            let mut initial_fields = HashMap::<String, String>::new();
            for (k, v) in &record.values {
                let str_val = match v {
                    CellValue::Text(s) => s.clone(),
                    CellValue::Number(n) => n.to_string(),
                    CellValue::Bool(b) => b.to_string(),
                    CellValue::Null => "".to_string(),
                };
                initial_fields.insert(k.clone(), str_val);
            }
            initial_fields
        })
    };

    let email = {
        let record = props.record.clone();
        use_state(move || {
            if let CellValue::Text(s) = record.get("email") {
                s.clone()
            } else {
                "".to_string()
            }
        })
    };
    let password = use_state(|| "".to_string());
    let confirm_password = use_state(|| "".to_string());
    let username = {
        let record = props.record.clone();
        use_state(move || {
            if let CellValue::Text(s) = record.get("username") {
                s.clone()
            } else {
                "".to_string()
            }
        })
    };
    let name = {
        let record = props.record.clone();
        use_state(move || {
            if let CellValue::Text(s) = record.get("name") {
                s.clone()
            } else {
                "".to_string()
            }
        })
    };
    let website = {
        let record = props.record.clone();
        use_state(move || {
            if let CellValue::Text(s) = record.get("website") {
                s.clone()
            } else {
                "".to_string()
            }
        })
    };
    let verified = {
        let record = props.record.clone();
        use_state(move || {
            if let CellValue::Bool(b) = record.get("verified") {
                *b
            } else {
                false
            }
        })
    };

    let error_msg = use_state(|| None::<String>);

    let on_close_click = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| {
            on_close.emit(());
        })
    };

    let on_drawer_click = Callback::from(|e: MouseEvent| {
        e.stop_propagation();
    });

    let on_email_input = {
        let email = email.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            email.set(input.value());
        })
    };

    let on_password_input = {
        let password = password.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            password.set(input.value());
        })
    };

    let on_confirm_input = {
        let confirm_password = confirm_password.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            confirm_password.set(input.value());
        })
    };

    let on_username_input = {
        let username = username.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            username.set(input.value());
        })
    };

    let on_name_input = {
        let name = name.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            name.set(input.value());
        })
    };

    let on_website_input = {
        let website = website.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            website.set(input.value());
        })
    };

    let on_verified_change = {
        let verified = verified.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            verified.set(input.checked());
        })
    };

    let on_dynamic_field_change = {
        let fields = fields.clone();
        Callback::from(move |(key, val): (String, String)| {
            let mut current = (*fields).clone();
            current.insert(key, val);
            fields.set(current);
        })
    };

    let on_submit = {
        let col_name = props.collection_name.clone();
        let collection_fields = props.collection_fields.clone();
        let record_id = record_id.clone();

        let email = email.clone();
        let password = password.clone();
        let confirm_password = confirm_password.clone();
        let username = username.clone();
        let name = name.clone();
        let website = website.clone();
        let verified = verified.clone();
        let is_users = props.collection_name == "users" || props.collection_name == "_superusers";

        let fields = fields.clone();
        let on_success = props.on_success.clone();
        let error_msg = error_msg.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            let col_name = col_name.clone();
            let collection_fields = collection_fields.clone();
            let record_id = record_id.clone();
            let on_success = on_success.clone();
            let error_msg = error_msg.clone();

            if is_users && !password.is_empty() && *password != *confirm_password {
                error_msg.set(Some("Passwords do not match".to_string()));
                return;
            }

            let mut data_map = serde_json::Map::new();

            if is_users {
                data_map.insert(
                    "email".to_string(),
                    serde_json::Value::String((*email).clone()),
                );
                if !password.is_empty() {
                    data_map.insert(
                        "password".to_string(),
                        serde_json::Value::String((*password).clone()),
                    );
                }
                data_map.insert(
                    "username".to_string(),
                    serde_json::Value::String((*username).clone()),
                );
                data_map.insert(
                    "name".to_string(),
                    serde_json::Value::String((*name).clone()),
                );
                data_map.insert(
                    "website".to_string(),
                    serde_json::Value::String((*website).clone()),
                );
                data_map.insert("verified".to_string(), serde_json::Value::Bool(*verified));
            } else {
                for f in &collection_fields {
                    if let Some(val) = fields.get(&f.name) {
                        let json_val = match f.data_type.to_lowercase().as_str() {
                            "bool" => {
                                let b = val == "true";
                                serde_json::Value::Bool(b)
                            }
                            "number" => {
                                if let Ok(n) = val.parse::<f64>() {
                                    if let Some(num) = serde_json::Number::from_f64(n) {
                                        serde_json::Value::Number(num)
                                    } else {
                                        serde_json::Value::Null
                                    }
                                } else {
                                    serde_json::Value::Null
                                }
                            }
                            _ => serde_json::Value::String(val.clone()),
                        };
                        data_map.insert(f.name.clone(), json_val);
                    }
                }
            }

            wasm_bindgen_futures::spawn_local(async move {
                let client = ApiClient::new("/api".to_string(), None);
                let payload = UpdateRecordRequest { data: data_map };

                match client.update_record(&col_name, &record_id, payload).await {
                    Ok(_) => {
                        error_msg.set(None);
                        on_success.emit(())
                    }
                    Err(err) => error_msg.set(Some(format!("API Error: {}", err))),
                }
            });
        })
    };

    let is_users = props.collection_name == "users" || props.collection_name == "_superusers";

    html! {
        <div onclick={on_close_click.clone()} class="absolute inset-0 bg-inverse-surface/10 bg-blur z-40 flex justify-end">
            <div onclick={on_drawer_click} class="w-[480px] h-full bg-surface shadow-2xl z-50 flex flex-col border-l border-outline-variant animate-slide-in-right duration-300 relative">
                <div class="p-6 border-b border-outline-variant flex justify-between items-center">
                    <div>
                        <h2 class="font-headline-md text-headline-md text-on-surface font-bold">{format!("Edit {} record", props.collection_name)}</h2>
                        <p class="font-label-xs text-label-xs text-on-surface-variant">{format!("Update existing entry in the {} collection", props.collection_name)}</p>
                    </div>
                    <button onclick={on_close_click.clone()} class="p-2 hover:bg-surface-container-high rounded-full transition-colors">
                        <span class="material-symbols-outlined">{"close"}</span>
                    </button>
                </div>

                <form onsubmit={on_submit} class="flex-1 flex flex-col min-h-0">
                    <div class="flex-1 overflow-y-auto p-6 space-y-6 custom-scrollbar">
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

                        <div class="space-y-4">
                            <div class="group">
                                <label class="block font-label-xs text-label-xs text-on-surface-variant mb-1 flex items-center gap-1">
                                    <span class="material-symbols-outlined text-[14px]">{"key"}</span> {"id"}
                                </label>
                                <input class="w-full bg-surface-container-lowest border border-outline-variant rounded p-3 font-code-md text-code-md text-on-surface-variant/70 outline-none cursor-not-allowed" readonly=true type="text" value={record_id} />
                            </div>

                            <div class="grid grid-cols-2 gap-4">
                                <div class="group">
                                    <label class="block font-label-xs text-label-xs text-on-surface-variant mb-1 flex items-center gap-1">
                                        <span class="material-symbols-outlined text-[14px]">{"calendar_today"}</span> {"created"}
                                    </label>
                                    <input class="w-full bg-surface-container-lowest border border-outline-variant rounded p-3 font-body-sm text-body-sm text-on-surface-variant/70 outline-none cursor-not-allowed" readonly=true type="text" value={created_val} />
                                </div>
                                <div class="group">
                                    <label class="block font-label-xs text-label-xs text-on-surface-variant mb-1 flex items-center gap-1">
                                        <span class="material-symbols-outlined text-[14px]">{"update"}</span> {"updated"}
                                    </label>
                                    <input class="w-full bg-surface-container-lowest border border-outline-variant rounded p-3 font-body-sm text-body-sm text-on-surface-variant/70 outline-none cursor-not-allowed" readonly=true type="text" value={updated_val} />
                                </div>
                            </div>

                            {
                                if is_users {
                                    html! {
                                        <>
                                            <div class="group">
                                                <label class="block font-label-xs text-label-xs text-on-surface-variant mb-1 flex items-center gap-1">
                                                    <span class="material-symbols-outlined text-[14px]">{"mail"}</span> {"email"} <span class="text-primary">{"*"}</span>
                                                </label>
                                                <div class="relative">
                                                    <input class="w-full bg-white border border-outline-variant rounded p-3 font-body-sm text-body-sm text-on-surface outline-none" placeholder="example@crabbase.io" required=true type="email" value={(*email).clone()} oninput={on_email_input} />
                                                    <div class="absolute right-3 top-3.5 flex items-center gap-1 bg-surface-container-high px-2 py-0.5 rounded border border-outline-variant">
                                                        <span class="font-label-xs text-label-xs">{"Public: Off"}</span>
                                                        <span class="material-symbols-outlined text-[14px] text-primary">{"visibility_off"}</span>
                                                    </div>
                                                </div>
                                            </div>

                                            <div class="grid grid-cols-2 gap-4">
                                                <div class="group">
                                                    <label class="block font-label-xs text-label-xs text-on-surface-variant mb-1 flex items-center gap-1">
                                                        <span class="material-symbols-outlined text-[14px]">{"lock"}</span> {"Password"}
                                                    </label>
                                                    <input class="w-full bg-white border border-outline-variant rounded p-3 font-body-sm text-body-sm text-on-surface outline-none" placeholder="Leave blank to keep same" type="password" value={(*password).clone()} oninput={on_password_input} />
                                                </div>
                                                <div class="group">
                                                    <label class="block font-label-xs text-label-xs text-on-surface-variant mb-1 flex items-center gap-1">
                                                        <span class="material-symbols-outlined text-[14px]">{"lock_reset"}</span> {"Confirm"}
                                                    </label>
                                                    <input class="w-full bg-white border border-outline-variant rounded p-3 font-body-sm text-body-sm text-on-surface outline-none" placeholder="Leave blank to keep same" type="password" value={(*confirm_password).clone()} oninput={on_confirm_input} />
                                                </div>
                                            </div>

                                            <div class="flex items-center justify-between p-3 bg-surface-container-low border border-outline-variant rounded">
                                                <div class="flex items-center gap-2">
                                                    <span class="material-symbols-outlined text-primary">{"verified_user"}</span>
                                                    <span class="font-label-xs text-label-xs">{"Verified status"}</span>
                                                </div>
                                                <label class="relative inline-flex items-center cursor-pointer">
                                                    <input class="sr-only peer" type="checkbox" checked={*verified} onchange={on_verified_change} />
                                                    <div class="w-11 h-6 bg-outline-variant peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary"></div>
                                                </label>
                                            </div>

                                            <div class="group">
                                                <label class="block font-label-xs text-label-xs text-on-surface-variant mb-1 flex items-center gap-1">
                                                    <span class="material-symbols-outlined text-[14px]">{"alternate_email"}</span> {"username"}
                                                </label>
                                                <input class="w-full bg-white border border-outline-variant rounded p-3 font-code-md text-code-md text-on-surface outline-none" placeholder="username" type="text" value={(*username).clone()} oninput={on_username_input} />
                                            </div>

                                            <div class="group">
                                                <label class="block font-label-xs text-label-xs text-on-surface-variant mb-1 flex items-center gap-1">
                                                    <span class="material-symbols-outlined text-[14px]">{"badge"}</span> {"name"}
                                                </label>
                                                <input class="w-full bg-white border border-outline-variant rounded p-3 font-body-sm text-body-sm text-on-surface outline-none" placeholder="Full name" type="text" value={(*name).clone()} oninput={on_name_input} />
                                            </div>

                                            <div class="group">
                                                <label class="block font-label-xs text-label-xs text-on-surface-variant mb-1 flex items-center gap-1">
                                                    <span class="material-symbols-outlined text-[14px]">{"photo_camera"}</span> {"avatar"}
                                                </label>
                                                <div class="border-2 border-dashed border-outline-variant rounded-xl p-8 flex flex-col items-center justify-center bg-surface-container-lowest hover:bg-surface-container transition-colors cursor-pointer group-hover:border-primary">
                                                    <span class="material-symbols-outlined text-primary text-4xl mb-2">{"cloud_upload"}</span>
                                                    <p class="font-body-sm text-body-sm font-bold text-on-surface">{"Upload or drop new file"}</p>
                                                    <p class="font-label-xs text-label-xs text-on-surface-variant mt-1">{"PNG, JPG, SVG up to 5MB"}</p>
                                                </div>
                                            </div>

                                            <div class="group">
                                                <label class="block font-label-xs text-label-xs text-on-surface-variant mb-1 flex items-center gap-1">
                                                    <span class="material-symbols-outlined text-[14px]">{"link"}</span> {"website"}
                                                </label>
                                                <input class="w-full bg-white border border-outline-variant rounded p-3 font-body-sm text-body-sm text-on-surface outline-none" placeholder="https://example.com" type="url" value={(*website).clone()} oninput={on_website_input} />
                                            </div>
                                        </>
                                    }
                                } else {
                                    html! {
                                        <>
                                            {
                                                props.collection_fields.iter().map(|f| {
                                                    let label_text = f.name.clone();
                                                    let key = f.name.clone();

                                                    let icon = match f.data_type.to_lowercase().as_str() {
                                                        "number" => "123",
                                                        "bool" => "check_box",
                                                        "json" => "data_object",
                                                        "relation" => "link",
                                                        "email" => "mail",
                                                        "url" => "link",
                                                        "file" => "photo_camera",
                                                        _ => "text_fields"
                                                    };

                                                    let on_change = {
                                                        let on_dynamic_field_change = on_dynamic_field_change.clone();
                                                        let key = key.clone();
                                                        Callback::from(move |e: Event| {
                                                            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                            let val = if input.checked() { "true".to_string() } else { "false".to_string() };
                                                            on_dynamic_field_change.emit((key.clone(), val));
                                                        })
                                                    };

                                                    let on_input = {
                                                        let on_dynamic_field_change = on_dynamic_field_change.clone();
                                                        let key = key.clone();
                                                        Callback::from(move |e: InputEvent| {
                                                            let input: HtmlInputElement = e.target_unchecked_into();
                                                            on_dynamic_field_change.emit((key.clone(), input.value()));
                                                        })
                                                    };

                                                    let field_val = fields.get(&key).cloned().unwrap_or_default();

                                                    if f.data_type.to_lowercase() == "bool" {
                                                        let is_checked = field_val == "true";
                                                        html! {
                                                            <div class="flex items-center justify-between p-3 bg-surface-container-low border border-outline-variant rounded" key={key.clone()}>
                                                                <div class="flex items-center gap-2">
                                                                    <span class="material-symbols-outlined text-primary">{icon}</span>
                                                                    <span class="font-label-xs text-label-xs">{label_text}</span>
                                                                </div>
                                                                <label class="relative inline-flex items-center cursor-pointer">
                                                                    <input class="sr-only peer" type="checkbox" checked={is_checked} onchange={on_change} />
                                                                    <div class="w-11 h-6 bg-outline-variant peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary"></div>
                                                                </label>
                                                            </div>
                                                        }
                                                    } else {
                                                        html! {
                                                            <div class="group" key={key.clone()}>
                                                                <label class="block font-label-xs text-label-xs text-on-surface-variant mb-1 flex items-center gap-1">
                                                                    <span class="material-symbols-outlined text-[14px]">{icon}</span> {label_text}
                                                                </label>
                                                                <input class="w-full bg-white border border-outline-variant rounded p-3 font-body-sm text-body-sm text-on-surface outline-none focus:border-primary" type="text" value={field_val} oninput={on_input} />
                                                            </div>
                                                        }
                                                    }
                                                }).collect::<Html>()
                                            }
                                        </>
                                    }
                                }
                            }
                        </div>
                    </div>

                    <footer class="p-6 border-t border-outline-variant bg-surface-container-low flex justify-end gap-3">
                        <button type="button" onclick={on_close_click} class="px-6 py-2 border border-outline-variant rounded-lg text-body-sm font-bold text-on-surface hover:bg-surface-container-high transition-colors">
                            {"Cancel"}
                        </button>
                        <button type="submit" class="px-8 py-2 bg-primary hover:bg-primary-container text-on-primary rounded-lg text-body-sm font-bold shadow-sm transition-all active:scale-95">
                            {"Save"}
                        </button>
                    </footer>
                </form>
            </div>
        </div>
    }
}
