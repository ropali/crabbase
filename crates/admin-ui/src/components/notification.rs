use gloo_timers::callback::Timeout;
use yew::prelude::*;

const SUCCESS_DURATION_MS: u32 = 4000;
const ERROR_DURATION_MS: u32 = 7000;

#[derive(Debug, Clone, PartialEq)]
pub enum NotificationKind {
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NotificationMessage {
    pub id: u64,
    pub kind: NotificationKind,
    pub title: String,
    pub message: String,
}

#[derive(Properties, PartialEq)]
pub struct NotificationToastProps {
    pub notification: Option<NotificationMessage>,
    pub on_dismiss: Callback<()>,
}

#[function_component(NotificationToast)]
pub fn notification_toast(props: &NotificationToastProps) -> Html {
    let on_dismiss = props.on_dismiss.clone();
    let handle_close = Callback::from(move |_| {
        on_dismiss.emit(());
    });

    let auto_dismiss = props.notification.as_ref().map(|note| {
        let delay = match note.kind {
            NotificationKind::Success => SUCCESS_DURATION_MS,
            NotificationKind::Error => ERROR_DURATION_MS,
        };
        (note.id, delay)
    });

    {
        let on_dismiss = props.on_dismiss.clone();
        use_effect_with(auto_dismiss, move |auto_dismiss| {
            let timeout = (*auto_dismiss).map(|(_, delay)| {
                let on_dismiss = on_dismiss.clone();
                Timeout::new(delay, move || on_dismiss.emit(()))
            });
            move || drop(timeout)
        });
    }

    if let Some(ref note) = props.notification {
        let (accent_cls, icon_wrap_cls, icon_name, duration_ms) = match note.kind {
            NotificationKind::Success => (
                "bg-[#4b7a5a]",
                "bg-[#e9f1ea] text-[#3f6b4f]",
                "check_circle",
                SUCCESS_DURATION_MS,
            ),
            NotificationKind::Error => (
                "bg-error",
                "bg-error-container text-on-error-container",
                "error",
                ERROR_DURATION_MS,
            ),
        };

        html! {
            <div
                key={note.id.to_string()}
                role="status"
                aria-live="polite"
                class="fixed left-4 right-4 top-4 z-[9999] overflow-hidden rounded-xl border border-outline-variant bg-surface-container-lowest shadow-[0_8px_24px_rgba(38,24,22,0.14)] animate-fade-in sm:left-auto sm:right-6 sm:top-6 sm:w-full sm:max-w-sm"
            >
                <div class="h-1 w-full bg-surface-container-high">
                    <div
                        class={classes!("notification-progress", "h-full", accent_cls)}
                        style={format!("animation-duration: {duration_ms}ms")}
                    ></div>
                </div>
                <div class="flex items-start gap-3 p-4">
                    <div class={classes!(
                        "flex", "h-9", "w-9", "shrink-0", "items-center", "justify-center",
                        "rounded-lg", icon_wrap_cls
                    )}>
                        <span class="material-symbols-outlined text-[20px]">{icon_name}</span>
                    </div>
                    <div class="min-w-0 flex-grow">
                        <h4 class="font-body-md text-body-md font-bold text-on-surface">{ &note.title }</h4>
                        <p class="mt-0.5 break-words font-body-sm text-body-sm text-on-surface-variant">{ &note.message }</p>
                    </div>
                    <button
                        onclick={handle_close}
                        class="shrink-0 cursor-pointer rounded-md p-1 text-on-surface-variant transition-colors hover:bg-surface-container hover:text-on-surface"
                        title="Dismiss"
                    >
                        <span class="material-symbols-outlined text-[18px]">{"close"}</span>
                    </button>
                </div>
            </div>
        }
    } else {
        html! {}
    }
}
