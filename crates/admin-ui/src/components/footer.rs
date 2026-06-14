use chrono::Datelike;
use yew::prelude::*;

#[function_component(Footer)]
pub fn footer() -> Html {
    let stylesheet = stylist::style!(
        r#"
        .footer-link {
              transition: all 0.2s ease;
              position: relative;
            }
            .footer-link:hover {
              color: #ab2815;
            }
    "#
    )
    .expect("Failed to mount style");

    let current_year = chrono::Utc::now().year();

    html! {
        <footer class={classes!("bg-surface-container-lowest", "dark:bg-surface-dim", "border-t", "border-outline-variant", "flex", "justify-between", "items-center", "w-full", "px-gutter", "py-2", "shrink-0", stylesheet.get_class_name().to_string())} id="crabbase-footer">
            /* Left side: copyright and version */
            <div class="font-label-xs text-label-xs text-on-surface-variant">
              {format!("© {current_year} • Crabbase Admin • v{}", env!("CARGO_PKG_VERSION"))}
            </div>

            /*  Right side: footer navigation links */
            <div class="flex gap-4">
              <a href="#" class="font-label-xs text-label-xs text-on-surface-variant hover:text-primary underline underline-offset-4 transition-colors footer-link" data-footer-link="docs">{"Documentation"}</a>
              <a href="#" class="font-label-xs text-label-xs text-on-surface-variant hover:text-primary underline underline-offset-4 transition-colors footer-link" data-footer-link="github">{"GitHub"}</a>
              <a href="#" class="font-label-xs text-label-xs text-off-surface-variant hover:text-primary underline underline-offset-4 transition-colors footer-link" data-footer-link="feedback">{"Feedback"}</a>
            </div>
          </footer>

    }
}
