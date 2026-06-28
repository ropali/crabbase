use crate::components::CollectionList;
use crate::models::collection::Collection;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SidebarProps {
    pub selected_collection_id: Option<String>,
    pub on_select: Callback<Collection>,
    pub on_create_click: Callback<()>,
    #[prop_or_default]
    pub refresh_trigger: usize,
}

#[function_component(Sidebar)]
pub fn sidebar(props: &SidebarProps) -> Html {
    let on_create_click = {
        let cb = props.on_create_click.clone();
        Callback::from(move |_| {
            cb.emit(());
        })
    };

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
            <nav class="flex flex-col gap-1 overflow-y-auto overflow-x-hidden flex-1 custom-scrollbar">
              /* List of Collections */
              <CollectionList
                  is_system={false}
                  selected_collection_id={props.selected_collection_id.clone()}
                  on_select={props.on_select.clone()}
                  refresh_trigger={props.refresh_trigger}
              />

              /* System section divider */
              <div class="mt-6 mb-2 px-3 border-t border-outline-variant pt-4">
                <span class="font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider opacity-50">{"System"}</span>
              </div>

              /*  List of System Collections */
              <CollectionList
                  is_system={true}
                  selected_collection_id={props.selected_collection_id.clone()}
                  on_select={props.on_select.clone()}
                  refresh_trigger={props.refresh_trigger}
              />


            </nav>

            <button onclick={on_create_click} id="new-collection-sidebar-btn" class="mt-auto bg-surface-container-highest hover:bg-outline-variant text-on-surface-variant font-bold border border-outline-variant rounded-lg px-3 py-2.5 flex items-center justify-center gap-2 transition-all active:scale-95">
              <span class="material-symbols-outlined text-sm">{"add"}</span>
              <span class="font-label-xs text-label-xs">{"New collection"}</span>
            </button>
          </aside>
    }
}
