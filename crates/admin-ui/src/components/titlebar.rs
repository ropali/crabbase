use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct TitlebarProps {
    pub title: String,
}

#[function_component(Titlebar)]
pub fn titlebar(props: &TitlebarProps) -> Html {
    html! {
        <header class="h-16 bg-slate-900 border-b border-slate-800 flex items-center justify-between px-6 flex-shrink-0">
            <h1 class="text-lg font-semibold text-slate-100">{ &props.title }</h1>
            <div class="flex items-center space-x-4">
                <span class="px-2.5 py-0.5 rounded-full text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                    {"Connected"}
                </span>
            </div>
        </header>
    }
}
