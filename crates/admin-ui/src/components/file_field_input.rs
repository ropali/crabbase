use std::collections::HashMap;

use crate::api::client::ApiClient;
use crate::api::use_file_token;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct FileFieldInputProps {
    pub label: String,
    pub value: String,
    #[prop_or_default]
    pub collection_name: String,
    #[prop_or_default]
    pub field_name: String,
    #[prop_or_default]
    pub required: bool,
    #[prop_or_default]
    pub multiple: bool,
    #[prop_or_default]
    pub max_files: Option<usize>,
    #[prop_or_default]
    pub accept: String,
    #[prop_or_default]
    pub item_names: HashMap<String, String>,
    pub on_change: Callback<String>,
}

fn parse_ids(value: &str, multiple: bool) -> Vec<String> {
    let trimmed = value.trim();
    if !multiple {
        return if trimmed.is_empty() {
            Vec::new()
        } else {
            vec![trimmed.to_string()]
        };
    }
    if trimmed.is_empty() || trimmed == "[]" {
        return Vec::new();
    }
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(trimmed) {
        return arr
            .iter()
            .filter_map(|item| {
                item.as_str()
                    .map(str::to_string)
                    .or_else(|| item.get("id").and_then(|v| v.as_str()).map(str::to_string))
            })
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    trimmed
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn serialize_ids(ids: &[String], multiple: bool) -> String {
    if multiple {
        serde_json::to_string(ids).unwrap_or_else(|_| "[]".to_string())
    } else {
        ids.first().cloned().unwrap_or_default()
    }
}

fn is_image_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    [".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".avif"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

fn resolve_file_url(collection_name: &str, value: &str, token: Option<&str>) -> String {
    let base_url = ApiClient::get_base_url();
    let base_trimmed = base_url.trim_end_matches('/');
    if value.starts_with("http://") || value.starts_with("https://") {
        return value.to_string();
    }
    let url = if value.starts_with('/') {
        format!("{}{}", base_trimmed, value)
    } else if !collection_name.trim().is_empty() {
        format!(
            "{}/collections/{}/files/{}",
            base_trimmed, collection_name, value
        )
    } else {
        format!("{}/files/{}", base_trimmed, value)
    };
    ApiClient::with_file_token(url, token, false)
}

fn display_name(id: &str) -> String {
    id.rsplit('/').next().unwrap_or(id).to_string()
}

#[function_component(FileFieldInput)]
pub fn file_field_input(props: &FileFieldInputProps) -> Html {
    let file_token = use_file_token();
    let is_uploading = use_state(|| false);
    let upload_error = use_state(|| None::<String>);
    let local_names = use_state(HashMap::<String, String>::new);
    let local_previews = use_state(HashMap::<String, String>::new);
    let file_input_ref = use_node_ref();

    let on_file_change = {
        let is_uploading = is_uploading.clone();
        let upload_error = upload_error.clone();
        let local_names = local_names.clone();
        let local_previews = local_previews.clone();
        let on_change = props.on_change.clone();
        let collection_name = props.collection_name.clone();
        let field_name = if props.field_name.trim().is_empty() {
            "file".to_string()
        } else {
            props.field_name.clone()
        };
        let multiple = props.multiple;
        let max_files = props.max_files;
        let existing_count = parse_ids(&props.value, multiple).len();
        let current_value = props.value.clone();

        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let Some(file_list) = input.files() else {
                return;
            };
            let remaining = max_files
                .map(|m| m.saturating_sub(existing_count))
                .unwrap_or(usize::MAX);
            let mut selected: Vec<web_sys::File> = Vec::new();
            for idx in 0..file_list.length() {
                if selected.len() >= remaining {
                    break;
                }
                if let Some(file) = file_list.get(idx) {
                    selected.push(file);
                    if !multiple {
                        break;
                    }
                }
            }
            input.set_value("");
            if selected.is_empty() {
                return;
            }

            let is_uploading = is_uploading.clone();
            let upload_error = upload_error.clone();
            let local_names = local_names.clone();
            let local_previews = local_previews.clone();
            let on_change = on_change.clone();
            let collection_name = collection_name.clone();
            let field_name = field_name.clone();
            let current_value = current_value.clone();

            is_uploading.set(true);
            upload_error.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                let form_data = match web_sys::FormData::new() {
                    Ok(fd) => fd,
                    Err(e) => {
                        upload_error.set(Some(format!("Failed to create form data: {:?}", e)));
                        is_uploading.set(false);
                        return;
                    }
                };

                let mut pending_names = Vec::with_capacity(selected.len());
                let mut pending_previews = Vec::with_capacity(selected.len());
                for file in &selected {
                    let filename = file.name();
                    if let Err(e) =
                        form_data.append_with_blob_and_filename(&field_name, file, &filename)
                    {
                        upload_error.set(Some(format!("Failed to attach file: {:?}", e)));
                        is_uploading.set(false);
                        return;
                    }
                    pending_names.push(filename.clone());
                    if is_image_name(&filename) {
                        pending_previews.push(web_sys::Url::create_object_url_with_blob(file).ok());
                    } else {
                        pending_previews.push(None);
                    }
                }

                let client = ApiClient::default();
                match client.upload_files(&collection_name, form_data).await {
                    Ok(uploaded_ids) => {
                        let mut existing = parse_ids(&current_value, multiple);
                        if !multiple {
                            existing.clear();
                        }
                        let mut names = if multiple {
                            (*local_names).clone()
                        } else {
                            HashMap::new()
                        };
                        let mut previews = if multiple {
                            (*local_previews).clone()
                        } else {
                            HashMap::new()
                        };
                        for (idx, id) in uploaded_ids.iter().enumerate() {
                            existing.push(id.clone());
                            if let Some(name) = pending_names.get(idx) {
                                names.insert(id.clone(), name.clone());
                            }
                            if let Some(Some(preview)) = pending_previews.get(idx) {
                                previews.insert(id.clone(), preview.clone());
                            }
                        }
                        local_names.set(names);
                        local_previews.set(previews);
                        is_uploading.set(false);
                        on_change.emit(serialize_ids(&existing, multiple));
                    }
                    Err(err) => {
                        is_uploading.set(false);
                        upload_error.set(Some(format!("Upload failed: {err}")));
                    }
                }
            });
        })
    };

    let ids = parse_ids(&props.value, props.multiple);

    let on_remove = {
        let on_change = props.on_change.clone();
        let local_names = local_names.clone();
        let local_previews = local_previews.clone();
        let multiple = props.multiple;
        let ids = ids.clone();
        Callback::from(move |id: String| {
            let remaining: Vec<String> = ids
                .iter()
                .filter(|existing| **existing != id)
                .cloned()
                .collect();
            if multiple {
                let mut names = (*local_names).clone();
                names.remove(&id);
                let mut previews = (*local_previews).clone();
                previews.remove(&id);
                local_names.set(names);
                local_previews.set(previews);
            } else {
                local_names.set(HashMap::new());
                local_previews.set(HashMap::new());
            }
            on_change.emit(serialize_ids(&remaining, multiple));
        })
    };

    let on_clear = {
        let on_change = props.on_change.clone();
        let local_names = local_names.clone();
        let local_previews = local_previews.clone();
        let file_input_ref = file_input_ref.clone();
        let multiple = props.multiple;
        Callback::from(move |_| {
            local_names.set(HashMap::new());
            local_previews.set(HashMap::new());
            if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                input.set_value("");
            }
            on_change.emit(if multiple {
                "[]".to_string()
            } else {
                "".to_string()
            });
        })
    };

    let trigger_file_input = {
        let file_input_ref = file_input_ref.clone();
        Callback::from(move |_| {
            if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                input.set_value("");
                input.click();
            }
        })
    };

    let has_file = !ids.is_empty();
    let max_files = props.max_files;
    let at_cap = max_files.map(|m| ids.len() >= m).unwrap_or(false);

    let render_name = |id: &str| -> String {
        props
            .item_names
            .get(id)
            .or_else(|| local_names.get(id))
            .cloned()
            .unwrap_or_else(|| display_name(id))
    };

    let render_preview = |id: &str| -> Option<String> {
        if let Some(preview) = local_previews.get(id) {
            return Some(preview.clone());
        }
        if is_image_name(&render_name(id)) {
            return Some(resolve_file_url(
                &props.collection_name,
                id,
                file_token.as_deref(),
            ));
        }
        None
    };

    html! {
        <div class="group">
            <div class="flex items-center justify-between mb-1">
                <label class="block font-label-xs text-label-xs text-on-surface-variant flex items-center gap-1">
                    <span class="material-symbols-outlined text-[14px]">{"attach_file"}</span>
                    {&props.label}
                    if props.required {
                        <span class="text-error font-bold">{"*"}</span>
                    }
                </label>
                if *is_uploading {
                    <span class="flex items-center gap-1 text-[11px] text-primary animate-pulse font-medium">
                        <span class="material-symbols-outlined text-[14px] animate-spin">{"progress_activity"}</span>
                        {"Uploading..."}
                    </span>
                } else if has_file {
                    <span class="text-[11px] text-emerald-600 font-medium flex items-center gap-1">
                        <span class="material-symbols-outlined text-[13px]">{"check_circle"}</span>
                        if props.multiple {
                            if let Some(m) = max_files {
                                {format!("{} of {} stored", ids.len(), m)}
                            } else {
                                {format!("{} stored", ids.len())}
                            }
                        } else {
                            {"Stored"}
                        }
                    </span>
                }
            </div>

            <input
                ref={file_input_ref}
                type="file"
                class="hidden"
                accept={props.accept.clone()}
                multiple={props.multiple}
                onchange={on_file_change}
                disabled={*is_uploading}
            />

            if has_file {
                <div class="border border-outline-variant bg-surface-container-low rounded-lg p-3 flex flex-col gap-2">
                    <div class="flex flex-col divide-y divide-outline-variant/60">
                        {for ids.iter().map(|id| {
                            let id = id.clone();
                            let preview = render_preview(&id);
                            let name = render_name(&id);
                            let url = resolve_file_url(
                                &props.collection_name,
                                &id,
                                file_token.as_deref(),
                            );
                            let on_remove = on_remove.clone();
                            let remove_id = id.clone();
                            html! {
                                <div class="flex items-center justify-between gap-2 py-2 first:pt-0 last:pb-0">
                                    <div class="flex items-center gap-2 min-w-0">
                                        if let Some(src) = preview {
                                            <img
                                                src={src}
                                                alt={name.clone()}
                                                class="w-10 h-10 object-cover rounded border border-outline-variant bg-surface shrink-0"
                                            />
                                        } else {
                                            <div class="w-10 h-10 rounded border border-outline-variant bg-surface flex items-center justify-center shrink-0 text-primary">
                                                <span class="material-symbols-outlined text-[20px]">{"description"}</span>
                                            </div>
                                        }
                                        <div class="min-w-0">
                                            <p class="text-xs font-medium text-on-surface truncate" title={id.clone()}>
                                                {name}
                                            </p>
                                            <div class="flex items-center gap-3 mt-0.5">
                                                <a
                                                    href={url}
                                                    target="_blank"
                                                    rel="noopener noreferrer"
                                                    class="text-[11px] text-primary hover:underline flex items-center gap-0.5"
                                                >
                                                    <span class="material-symbols-outlined text-[12px]">{"open_in_new"}</span>
                                                    {"View file"}
                                                </a>
                                                <span class="text-[10px] font-mono text-on-surface-variant/70 truncate max-w-[180px]" title={id.clone()}>
                                                    {id.clone()}
                                                </span>
                                            </div>
                                        </div>
                                    </div>
                                    <button
                                        type="button"
                                        onclick={Callback::from(move |_| on_remove.emit(remove_id.clone()))}
                                        class="p-1.5 hover:bg-error-container/20 rounded text-on-surface-variant hover:text-error transition-colors shrink-0"
                                        title="Remove file"
                                        disabled={*is_uploading}
                                    >
                                        <span class="material-symbols-outlined text-[16px]">{"delete"}</span>
                                    </button>
                                </div>
                            }
                        })}
                    </div>

                    <div class="flex items-center gap-2 pt-1 border-t border-outline-variant/60">
                        <button
                            type="button"
                            onclick={trigger_file_input}
                            class="px-2 py-1 rounded hover:bg-surface-container text-primary text-[11px] font-medium flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
                            disabled={*is_uploading || at_cap}
                            title={if at_cap { format!("Limit of {} file(s) reached", max_files.unwrap_or(0)) } else { String::new() }}
                        >
                            <span class="material-symbols-outlined text-[16px]">{"upload"}</span>
                            if props.multiple { {"Add files"} } else { {"Replace"} }
                        </button>
                        if props.multiple {
                            <button
                                type="button"
                                onclick={on_clear}
                                class="px-2 py-1 rounded hover:bg-error-container/20 text-on-surface-variant hover:text-error text-[11px] font-medium flex items-center gap-1"
                                disabled={*is_uploading}
                            >
                                <span class="material-symbols-outlined text-[16px]">{"delete_sweep"}</span>
                                {"Remove all"}
                            </button>
                        } else {
                            <button
                                type="button"
                                onclick={on_clear}
                                class="px-2 py-1 rounded hover:bg-error-container/20 text-on-surface-variant hover:text-error text-[11px] font-medium flex items-center gap-1"
                                disabled={*is_uploading}
                            >
                                <span class="material-symbols-outlined text-[16px]">{"delete"}</span>
                                {"Remove"}
                            </button>
                        }
                    </div>
                </div>
            } else {
                <div
                    onclick={trigger_file_input}
                    class={format!(
                        "border-2 border-dashed border-outline-variant rounded-lg p-5 flex flex-col items-center justify-center cursor-pointer transition-colors bg-white hover:bg-surface-container-low hover:border-primary group {}",
                        if *is_uploading { "opacity-50 cursor-not-allowed" } else { "" }
                    )}
                >
                    <span class="material-symbols-outlined text-primary text-3xl mb-1 group-hover:scale-110 transition-transform">
                        {"cloud_upload"}
                    </span>
                    <p class="font-body-sm text-body-sm font-semibold text-on-surface">
                        if *is_uploading {
                            {"Uploading file to storage..."}
                        } else if props.multiple {
                            {"Click to browse or drop files"}
                        } else {
                            {"Click to browse or drop file"}
                        }
                    </p>
                    <p class="font-label-xs text-label-xs text-on-surface-variant mt-0.5">
                        if props.multiple {
                            {"Files will be stored and linked to this record"}
                        } else {
                            {"File will be stored and linked to this record"}
                        }
                    </p>
                </div>
            }

            if let Some(ref err) = *upload_error {
                <p class="mt-1 text-xs text-error flex items-center gap-1">
                    <span class="material-symbols-outlined text-[13px]">{"error"}</span>
                    {err}
                </p>
            }
        </div>
    }
}
