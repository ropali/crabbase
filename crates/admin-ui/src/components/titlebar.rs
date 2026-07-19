use crate::routes::Route;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Properties, PartialEq)]
pub struct TitlebarProps {
    pub title: String,
    pub on_logout: Callback<()>,
}

#[function_component(Titlebar)]
pub fn titlebar(props: &TitlebarProps) -> Html {
    let is_dropdown_open = use_state(|| false);
    let navigator = use_navigator();

    let toggle_dropdown = {
        let is_dropdown_open = is_dropdown_open.clone();
        Callback::from(move |_| {
            is_dropdown_open.set(!*is_dropdown_open);
        })
    };

    let on_logout_click = {
        let is_dropdown_open = is_dropdown_open.clone();
        let on_logout = props.on_logout.clone();
        let navigator = navigator.clone();
        Callback::from(move |_| {
            is_dropdown_open.set(false);
            if let Some(ref nav) = navigator {
                nav.push(&Route::Login);
            }
            on_logout.emit(());
        })
    };

    html! {
        <header class="bg-primary dark:bg-primary-container text-on-primary dark:text-on-primary-container border-b border-outline-variant flex justify-between items-center w-full px-gutter h-12 z-50 shadow-sm shrink-0" id="crabbase-topbar">
            /* Left section: Brand + Navigation */
            <div class="flex items-center gap-6">
              /* Crabbase logo / brand name & active page title */
              <div class="flex items-center gap-2">
                <div class="font-headline-md text-headline-md font-bold text-on-primary dark:text-on-primary-container tracking-tight">
                  {"CRABBASE"}
                </div>
              </div>

              /* Desktop navigation links */
              <nav class="hidden md:flex gap-4">
                <a href="#" class="text-on-primary font-bold border-b-2 border-on-primary pb-1 font-label-xs text-label-xs transition-colors topbar-nav-link active-nav-link" data-nav="collections">{"Collections"}</a>
                <a href="#" class="text-on-primary/70 font-medium font-label-xs text-label-xs hover:bg-primary-container/20 transition-colors topbar-nav-link" data-nav="logs">{"Logs"}</a>
                <a href="#" class="text-on-primary/70 font-medium font-label-xs text-label-xs hover:bg-primary-container/20 transition-colors topbar-nav-link" data-nav="settings">{"Settings"}</a>
              </nav>
            </div>

            /* Right section: Theme toggle + User profile */
            <div class="flex items-center gap-3">
              /* Theme toggle button (light/dark simulation) */
              <button id="theme-toggle-btn" class="material-symbols-outlined hover:bg-primary-container/20 p-1 rounded-full transition-transform active:scale-90 text-on-primary" aria-label="Toggle theme">
                {"wb_sunny"}
              </button>

              /* User profile area (avatar + email) with Dropdown */
              <div class="relative">
                <div onclick={toggle_dropdown} class="flex items-center gap-2 px-2 py-1 rounded-lg hover:bg-primary-container/20 transition-colors cursor-pointer group" id="user-profile-btn">
                  <img alt="Admin profile" class="w-6 h-6 rounded-full border border-on-primary/20 object-cover" src="https://lh3.googleusercontent.com/aida-public/AB6AXuCADdhr6H-RpZTYAkFJa3AGQknbd7ZF3aAheUWbiOof8PEz0iR0QuwwwG1_LyyEBLtpdb7kJn5Zn9Owayd4r-hHl5hrwBTrqatMT6WWrh-ZFF9bGowRL7jp8LGDRrhnlCa9Hrp7bMT28VhzUGKJQSkS2O-01cXPPiuZIAwyKZgwsqNaoDHuqQ7E2MOjxC_Zra1UZw10l5iUNHqHHLK7slZ5pprZ2m6M-CGMT0SmUS4ExzxOFFwKh6VQcUxlJV5xk4Qws0TgX-BMD5mF"/>
                  <span class="font-label-xs text-label-xs font-medium hidden sm:inline-block">{"admin@crabbase.io"}</span>
                  <span class="material-symbols-outlined text-sm text-on-primary/60 group-hover:text-on-primary transition-colors">{"expand_more"}</span>
                </div>
                {
                  if *is_dropdown_open {
                    html! {
                      <div class="absolute right-0 mt-2 w-48 bg-surface-container-lowest border border-outline-variant rounded-lg shadow-lg py-1 z-50 text-on-surface">
                        <div class="px-4 py-2 border-b border-outline-variant/50">
                          <p class="font-label-xs text-label-xs uppercase tracking-wider text-on-surface-variant">{"Signed in as"}</p>
                          <p class="font-body-sm text-body-sm font-bold truncate">{"admin@crabbase.io"}</p>
                        </div>
                        <a href="#" class="flex items-center gap-2 px-4 py-2 text-body-sm hover:bg-surface-container-low transition-colors">
                          <span class="material-symbols-outlined text-[18px]">{"account_circle"}</span>
                          {"My Profile"}
                        </a>
                        <a href="#" class="flex items-center gap-2 px-4 py-2 text-body-sm hover:bg-surface-container-low transition-colors">
                          <span class="material-symbols-outlined text-[18px]">{"settings"}</span>
                          {"Settings"}
                        </a>
                        <div class="border-t border-outline-variant/50 my-1"></div>
                        <button onclick={on_logout_click} class="w-full flex items-center gap-2 px-4 py-2 text-body-sm text-error hover:bg-error-container/30 transition-colors text-left">
                          <span class="material-symbols-outlined text-[18px]">{"logout"}</span>
                          {"Logout"}
                        </button>
                      </div>
                    }
                  } else {
                    html! {}
                  }
                }
              </div>
            </div>
          </header>
    }
}
