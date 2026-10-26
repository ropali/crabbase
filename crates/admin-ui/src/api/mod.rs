pub mod client;

use gloo_timers::callback::Interval;
use yew::prelude::*;

use client::ApiClient;

const FILE_TOKEN_REFRESH_MS: u32 = 60_000;

#[derive(Clone, PartialEq)]
pub struct FileToken(pub Option<String>);

fn refresh_file_token(token: UseStateHandle<Option<String>>) {
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(value) = ApiClient::default().get_file_token().await {
            if (*token).as_deref() != Some(value.as_str()) {
                token.set(Some(value));
            }
        }
    });
}

#[derive(Properties, PartialEq)]
pub struct FileTokenProviderProps {
    #[prop_or_default]
    pub children: Children,
}

/// Fetches the short-lived file download token once per session and refreshes
/// it before expiry. Descendants read it with [`use_file_token`].
#[function_component(FileTokenProvider)]
pub fn file_token_provider(props: &FileTokenProviderProps) -> Html {
    let token = use_state(ApiClient::cached_file_token);

    {
        let token = token.clone();
        use_effect_with((), move |_| {
            refresh_file_token(token.clone());
            let interval = Interval::new(FILE_TOKEN_REFRESH_MS, move || {
                refresh_file_token(token.clone());
            });
            move || drop(interval)
        });
    }

    html! {
        <ContextProvider<FileToken> context={FileToken((*token).clone())}>
            { props.children.clone() }
        </ContextProvider<FileToken>>
    }
}

#[hook]
pub fn use_file_token() -> Option<String> {
    use_context::<FileToken>().and_then(|token| token.0)
}
