use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    pub collection_name: String,
    pub on_search: Callback<String>,
    pub on_create: Callback<()>,
    #[prop_or_default]
    pub on_api_preview: Option<Callback<()>>,
}

#[function_component(PageHeader)]
pub fn page_header(props: &HeaderProps) -> Html {
    let search_input_ref = use_node_ref();
    let on_search = props.on_search.clone();
    let on_create = props.on_create.clone();
    let on_api_preview = props.on_api_preview.clone();

    let handle_search = {
        let search_input_ref = search_input_ref.clone();
        let on_search = on_search.clone();
        Callback::from(move |_| {
            if let Some(input) = search_input_ref.cast::<HtmlInputElement>() {
                on_search.emit(input.value());
            }
        })
    };

    let handle_create = Callback::from(move |_| {
        on_create.emit(());
    });

    let handle_api_preview = Callback::from(move |_| {
        if let Some(ref cb) = on_api_preview {
            cb.emit(());
        }
    });

    html! {
        <>
            <div class="px-6 py-4 flex items-center justify-between">
                <div class="flex items-center gap-2 text-body-sm font-body-sm">
                    <span class="text-on-surface-variant">{"Collections"}</span>
                    <span class="text-outline">{"/"}</span>
                    <span class="font-bold text-on-surface">{ &props.collection_name }</span>
                    <button class="material-symbols-outlined text-outline-variant hover:text-primary transition-colors ml-2">{"settings"}</button>
                    <button class="material-symbols-outlined text-outline-variant hover:text-primary transition-colors">{"refresh"}</button>
                </div>
                <div class="flex items-center gap-3">
                    <button onclick={handle_api_preview} class="border border-outline-variant bg-surface-container-lowest text-on-surface-variant font-bold px-4 py-1.5 rounded-lg font-label-xs text-label-xs hover:bg-surface-container-high transition-colors flex items-center gap-2">
                        <span class="material-symbols-outlined text-sm">{"code"}</span>
                        {"API preview"}
                    </button>
                    <button onclick={handle_create} class="bg-primary text-on-primary font-bold px-4 py-1.5 rounded-lg font-label-xs text-label-xs hover:bg-primary-container transition-colors shadow-sm active:scale-95 flex items-center gap-2">
                        <span class="material-symbols-outlined text-sm">{"add"}</span>
                        {"New record"}
                    </button>
                </div>
            </div>

            <div class="px-6 pb-4">
                <div class="bg-surface-container-high/50 border border-outline-variant rounded-xl flex items-center px-4 h-10 gap-3 group focus-within:ring-2 focus-within:ring-secondary/20 focus-within:border-secondary transition-all">
                    <span class="material-symbols-outlined text-on-surface-variant/60">{"filter_list"}</span>
                    <input
                        type="text"
                        ref={search_input_ref}
                        oninput={handle_search}
                        class="bg-transparent border-none w-full text-body-sm font-body-sm focus:ring-0 placeholder:text-on-surface-variant/40 focus:outline-none"
                        placeholder="Search term or filter..."
                    />
                    <span class="text-label-xs font-label-xs text-on-surface-variant/40 bg-surface-container-lowest px-1.5 py-0.5 rounded border border-outline-variant">{"⌘K"}</span>
                </div>
            </div>
        </>
    }
}
