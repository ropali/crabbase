use crate::models::collection::Collection;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SidebarProps {
    pub collections: Vec<Collection>,
    pub selected_collection_id: Option<String>,
    pub on_select: Callback<String>,
}

#[function_component(Sidebar)]
pub fn sidebar(props: &SidebarProps) -> Html {
    html! {
        <aside class="w-64 bg-slate-900 border-r border-slate-800 flex flex-col h-full">
            <div class="p-4 border-b border-slate-800">
                <span class="text-lg font-bold text-slate-200">{"Crabbase Admin"}</span>
            </div>
            <div class="flex-1 overflow-y-auto p-4 space-y-4">
                <div>
                    <h3 class="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-2">{"Collections"}</h3>
                    <ul class="space-y-1">
                        {
                            props.collections.iter().map(|col| {
                                let id = col.id.clone();
                                let on_click = {
                                    let on_select = props.on_select.clone();
                                    let id = id.clone();
                                    Callback::from(move |_| on_select.emit(id.clone()))
                                };
                                let is_selected = props.selected_collection_id.as_ref() == Some(&col.id);
                                let class = if is_selected {
                                    "flex items-center px-3 py-2 text-sm font-medium rounded-md bg-indigo-600 text-white cursor-pointer"
                                } else {
                                    "flex items-center px-3 py-2 text-sm font-medium rounded-md text-slate-300 hover:bg-slate-800 hover:text-white cursor-pointer"
                                };
                                html! {
                                    <li key={col.id.clone()} onclick={on_click} class={class}>
                                        <span class="truncate">{ &col.name }</span>
                                    </li>
                                }
                            }).collect::<Html>()
                        }
                    </ul>
                </div>
            </div>
        </aside>
    }
}
