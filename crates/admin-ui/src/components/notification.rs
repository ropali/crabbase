use yew::prelude::*;

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

    if let Some(ref note) = props.notification {
        let (container_cls, icon_wrap_cls, icon_name) = match note.kind {
            NotificationKind::Success => (
                // Explicit green — unaffected by the red primary theme
                "fixed bottom-6 right-6 z-[9999] max-w-md w-full p-4 bg-[#1a3a2a] text-[#d4f0e0] border border-[#2d6b47]/60 rounded-xl shadow-2xl flex items-start gap-3",
                "p-2 rounded-lg bg-[#2d6b47]/30 text-[#4ade80] flex items-center justify-center shrink-0",
                "check_circle",
            ),
            NotificationKind::Error => (
                "fixed bottom-6 right-6 z-[9999] max-w-md w-full p-4 bg-error-container text-on-error-container border border-error/30 rounded-xl shadow-2xl flex items-start gap-3",
                "p-2 rounded-lg bg-error/10 text-error flex items-center justify-center shrink-0",
                "error",
            ),
        };

        html! {
            <div class={container_cls}>
                <div class={icon_wrap_cls}>
                    <span class="material-symbols-outlined text-[22px]">{icon_name}</span>
                </div>
                <div class="flex-grow min-w-0 pr-2">
                    <h4 class="font-body-md text-body-md font-bold">{ &note.title }</h4>
                    <p class="font-body-sm text-body-sm opacity-80 mt-0.5 break-words">{ &note.message }</p>
                </div>
                <button
                    onclick={handle_close}
                    class="opacity-60 hover:opacity-100 p-1 rounded-md transition-opacity shrink-0 cursor-pointer"
                    title="Dismiss"
                >
                    <span class="material-symbols-outlined text-[20px]">{"close"}</span>
                </button>
            </div>
        }
    } else {
        html! {}
    }
}
