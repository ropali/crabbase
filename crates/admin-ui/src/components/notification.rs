use yew::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct NotificationMessage {
    pub id: u64,
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
        html! {
            <div class="fixed top-5 right-5 z-50 max-w-md w-full p-4 bg-error-container text-on-error-container border border-error/30 rounded-xl shadow-xl flex items-start gap-3 transition-all duration-300">
                <div class="p-2 rounded-lg bg-error/10 text-error flex items-center justify-center shrink-0">
                    <span class="material-symbols-outlined text-[24px]">{"lock_reset"}</span>
                </div>
                <div class="flex-grow min-w-0 pr-2">
                    <h4 class="font-headline-md text-body-md font-bold text-on-error-container">{ &note.title }</h4>
                    <p class="font-body-sm text-body-sm opacity-90 mt-0.5 break-words">{ &note.message }</p>
                </div>
                <button
                    onclick={handle_close}
                    class="text-on-error-container/70 hover:text-on-error-container p-1 rounded-md hover:bg-error/10 transition-colors shrink-0"
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
