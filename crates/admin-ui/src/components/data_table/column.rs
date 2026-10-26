use super::cell_value::CellValue;
use crate::api::client::ApiClient;
use crate::api::use_file_token;
use std::rc::Rc;
use yew::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ViewCellPayload {
    pub field_name: String,
    pub data_type: String,
    pub value: String,
}

#[derive(Clone)]
pub struct ColumnDef {
    pub key: String,
    pub header: String,
    pub data_type: String,
    pub icon: Option<&'static str>,
    pub sortable: bool,
    pub render: Rc<dyn Fn(&CellValue, Option<&Callback<ViewCellPayload>>) -> Html>,
}

impl PartialEq for ColumnDef {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.header == other.header
            && self.data_type == other.data_type
            && self.icon == other.icon
            && self.sortable == other.sortable
    }
}

impl ColumnDef {
    pub fn new(key: &str, header: &str) -> Self {
        Self {
            key: key.to_string(),
            header: header.to_string(),
            data_type: "text".to_string(),
            icon: None,
            render: Rc::new(default_render),
            sortable: false,
        }
    }

    pub fn data_type(mut self, dt: &str) -> Self {
        self.data_type = dt.to_string();
        self
    }

    pub fn render(
        mut self,
        f: impl Fn(&CellValue, Option<&Callback<ViewCellPayload>>) -> Html + 'static,
    ) -> Self {
        self.render = Rc::new(f);
        self
    }

    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }
}

pub fn default_render(v: &CellValue, _on_view: Option<&Callback<ViewCellPayload>>) -> Html {
    html! {
        <span class="font-body-sm text-body-sm text-on-surface">{ v.display() }</span>
    }
}

pub fn id_render(v: &CellValue, _on_view: Option<&Callback<ViewCellPayload>>) -> Html {
    html! {
        <span class="font-code-md text-code-md bg-surface-container-high/50 px-2 py-0.5 rounded border border-outline-variant/30 text-on-surface">
            { v.display() }
        </span>
    }
}

pub fn bool_render(v: &CellValue, _on_view: Option<&Callback<ViewCellPayload>>) -> Html {
    match v.as_bool() {
        Some(true) => html! {
            <span class="bg-[#e6f4ea] text-[#1e7e34] px-2 py-0.5 rounded font-label-xs text-label-xs font-semibold">
                { "True" }
            </span>
        },
        _ => html! {
            <span class="bg-surface-container-highest text-on-surface-variant px-2 py-0.5 rounded font-label-xs text-label-xs">
                { v.display() }
            </span>
        },
    }
}

pub fn date_render(v: &CellValue, _on_view: Option<&Callback<ViewCellPayload>>) -> Html {
    html! {
        <span class="font-label-xs text-label-xs text-on-surface-variant">{ v.display() }</span>
    }
}

pub fn code_render(v: &CellValue, _on_view: Option<&Callback<ViewCellPayload>>) -> Html {
    html! {
        <span class="font-code-md text-code-md text-on-surface-variant">{ v.display() }</span>
    }
}

pub fn nullable_text_render(v: &CellValue, _on_view: Option<&Callback<ViewCellPayload>>) -> Html {
    if v.is_null() {
        html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> }
    } else {
        html! { <span class="font-body-sm text-body-sm">{ v.display() }</span> }
    }
}

pub fn clipped_text_render(
    _field_name: String,
    _data_type: String,
    v: &CellValue,
    _on_view: Option<&Callback<ViewCellPayload>>,
) -> Html {
    if v.is_null() {
        return html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> };
    }

    let text = v.display();
    let is_long = text.len() > 30 || text.contains('\n') || v.is_json();

    if is_long {
        let display_text = if text.len() > 30 {
            format!("{}...", &text[..30])
        } else {
            text.clone()
        };

        html! {
            <div
                title={text.clone()}
                class="group flex items-center gap-1.5 px-2 py-1 rounded-lg border border-transparent text-left transition-all max-w-[220px]"
            >
                <span class="font-body-sm text-body-sm text-on-surface truncate">
                    { display_text }
                </span>
                <span class="material-symbols-outlined text-[14px] text-on-surface-variant/60 group-hover:text-primary transition-colors flex-shrink-0">
                    {"unfold_more"}
                </span>
            </div>
        }
    } else {
        html! {
            <span class="font-body-sm text-body-sm text-on-surface">{ text }</span>
        }
    }
}

pub fn json_render(
    _field_name: String,
    v: &CellValue,
    _on_view: Option<&Callback<ViewCellPayload>>,
) -> Html {
    if v.is_null() {
        return html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> };
    }

    let raw_text = v.display();

    let is_empty = raw_text == "{}" || raw_text == "[]" || raw_text.is_empty();
    let badge_text = if is_empty {
        "{} empty".to_string()
    } else if raw_text.len() > 25 {
        format!("{}...", &raw_text[..25])
    } else {
        raw_text.clone()
    };

    html! {
        <div
            title="Click row to view & edit JSON in sidebar"
            class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-surface-container-high/60 border border-outline-variant/40 font-code-md text-code-md text-on-surface transition-all max-w-[220px] group"
        >
            <span class="material-symbols-outlined text-sm text-primary group-hover:scale-110 transition-transform">
                {"data_object"}
            </span>
            <span class="truncate font-mono text-xs">
                { badge_text }
            </span>
        </div>
    }
}

pub fn richtext_render(
    _field_name: String,
    v: &CellValue,
    _on_view: Option<&Callback<ViewCellPayload>>,
) -> Html {
    if v.is_null() {
        return html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> };
    }

    let text = v.display();
    let display_text = if text.len() > 25 {
        format!("{}...", &text[..25])
    } else {
        text.clone()
    };

    html! {
        <div
            title="Click row to view & edit content in sidebar"
            class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-secondary-container/30 border border-secondary/20 font-body-sm text-body-sm text-on-surface transition-all max-w-[220px] group"
        >
            <span class="material-symbols-outlined text-sm text-secondary group-hover:scale-110 transition-transform">
                {"article"}
            </span>
            <span class="truncate text-xs font-medium">
                { display_text }
            </span>
        </div>
    }
}

pub fn relation_render(
    v: &CellValue,
    is_multiple: bool,
    _on_view: Option<&Callback<ViewCellPayload>>,
) -> Html {
    if v.is_null() {
        return html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> };
    }

    if is_multiple {
        let items: Vec<String> = match v {
            CellValue::Json(serde_json::Value::Array(arr)) => arr
                .iter()
                .filter_map(|x| match x {
                    serde_json::Value::String(s) => Some(s.clone()),
                    other => Some(other.to_string()),
                })
                .collect(),
            CellValue::Text(s) => {
                if let Ok(arr) = serde_json::from_str::<Vec<String>>(s) {
                    arr
                } else if s.trim().is_empty() || s == "[]" {
                    vec![]
                } else {
                    s.split(',')
                        .map(|item| item.trim().to_string())
                        .filter(|item| !item.is_empty())
                        .collect()
                }
            }
            _ => vec![],
        };

        if items.is_empty() {
            html! {
                <span class="inline-flex items-center gap-1 text-on-surface-variant/50 text-xs italic">
                    <span class="material-symbols-outlined text-[14px]">{"link_off"}</span>
                    <span>{ "0 relations" }</span>
                </span>
            }
        } else if items.len() == 1 {
            html! {
                <div class="inline-flex items-center gap-1.5 bg-primary/10 text-primary border border-primary/20 px-2 py-0.5 rounded font-mono text-xs">
                    <span class="material-symbols-outlined text-[13px]">{"link"}</span>
                    <span class="truncate max-w-[130px]" title={items[0].clone()}>{ &items[0] }</span>
                </div>
            }
        } else {
            let tooltip = items.join(", ");
            html! {
                <div
                    title={tooltip}
                    class="inline-flex items-center gap-1.5 bg-primary/10 text-primary border border-primary/20 px-2.5 py-0.5 rounded-full font-mono text-xs font-semibold"
                >
                    <span class="material-symbols-outlined text-[13px]">{"library_add"}</span>
                    <span>{ format!("{} relations", items.len()) }</span>
                </div>
            }
        }
    } else {
        let text = v.display();
        if text.is_empty() || text == "null" {
            return html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> };
        }
        html! {
            <div class="inline-flex items-center gap-1 bg-surface-container-high/60 px-2 py-0.5 rounded border border-outline-variant/40 font-code-md text-code-md text-on-surface max-w-[180px]">
                <span class="material-symbols-outlined text-[13px] text-primary shrink-0">{"link"}</span>
                <span class="truncate font-mono text-xs" title={text.clone()}>{ text }</span>
            </div>
        }
    }
}

#[derive(Clone, PartialEq)]
struct FileItemRender {
    key: String,
    name: String,
    path: String,
    ctype: String,
    size_bytes: Option<u64>,
}

impl FileItemRender {
    fn bare(key: &str) -> Self {
        let name = key.rsplit('/').next().unwrap_or(key).to_string();
        Self {
            key: key.to_string(),
            name,
            path: String::new(),
            ctype: String::new(),
            size_bytes: None,
        }
    }

    fn filename(&self) -> String {
        if self.name.trim().is_empty() {
            self.key.rsplit('/').next().unwrap_or(&self.key).to_string()
        } else {
            self.name.clone()
        }
    }

    fn file_url(
        &self,
        base_trimmed: &str,
        collection_name: &str,
        token: Option<&str>,
        download: bool,
    ) -> String {
        if self.path.starts_with("http://") || self.path.starts_with("https://") {
            return self.path.clone();
        }
        let url = if self.path.starts_with('/') {
            format!("{}{}", base_trimmed, self.path)
        } else if self.key.starts_with("http://") || self.key.starts_with("https://") {
            return self.key.clone();
        } else if self.key.starts_with('/') {
            format!("{}{}", base_trimmed, self.key)
        } else if !collection_name.trim().is_empty() {
            format!(
                "{}/collections/{}/files/{}",
                base_trimmed, collection_name, self.key
            )
        } else {
            format!("{}/files/{}", base_trimmed, self.key)
        };
        ApiClient::with_file_token(url, token, download)
    }

    fn api_url(&self, base_trimmed: &str, token: Option<&str>, download: bool) -> String {
        let key = if !self.key.trim().is_empty() {
            self.key.trim()
        } else {
            self.path.trim()
        };
        let url = if key.starts_with("http://") || key.starts_with("https://") {
            return key.to_string();
        } else if key.starts_with('/') {
            format!("{}{}", base_trimmed, key)
        } else {
            let encoded: String = js_sys::encode_uri_component(key).into();
            format!("{}/files/{}", base_trimmed, encoded)
        };
        ApiClient::with_file_token(url, token, download)
    }

    fn is_image(&self) -> bool {
        let lower_name = self.filename().to_lowercase();
        let lower_key = self.key.to_lowercase();
        self.ctype.starts_with("image/")
            || [".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".avif"]
                .iter()
                .any(|ext| lower_name.ends_with(ext) || lower_key.ends_with(ext))
    }
}

fn file_type_icon(item: &FileItemRender) -> &'static str {
    let name = item.filename().to_lowercase();
    let ctype = item.ctype.to_lowercase();
    if item.is_image() {
        "image"
    } else if ctype.starts_with("video/") {
        "movie"
    } else if ctype.starts_with("audio/") {
        "audiotrack"
    } else if name.ends_with(".pdf") || ctype == "application/pdf" {
        "picture_as_pdf"
    } else if [".doc", ".docx", ".odt", ".rtf", ".txt", ".md"]
        .iter()
        .any(|ext| name.ends_with(ext))
    {
        "description"
    } else if [".xls", ".xlsx", ".csv", ".ods"]
        .iter()
        .any(|ext| name.ends_with(ext))
    {
        "table_chart"
    } else if [".ppt", ".pptx", ".odp"]
        .iter()
        .any(|ext| name.ends_with(ext))
    {
        "slideshow"
    } else if [".zip", ".rar", ".7z", ".tar", ".gz"]
        .iter()
        .any(|ext| name.ends_with(ext))
    {
        "folder_zip"
    } else if [".json", ".xml", ".yaml", ".yml"]
        .iter()
        .any(|ext| name.ends_with(ext))
    {
        "data_object"
    } else {
        "draft"
    }
}

fn format_size(bytes: Option<u64>) -> Option<String> {
    let bytes = bytes?;
    let value = bytes as f64;
    Some(if bytes >= 1024 * 1024 {
        format!("{:.1} MB", value / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", value / 1024.0)
    } else {
        format!("{bytes} B")
    })
}

fn render_file_chip(
    item: &FileItemRender,
    collection_name: &str,
    base_trimmed: &str,
    stop_prop: &Callback<MouseEvent>,
    token: Option<&str>,
) -> Html {
    let filename = item.filename();
    let file_url = item.file_url(base_trimmed, collection_name, token, true);
    let api_url = item.api_url(base_trimmed, token, false);
    let display_name = if filename.chars().count() > 18 {
        format!("{}...", filename.chars().take(15).collect::<String>())
    } else {
        filename.clone()
    };

    let stop_prop_click = stop_prop.clone();
    let stop_prop_dl = stop_prop.clone();
    let storage_key = item.key.clone();

    if item.is_image() {
        html! {
            <div class="inline-flex items-center gap-1.5" title={storage_key.clone()}>
                <a
                    href={api_url.clone()}
                    target="_blank"
                    rel="noopener noreferrer"
                    onclick={stop_prop_click}
                    class="shrink-0 relative group/thumb"
                    title={format!("View image: {}", filename)}
                >
                    <img
                        src={api_url.clone()}
                        alt={filename.clone()}
                        class="w-7 h-7 object-cover rounded border border-outline-variant bg-surface hover:scale-125 transition-transform shadow-xs"
                    />
                </a>
                <span class="font-mono text-xs text-on-surface truncate min-w-0 max-w-[140px]" title={filename.clone()}>
                    { display_name }
                </span>
                <a
                    href={file_url.clone()}
                    download={filename.clone()}
                    onclick={stop_prop_dl}
                    class="p-1 rounded text-on-surface-variant hover:text-primary hover:bg-surface-container-high transition-colors shrink-0 flex items-center justify-center"
                    title={format!("Download {}", filename)}
                >
                    <span class="material-symbols-outlined text-[15px]">{"download"}</span>
                </a>
            </div>
        }
    } else {
        html! {
            <div class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded bg-surface-container-high/60 border border-outline-variant/40 max-w-[220px]" title={storage_key.clone()}>
                <span class="material-symbols-outlined text-[16px] text-primary shrink-0">
                    {"description"}
                </span>
                <span class="font-mono text-xs text-on-surface truncate min-w-0" title={filename.clone()}>
                    { display_name }
                </span>
                <a
                    href={file_url.clone()}
                    download={filename.clone()}
                    onclick={stop_prop_dl}
                    class="p-0.5 rounded text-on-surface-variant hover:text-primary hover:bg-surface-container transition-colors shrink-0 flex items-center justify-center ml-0.5"
                    title={format!("Download {}", filename)}
                >
                    <span class="material-symbols-outlined text-[15px]">{"download"}</span>
                </a>
            </div>
        }
    }
}

#[derive(Properties, PartialEq)]
struct FileChipProps {
    item: FileItemRender,
    collection_name: String,
}

#[function_component(FileChip)]
fn file_chip(props: &FileChipProps) -> Html {
    let token = use_file_token();
    let base_url = ApiClient::get_base_url();
    let base_trimmed = base_url.trim_end_matches('/').to_string();
    let stop_prop = Callback::from(|e: MouseEvent| {
        e.stop_propagation();
    });

    render_file_chip(
        &props.item,
        &props.collection_name,
        &base_trimmed,
        &stop_prop,
        token.as_deref(),
    )
}

#[derive(Properties, PartialEq)]
struct FileGalleryProps {
    collection_name: String,
    field_name: String,
    items: Vec<FileItemRender>,
}

#[function_component(FileGallery)]
fn file_gallery(props: &FileGalleryProps) -> Html {
    let open = use_state(|| false);
    let token = use_file_token();
    let base_url = ApiClient::get_base_url();
    let base_trimmed = base_url.trim_end_matches('/').to_string();

    let stop_prop = Callback::from(|e: MouseEvent| {
        e.stop_propagation();
    });

    let on_open = {
        let open = open.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            open.set(true);
        })
    };

    let on_close = {
        let open = open.clone();
        Callback::from(move |_| open.set(false))
    };

    let on_backdrop = {
        let on_close = on_close.clone();
        Callback::from(move |_| on_close.emit(()))
    };

    let on_modal_click = Callback::from(|e: MouseEvent| e.stop_propagation());

    let first = props.items.first().cloned();
    let extra = props.items.len().saturating_sub(1);
    let tooltip = props
        .items
        .iter()
        .map(|item| item.filename())
        .collect::<Vec<_>>()
        .join(", ");

    html! {
        <>
            <div class="flex items-center gap-1.5 flex-wrap cursor-pointer" title={tooltip} onclick={on_open}>
                if let Some(first) = first {
                    { render_file_chip(
                        &first,
                        &props.collection_name,
                        &base_trimmed,
                        &stop_prop,
                        token.as_deref(),
                    ) }
                }
                <span class="text-xs text-on-surface-variant bg-surface-container px-1.5 py-0.5 rounded font-mono hover:bg-surface-container-high hover:text-primary transition-colors">
                    { format!("+{extra}") }
                </span>
            </div>
            if *open {
                <div onclick={on_backdrop} class="fixed inset-0 bg-inverse-surface/30 backdrop-blur-sm z-50 flex items-center justify-center p-4">
                    <div onclick={on_modal_click} class="bg-surface border border-outline-variant rounded-2xl shadow-2xl w-full max-w-2xl flex flex-col max-h-[85vh] overflow-hidden animate-scale-in">
                        <div class="px-6 py-4 border-b border-outline-variant flex items-center justify-between bg-surface-container-low/50">
                            <div class="flex items-center gap-3">
                                <div class="w-9 h-9 rounded-xl bg-primary/10 text-primary flex items-center justify-center">
                                    <span class="material-symbols-outlined text-xl">{"folder_open"}</span>
                                </div>
                                <div>
                                    <div class="flex items-center gap-2">
                                        <h3 class="font-bold text-on-surface text-lg">{ &props.field_name }</h3>
                                        <span class="bg-primary-container text-on-primary-container px-2 py-0.5 rounded-full font-label-xs text-[10px] uppercase font-bold tracking-wider">
                                            { format!("{} files", props.items.len()) }
                                        </span>
                                    </div>
                                    <p class="font-label-xs text-label-xs text-on-surface-variant">
                                        { format!("Files in {}", props.collection_name) }
                                    </p>
                                </div>
                            </div>
                            <button onclick={let cb = on_close.clone(); move |_| cb.emit(())} class="p-1.5 hover:bg-surface-container-high rounded-full transition-colors text-on-surface-variant">
                                <span class="material-symbols-outlined">{"close"}</span>
                            </button>
                        </div>

                        <div class="flex-1 p-4 overflow-y-auto bg-surface-container-lowest custom-scrollbar grid grid-cols-2 sm:grid-cols-3 gap-3 content-start">
                            { for props.items.iter().map(|item| {
                                let filename = item.filename();
                                let view_url = item.api_url(&base_trimmed, token.as_deref(), false);
                                let download_url = item.api_url(&base_trimmed, token.as_deref(), true);
                                let is_image = item.is_image();
                                let size = format_size(item.size_bytes);
                                let meta = match (item.ctype.trim(), size) {
                                    ("", Some(size)) => size,
                                    (ctype, Some(size)) => format!("{ctype} · {size}"),
                                    ("", None) => "Unknown type".to_string(),
                                    (ctype, None) => ctype.to_string(),
                                };

                                html! {
                                    <div class="flex flex-col rounded-xl border border-outline-variant/60 bg-surface-container-lowest hover:border-primary/40 hover:bg-surface-container-low transition-colors overflow-hidden">
                                        if is_image {
                                            <a
                                                href={view_url.clone()}
                                                target="_blank"
                                                rel="noopener noreferrer"
                                                class="block shrink-0"
                                                title={format!("View {}", filename)}
                                            >
                                                <img
                                                    src={view_url.clone()}
                                                    alt={filename.clone()}
                                                    class="w-full h-32 object-cover bg-surface-container-high"
                                                />
                                            </a>
                                        } else {
                                            <div class="w-full h-32 shrink-0 bg-surface-container-high flex items-center justify-center">
                                                <span class="material-symbols-outlined text-5xl text-primary/70">
                                                    { file_type_icon(item) }
                                                </span>
                                            </div>
                                        }
                                        <div class="p-3 flex flex-col gap-2 min-w-0 flex-1">
                                            <div class="min-w-0">
                                                <p class="font-body-sm text-body-sm font-semibold text-on-surface truncate" title={filename.clone()}>
                                                    { filename.clone() }
                                                </p>
                                                <p class="font-mono text-[11px] text-on-surface-variant truncate" title={item.key.clone()}>
                                                    { meta }
                                                </p>
                                            </div>
                                            <a
                                                href={download_url}
                                                download={filename.clone()}
                                                onclick={stop_prop.clone()}
                                                class="mt-auto w-full px-3 py-1.5 rounded-lg border border-outline-variant hover:bg-surface-container-high hover:text-primary transition-colors font-label-xs text-label-xs flex items-center justify-center gap-1.5 text-on-surface"
                                                title={format!("Download {}", filename)}
                                            >
                                                <span class="material-symbols-outlined text-sm">{"download"}</span>
                                                { "Download" }
                                            </a>
                                        </div>
                                    </div>
                                }
                            }) }
                        </div>

                        <div class="px-6 py-3 border-t border-outline-variant bg-surface-container-low/30 flex justify-end">
                            <button
                                onclick={let cb = on_close.clone(); move |_| cb.emit(())}
                                class="px-6 py-2 bg-primary text-on-primary font-bold rounded-lg text-body-sm hover:bg-primary-container transition-colors shadow-sm"
                            >
                                {"Done"}
                            </button>
                        </div>
                    </div>
                </div>
            }
        </>
    }
}

pub fn file_render(
    collection_name: String,
    field_name: String,
    v: &CellValue,
    _on_view: Option<&Callback<ViewCellPayload>>,
) -> Html {
    let na =
        || html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> };

    if v.is_null() {
        return na();
    }

    let make_item = |value: &serde_json::Value| -> Option<FileItemRender> {
        if let Some(obj) = value.as_object() {
            let id = obj.get("id").and_then(|x| x.as_str()).unwrap_or("");
            let path = obj.get("path").and_then(|x| x.as_str()).unwrap_or("");
            let storage_key = obj.get("key").and_then(|x| x.as_str()).unwrap_or("");
            if id.is_empty() && path.is_empty() && storage_key.is_empty() {
                return None;
            }
            let key = if !storage_key.is_empty() {
                storage_key
            } else if !id.is_empty() {
                id
            } else {
                path
            };
            Some(FileItemRender {
                key: key.to_string(),
                name: obj
                    .get("name")
                    .or_else(|| obj.get("originalName"))
                    .and_then(|x| x.as_str())
                    .unwrap_or(key)
                    .to_string(),
                path: path.to_string(),
                ctype: obj
                    .get("type")
                    .or_else(|| obj.get("contentType"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                size_bytes: obj.get("sizeBytes").and_then(|x| x.as_u64()),
            })
        } else if let Some(s) = value.as_str() {
            if s.trim().is_empty() {
                None
            } else {
                Some(FileItemRender::bare(s))
            }
        } else {
            None
        }
    };

    let items: Vec<FileItemRender> = match v {
        CellValue::Json(val) => match val {
            serde_json::Value::Array(arr) => arr.iter().filter_map(make_item).collect(),
            serde_json::Value::Object(_) => make_item(val).into_iter().collect(),
            _ => val
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .map(|s| vec![FileItemRender::bare(s)])
                .unwrap_or_default(),
        },
        _ => {
            let text = v.display();
            if text.trim().is_empty() || text == "null" || text == "N/A" {
                return na();
            }
            if text.trim_start().starts_with('[') {
                if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                    arr.iter().filter_map(make_item).collect()
                } else {
                    vec![FileItemRender::bare(&text)]
                }
            } else {
                vec![FileItemRender::bare(&text)]
            }
        }
    };

    if items.is_empty() {
        return na();
    }

    if items.len() == 1 {
        html! {
            <FileChip item={items[0].clone()} collection_name={collection_name} />
        }
    } else {
        html! {
            <FileGallery
                collection_name={collection_name}
                field_name={field_name}
                items={items}
            />
        }
    }
}
