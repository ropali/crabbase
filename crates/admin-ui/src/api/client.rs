use crate::models::{
    collection::{Collection, CollectionListResponse, CreateCollectionRequest, RecordsResponse},
    record::{CreateRecordRequest, UpdateRecordRequest},
};
use gloo_net::http::{Request, RequestBuilder, Response};
use std::sync::Mutex;

static TOKEN: Mutex<Option<String>> = Mutex::new(None);
static BASE_URL: Mutex<Option<String>> = Mutex::new(None);

pub struct ApiClient {
    base_url: String,
    token: Option<String>,
}

fn get_local_storage() -> Option<web_sys::Storage> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
}

fn get_session_storage() -> Option<web_sys::Storage> {
    web_sys::window()
        .and_then(|w| w.session_storage().ok())
        .flatten()
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new(Self::get_base_url(), Self::get_token())
    }
}

impl ApiClient {
    pub fn set_base_url(url: String) {
        if let Ok(mut guard) = BASE_URL.lock() {
            *guard = Some(url.clone());
        }
        if let Some(storage) = get_local_storage() {
            let _ = storage.set_item("crabbase_api_url", &url);
        }
    }

    pub fn get_base_url() -> String {
        // 1. Check in-memory override
        if let Ok(guard) = BASE_URL.lock() {
            if let Some(ref url) = *guard {
                if !url.trim().is_empty() {
                    return url.clone();
                }
            }
        }

        // 2. Check localStorage override ("crabbase_api_url")
        if let Some(storage) = get_local_storage() {
            if let Ok(Some(url)) = storage.get_item("crabbase_api_url") {
                if !url.trim().is_empty() {
                    return url;
                }
            }
        }

        // 3. Check window global property window.CRABBASE_API_URL
        if let Some(window) = web_sys::window() {
            if let Ok(val) = js_sys::Reflect::get(&window, &"CRABBASE_API_URL".into()) {
                if let Some(url) = val.as_string() {
                    if !url.trim().is_empty() {
                        return url;
                    }
                }
            }
        }

        // 4. Check compile-time environment variable CRABBASE_API_URL
        if let Some(env_url) = option_env!("CRABBASE_API_URL") {
            if !env_url.trim().is_empty() {
                return env_url.to_string();
            }
        }

        // 5. Default fallback
        "/api".to_string()
    }

    pub fn set_token(token: Option<String>) {
        if let Ok(mut guard) = TOKEN.lock() {
            *guard = token.clone();
        }
        if let Some(storage) = get_session_storage() {
            if let Some(ref t) = token {
                let _ = storage.set_item("crabbase_token", t);
            } else {
                let _ = storage.remove_item("crabbase_token");
            }
        }
    }

    pub fn get_token() -> Option<String> {
        let in_mem = TOKEN.lock().ok().and_then(|guard| guard.clone());
        if in_mem.is_some() {
            return in_mem;
        }

        if let Some(storage) = get_session_storage() {
            if let Ok(Some(t)) = storage.get_item("crabbase_token") {
                if let Ok(mut guard) = TOKEN.lock() {
                    *guard = Some(t.clone());
                }
                return Some(t);
            }
        }
        None
    }

    pub fn handle_401() {
        Self::set_token(None);
        if let Some(window) = web_sys::window() {
            if let Ok(history) = window.history() {
                let _ = history.push_state_with_url(
                    &web_sys::wasm_bindgen::JsValue::NULL,
                    "",
                    Some("/login"),
                );
            }
            if let Ok(event) = web_sys::Event::new("crabbase_401_unauthorized") {
                let _ = window.dispatch_event(&event);
            }
        }
    }

    async fn check_response(response: Response) -> Result<Response, gloo_net::Error> {
        if response.status() == 401 {
            Self::handle_401();
            let text = response.text().await.unwrap_or_default();
            return Err(gloo_net::Error::GlooError(format!(
                "HTTP Status 401: Unauthorized. {}",
                text
            )));
        }
        if !response.ok() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(gloo_net::Error::GlooError(format!(
                "HTTP Status {}: {}",
                status, text
            )));
        }
        Ok(response)
    }

    pub fn new(base_url: String, token: Option<String>) -> Self {
        let final_base_url = if base_url.trim().is_empty() {
            Self::get_base_url()
        } else {
            base_url
        };
        let active_token = token.or_else(Self::get_token);
        if let Some(ref t) = active_token {
            Self::set_token(Some(t.clone()));
        }
        Self {
            base_url: final_base_url,
            token: active_token,
        }
    }

    fn request(&self, method: &str, path: &str) -> RequestBuilder {
        let base = self.base_url.trim_end_matches('/');
        let formatted_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };
        let url = format!("{}{}", base, formatted_path);
        let mut req = match method {
            "GET" => Request::get(&url),
            "POST" => Request::post(&url),
            "PATCH" => Request::patch(&url),
            "DELETE" => Request::delete(&url),
            _ => Request::get(&url),
        };

        let active_token = Self::get_token().or_else(|| self.token.clone());
        if let Some(ref jwt) = active_token {
            req = req.header("Authorization", &format!("Bearer {}", jwt));
        }
        req
    }

    pub async fn create_collection(
        &self,
        body: CreateCollectionRequest,
    ) -> Result<Collection, gloo_net::Error> {
        let res = self
            .request("POST", "/collections")
            .json(&body)?
            .send()
            .await?;
        Self::check_response(res).await?.json::<Collection>().await
    }

    pub async fn get_collections(&self) -> Result<CollectionListResponse, gloo_net::Error> {
        let res = self.request("GET", "/collections").send().await?;
        Self::check_response(res)
            .await?
            .json::<CollectionListResponse>()
            .await
    }

    pub async fn get_collection_by_name(&self, name: &str) -> Result<Collection, gloo_net::Error> {
        let res = self
            .request("GET", &format!("/collections/{}", name))
            .send()
            .await?;
        Self::check_response(res).await?.json::<Collection>().await
    }

    pub async fn get_records(
        &self,
        collection_name: &str,
        page: Option<usize>,
        per_page: Option<usize>,
    ) -> Result<RecordsResponse, gloo_net::Error> {
        let mut url = format!("/collections/{}/records", collection_name);
        let mut query = Vec::new();
        if let Some(p) = page {
            query.push(format!("page={}", p));
        }
        if let Some(pp) = per_page {
            query.push(format!("per_page={}", pp));
        }
        if !query.is_empty() {
            url = format!("{}?{}", url, query.join("&"));
        }

        let res = self.request("GET", &url).send().await?;
        Self::check_response(res)
            .await?
            .json::<RecordsResponse>()
            .await
    }

    pub async fn delete_record(
        &self,
        collection_name: &str,
        id: &str,
    ) -> Result<serde_json::Value, gloo_net::Error> {
        let url = format!("/collections/{}/records/{}", collection_name, id);
        let res = self.request("DELETE", &url).send().await?;
        Self::check_response(res)
            .await?
            .json::<serde_json::Value>()
            .await
    }

    pub async fn create_record(
        &self,
        collection_name: &str,
        body: CreateRecordRequest,
    ) -> Result<serde_json::Value, gloo_net::Error> {
        let url = format!("/collections/{}/records", collection_name);
        let res = self.request("POST", &url).json(&body)?.send().await?;
        Self::check_response(res)
            .await?
            .json::<serde_json::Value>()
            .await
    }

    pub async fn update_record(
        &self,
        collection_name: &str,
        id: &str,
        body: UpdateRecordRequest,
    ) -> Result<serde_json::Value, gloo_net::Error> {
        let url = format!("/collections/{}/records/{}", collection_name, id);
        let res = self.request("PATCH", &url).json(&body)?.send().await?;
        Self::check_response(res)
            .await?
            .json::<serde_json::Value>()
            .await
    }

    pub async fn update_collection(
        &self,
        name: &str,
        body: crate::models::collection::UpdateCollectionRequest,
    ) -> Result<crate::models::collection::Collection, gloo_net::Error> {
        let url = format!("/collections/{}", name);
        let res = self.request("PATCH", &url).json(&body)?.send().await?;
        Self::check_response(res)
            .await?
            .json::<crate::models::collection::Collection>()
            .await
    }

    pub async fn delete_collection(
        &self,
        name: &str,
    ) -> Result<serde_json::Value, gloo_net::Error> {
        let url = format!("/collections/{}", name);
        let res = self.request("DELETE", &url).send().await?;
        Self::check_response(res)
            .await?
            .json::<serde_json::Value>()
            .await
    }

    pub async fn truncate_collection(
        &self,
        name: &str,
    ) -> Result<serde_json::Value, gloo_net::Error> {
        let url = format!("/collections/{}/truncate", name);
        let res = self.request("POST", &url).send().await?;
        Self::check_response(res)
            .await?
            .json::<serde_json::Value>()
            .await
    }

    pub async fn login(
        &self,
        collection: &str,
        email: &str,
        password: &str,
    ) -> Result<String, gloo_net::Error> {
        let url = format!("/auth/{}/login", collection);
        let body = serde_json::json!({
            "email": email,
            "password": password,
        });

        let res = Request::post(&format!("{}{}", self.base_url, url))
            .json(&body)?
            .send()
            .await?;
        let response = Self::check_response(res).await?;

        let login_res = response.json::<serde_json::Value>().await?;
        let token = login_res
            .get("tokens")
            .and_then(|t| t.get("accessToken"))
            .and_then(|t| t.as_str())
            .or_else(|| login_res.get("token").and_then(|t| t.as_str()))
            .or_else(|| login_res.get("accessToken").and_then(|t| t.as_str()))
            .map(|t| t.to_string())
            .ok_or_else(|| {
                gloo_net::Error::GlooError(format!("No token found in response: {}", login_res))
            })?;

        Self::set_token(Some(token.clone()));
        Ok(token)
    }

    pub async fn forget_password(
        &self,
        collection: &str,
        email: &str,
    ) -> Result<serde_json::Value, gloo_net::Error> {
        let url = format!("/auth/{}/forget-password", collection);
        let body = serde_json::json!({
            "email": email,
        });

        let res = self.request("POST", &url).json(&body)?.send().await?;
        let response = Self::check_response(res).await?;
        response.json::<serde_json::Value>().await
    }
    pub async fn get_mail_settings(&self) -> Result<serde_json::Value, gloo_net::Error> {
        let res = self.request("GET", "/settings/mail").send().await?;
        Self::check_response(res)
            .await?
            .json::<serde_json::Value>()
            .await
    }

    pub async fn save_mail_settings(&self, body: serde_json::Value) -> Result<(), gloo_net::Error> {
        let res = self
            .request("POST", "/settings/mail")
            .json(&body)?
            .send()
            .await?;
        Self::check_response(res).await?;
        Ok(())
    }

    pub async fn get_app_settings(&self) -> Result<Option<serde_json::Value>, gloo_net::Error> {
        let res = self.request("GET", "/settings/app").send().await?;
        if res.status() == 204 {
            return Ok(None);
        }
        let data = Self::check_response(res)
            .await?
            .json::<serde_json::Value>()
            .await?;
        Ok(Some(data))
    }

    pub async fn save_app_settings(&self, body: serde_json::Value) -> Result<(), gloo_net::Error> {
        let res = self
            .request("POST", "/settings/app")
            .json(&body)?
            .send()
            .await?;
        Self::check_response(res).await?;
        Ok(())
    }
}
