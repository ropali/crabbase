use web_sys::{HtmlInputElement, SubmitEvent};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    pub collection_name: String,
    pub on_search: Callback<String>,
    pub on_create: Callback<()>,
    #[prop_or_default]
    pub on_api_preview: Option<Callback<()>>,
    #[prop_or_default]
    pub on_settings: Option<Callback<()>>,
    #[prop_or_default]
    pub on_refresh: Option<Callback<()>>,
}

#[function_component(PageHeader)]
pub fn page_header(props: &HeaderProps) -> Html {
    let search_input_ref = use_node_ref();
    let search_text = use_state(String::new);

    let on_search = props.on_search.clone();
    let on_create = props.on_create.clone();
    let on_api_preview = props.on_api_preview.clone();
    let on_settings = props.on_settings.clone();
    let on_refresh = props.on_refresh.clone();

    // Reset search input when collection changes
    {
        let search_text = search_text.clone();
        let search_input_ref = search_input_ref.clone();
        let col_name = props.collection_name.clone();
        use_effect_with(col_name, move |_| {
            search_text.set(String::new());
            if let Some(input) = search_input_ref.cast::<HtmlInputElement>() {
                input.set_value("");
            }
            || ()
        });
    }

    let handle_input = {
        let search_input_ref = search_input_ref.clone();
        let search_text = search_text.clone();
        Callback::from(move |_| {
            if let Some(input) = search_input_ref.cast::<HtmlInputElement>() {
                search_text.set(input.value());
            }
        })
    };

    let handle_submit = {
        let search_input_ref = search_input_ref.clone();
        let on_search = on_search.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if let Some(input) = search_input_ref.cast::<HtmlInputElement>() {
                on_search.emit(input.value());
            }
        })
    };

    let handle_keydown = {
        let search_input_ref = search_input_ref.clone();
        let on_search = on_search.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                e.prevent_default();
                if let Some(input) = search_input_ref.cast::<HtmlInputElement>() {
                    on_search.emit(input.value());
                }
            }
        })
    };

    let handle_clear = {
        let search_input_ref = search_input_ref.clone();
        let on_search = on_search.clone();
        let search_text = search_text.clone();
        Callback::from(move |_| {
            search_text.set(String::new());
            if let Some(input) = search_input_ref.cast::<HtmlInputElement>() {
                input.set_value("");
                let _ = input.focus();
            }
            on_search.emit(String::new());
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

    let handle_settings = Callback::from(move |_| {
        if let Some(ref cb) = on_settings {
            cb.emit(());
        }
    });

    let handle_refresh = Callback::from(move |_| {
        if let Some(ref cb) = on_refresh {
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
                    <button onclick={handle_settings} class="material-symbols-outlined text-outline-variant hover:text-primary transition-colors ml-2">{"settings"}</button>
                    <button onclick={handle_refresh} class="material-symbols-outlined text-outline-variant hover:text-primary transition-colors">{"refresh"}</button>
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
                <form onsubmit={handle_submit} class="bg-surface-container-high/50 border border-outline-variant rounded-xl flex items-center px-4 h-10 gap-3 group focus-within:ring-2 focus-within:ring-secondary/20 focus-within:border-secondary transition-all">
                    <span class="material-symbols-outlined text-on-surface-variant/60 text-lg">{"filter_list"}</span>
                    <input
                        type="text"
                        ref={search_input_ref}
                        oninput={handle_input}
                        onkeydown={handle_keydown}
                        class="bg-transparent border-none w-full text-body-sm font-body-sm focus:ring-0 placeholder:text-on-surface-variant/40 focus:outline-none"
                        placeholder="Filter records (e.g. status = 'active' || title ~ 'john' || views > 10)..."
                    />
                    if !search_text.is_empty() {
                        <button
                            onclick={handle_clear}
                            type="button"
                            title="Clear filter"
                            class="text-on-surface-variant/50 hover:text-on-surface transition-colors p-1 rounded-full flex items-center justify-center"
                        >
                            <span class="material-symbols-outlined text-base">{"close"}</span>
                        </button>
                    }
                    <button
                        type="submit"
                        title="Apply filter (Enter)"
                        class="px-2.5 py-1 bg-surface-container-lowest hover:bg-surface-container text-on-surface-variant hover:text-on-surface font-label-xs text-label-xs rounded-lg border border-outline-variant flex items-center gap-1.5 transition-colors shadow-xs"
                    >
                        <span class="material-symbols-outlined text-xs">{"search"}</span>
                        <span>{"Filter"}</span>
                        <span class="text-[10px] opacity-60 ml-0.5">{"↵"}</span>
                    </button>
                </form>
            </div>
        </>
    }
}
