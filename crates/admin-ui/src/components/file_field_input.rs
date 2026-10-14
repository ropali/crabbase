use crate::api::client::ApiClient;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct FileFieldInputProps {
    pub label: String,
    pub value: String,
    #[prop_or_default]
    pub required: bool,
    pub on_change: Callback<String>,
}

#[function_component(FileFieldInput)]
pub fn file_field_input(props: &FileFieldInputProps) -> Html {
    let is_uploading = use_state(|| false);
    let upload_error = use_state(|| None::<String>);
    let file_input_ref = use_node_ref();

    let on_file_change = {
        let is_uploading = is_uploading.clone();
        let upload_error = upload_error.clone();
        let on_change = props.on_change.clone();

        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    let is_uploading = is_uploading.clone();
                    let upload_error = upload_error.clone();
                    let on_change = on_change.clone();
                    let filename = file.name();

                    is_uploading.set(true);
                    upload_error.set(None);

                    wasm_bindgen_futures::spawn_local(async move {
                        let form_data = match web_sys::FormData::new() {
                            Ok(fd) => fd,
                            Err(e) => {
                                upload_error
                                    .set(Some(format!("Failed to create form data: {:?}", e)));
                                is_uploading.set(false);
                                return;
                            }
                        };

                        if let Err(e) =
                            form_data.append_with_blob_and_filename("file", &file, &filename)
                        {
                            upload_error.set(Some(format!("Failed to attach file: {:?}", e)));
                            is_uploading.set(false);
                            return;
                        }

                        let client = ApiClient::default();
                        match client.upload_file(form_data).await {
                            Ok(storage_key) => {
                                is_uploading.set(false);
                                on_change.emit(storage_key);
                            }
                            Err(err) => {
                                is_uploading.set(false);
                                upload_error.set(Some(format!("Upload failed: {err}")));
                            }
                        }
                    });
                }
            }
        })
    };

    let on_clear = {
        let on_change = props.on_change.clone();
        let file_input_ref = file_input_ref.clone();
        Callback::from(move |_| {
            if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                input.set_value("");
            }
            on_change.emit("".to_string());
        })
    };

    let trigger_file_input = {
        let file_input_ref = file_input_ref.clone();
        Callback::from(move |_| {
            if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                input.click();
            }
        })
    };

    let has_file = !props.value.trim().is_empty();
    let is_image = {
        let lower = props.value.to_lowercase();
        lower.ends_with(".png")
            || lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".gif")
            || lower.ends_with(".webp")
            || lower.ends_with(".svg")
    };

    let file_url = if has_file {
        let base_url = ApiClient::get_base_url();
        format!("{}/files/{}", base_url.trim_end_matches('/'), props.value)
    } else {
        "".to_string()
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
                        {"Stored"}
                    </span>
                }
            </div>

            <input
                ref={file_input_ref}
                type="file"
                class="hidden"
                onchange={on_file_change}
                disabled={*is_uploading}
            />

            if has_file {
                <div class="border border-outline-variant bg-surface-container-low rounded-lg p-3 flex flex-col gap-2">
                    <div class="flex items-center justify-between gap-2">
                        <div class="flex items-center gap-2 min-w-0">
                            if is_image {
                                <img
                                    src={file_url.clone()}
                                    alt={props.label.clone()}
                                    class="w-10 h-10 object-cover rounded border border-outline-variant bg-surface shrink-0"
                                />
                            } else {
                                <div class="w-10 h-10 rounded border border-outline-variant bg-surface flex items-center justify-center shrink-0 text-primary">
                                    <span class="material-symbols-outlined text-[20px]">{"description"}</span>
                                </div>
                            }
                            <div class="min-w-0">
                                <p class="text-xs font-mono font-medium text-on-surface truncate" title={props.value.clone()}>
                                    {&props.value}
                                </p>
                                <div class="flex items-center gap-3 mt-0.5">
                                    <a
                                        href={file_url.clone()}
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        class="text-[11px] text-primary hover:underline flex items-center gap-0.5"
                                    >
                                        <span class="material-symbols-outlined text-[12px]">{"open_in_new"}</span>
                                        {"View file"}
                                    </a>
                                </div>
                            </div>
                        </div>

                        <div class="flex items-center gap-1 shrink-0">
                            <button
                                type="button"
                                onclick={trigger_file_input}
                                class="p-1.5 hover:bg-surface-container rounded text-on-surface-variant hover:text-primary transition-colors text-xs flex items-center gap-1"
                                title="Replace file"
                                disabled={*is_uploading}
                            >
                                <span class="material-symbols-outlined text-[16px]">{"upload"}</span>
                                <span class="hidden sm:inline text-[11px]">{"Replace"}</span>
                            </button>
                            <button
                                type="button"
                                onclick={on_clear}
                                class="p-1.5 hover:bg-error-container/20 rounded text-on-surface-variant hover:text-error transition-colors text-xs flex items-center gap-1"
                                title="Remove file"
                                disabled={*is_uploading}
                            >
                                <span class="material-symbols-outlined text-[16px]">{"delete"}</span>
                                <span class="hidden sm:inline text-[11px]">{"Remove"}</span>
                            </button>
                        </div>
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
                        } else {
                            {"Click to browse or drop file"}
                        }
                    </p>
                    <p class="font-label-xs text-label-xs text-on-surface-variant mt-0.5">
                        {"File will be stored and linked to this record"}
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
