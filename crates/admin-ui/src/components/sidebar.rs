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
    let stylesheet = stylist::style!(
        r#"
        .material-symbols-outlined {
              font-variation-settings: 'FILL' 0, 'wght' 400, 'GRAD' 0, 'opsz' 24;
              vertical-align: middle;
              font-size: 18px;
            }
            .custom-scrollbar::-webkit-scrollbar {
              width: 6px;
              height: 6px;
            }
            .custom-scrollbar::-webkit-scrollbar-track {
              background: transparent;
            }
            .custom-scrollbar::-webkit-scrollbar-thumb {
              background: #e2bfb8;
              border-radius: 10px;
            }
            body {
              background-color: #fff8f6;
              font-family: 'Inter', sans-serif;
            }
            /* extra micro animations and hover utilities */
            .sidebar-transition {
              transition: all 0.2s ease;
            }
            .active-nav-glow {
              box-shadow: inset 4px 0 0 #b4281c;
            }
    "#
    )
    .expect("Failed to mount style");

    html! {
        <aside class={classes!("bg-surface-container-low", "border-r", "border-outline-variant", "flex", "flex-col", "h-full", "w-[240px]", "py-4", "px-3", "gap-2", "shrink-0", "shadow-sm", stylesheet.get_class_name().to_string())} id="crabbase-sidebar">
            /* Sidebar header: collection identity */
            <div class="px-2 mb-4">
              <div class="flex items-center gap-3 mb-1">
                <span class="material-symbols-outlined text-primary" style="font-variation-settings: 'FILL' 1;">{"database"}</span>
                <span class="font-headline-md text-primary">{"Collections"}</span>
              </div>
              <p class="font-body-sm text-body-sm text-on-surface-variant opacity-70">{"Manage schema"}</p>
            </div>

            /* Search collections input (exact style from original) */
            <div class="relative mb-4">
              <span class="material-symbols-outlined absolute left-3 top-1/2 -translate-y-1/2 text-on-surface-variant text-sm">{"search"}</span>
              <input class="w-full bg-surface-container-lowest border border-outline-variant rounded-lg pl-9 pr-3 py-1.5 text-body-sm font-body-sm focus:ring-2 focus:ring-secondary focus:border-transparent outline-none" placeholder="Search collections..." type="text" id="sidebar-search-input" />
            </div>

            /* Navigation links (with active state, system section, exactly as original) */
            <nav class="flex flex-col gap-1 overflow-y-auto flex-1 custom-scrollbar">
              /* Dynamic List of Collections */
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
                        "bg-secondary-container text-on-secondary-container font-bold rounded-lg px-3 py-2 flex items-center gap-3 translate-x-1 transition-transform sidebar-nav-link cursor-pointer"
                    } else {
                        "text-on-surface-variant hover:bg-surface-container-high rounded-lg px-3 py-2 flex items-center gap-3 transition-all sidebar-nav-link cursor-pointer"
                    };

                    let icon = if col.name == "users" {
                        "person"
                    } else if col.name == "posts" {
                        "article"
                    } else if col.name == "messages" || col.name == "messagesReport" {
                        "chat"
                    } else {
                        "database"
                    };

                    let font_variation = if is_selected {
                        "font-variation-settings: 'FILL' 1;"
                    } else {
                        "font-variation-settings: 'FILL' 0;"
                    };

                    html! {
                      <a onclick={on_click} class={class} data-collection={col.name.clone()}>
                        <span class="material-symbols-outlined" style={font_variation}>{icon}</span>
                        <span class="font-label-xs text-label-xs">{&col.name}</span>
                      </a>
                    }
                }).collect::<Html>()
              }

              /* System section divider (exactly as original) */
              <div class="mt-6 mb-2 px-3 border-t border-outline-variant pt-4">
                <span class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider opacity-50">{"System"}</span>
              </div>

              <a href="#" class="text-on-surface-variant hover:bg-surface-container-high rounded-lg px-3 py-2 flex items-center gap-3 transition-all sidebar-nav-link" data-collection="docs">
                <span class="material-symbols-outlined">{"description"}</span>
                <span class="font-label-xs text-label-xs">{"Docs"}</span>
              </a>

              <a href="#" class="text-on-surface-variant hover:bg-surface-container-high rounded-lg px-3 py-2 flex items-center gap-3 transition-all sidebar-nav-link" data-collection="system">
                <span class="material-symbols-outlined">{"settings_input_component"}</span>
                <span class="font-label-xs text-label-xs">{"System"}</span>
              </a>
            </nav>

            /* New collection button (exact style, hover + active scale) */
            <button id="new-collection-sidebar-btn" class="mt-auto bg-surface-container-highest hover:bg-outline-variant text-on-surface-variant font-bold border border-outline-variant rounded-lg px-3 py-2.5 flex items-center justify-center gap-2 transition-all active:scale-95">
              <span class="material-symbols-outlined text-sm">{"add"}</span>
              <span class="font-label-xs text-label-xs">{"New collection"}</span>
            </button>
          </aside>
    }
}
