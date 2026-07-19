use crate::models::{
    collection::{Collection, CollectionListResponse, CreateCollectionRequest, RecordsResponse},
    record::{CreateRecordRequest, UpdateRecordRequest},
};
use gloo_net::http::{Request, RequestBuilder};
use std::sync::Mutex;

static TOKEN: Mutex<Option<String>> = Mutex::new(None);

pub struct ApiClient {
    base_url: String,
    token: Option<String>,
}

fn get_session_storage() -> Option<web_sys::Storage> {
    web_sys::window()
        .and_then(|w| w.session_storage().ok())
        .flatten()
}

impl ApiClient {
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

    pub fn new(base_url: String, token: Option<String>) -> Self {
        if let Some(t) = token.clone() {
            Self::set_token(Some(t));
        }
        Self { base_url, token }
    }

    fn request(&self, method: &str, path: &str) -> RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
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
        self.request("POST", "/collections")
            .json(&body)?
            .send()
            .await?
            .json::<Collection>()
            .await
    }

    pub async fn get_collections(&self) -> Result<CollectionListResponse, gloo_net::Error> {
        self.request("GET", "/collections")
            .send()
            .await?
            .json::<CollectionListResponse>()
            .await
    }

    pub async fn get_collection_by_name(&self, name: &str) -> Result<Collection, gloo_net::Error> {
        self.request("GET", &format!("/collections/{}", name))
            .send()
            .await?
            .json::<Collection>()
            .await
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

        self.request("GET", &url)
            .send()
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
        self.request("DELETE", &url)
            .send()
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

        self.request("POST", &url)
            .json(&body)?
            .send()
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

        self.request("PATCH", &url)
            .json(&body)?
            .send()
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
        self.request("PATCH", &url)
            .json(&body)?
            .send()
            .await?
            .json::<crate::models::collection::Collection>()
            .await
    }

    pub async fn delete_collection(
        &self,
        name: &str,
    ) -> Result<serde_json::Value, gloo_net::Error> {
        let url = format!("/collections/{}", name);
        self.request("DELETE", &url)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await
    }

    pub async fn truncate_collection(
        &self,
        name: &str,
    ) -> Result<serde_json::Value, gloo_net::Error> {
        let url = format!("/collections/{}/truncate", name);
        self.request("POST", &url)
            .send()
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

        let response = Request::post(&format!("{}{}", self.base_url, url))
            .json(&body)?
            .send()
            .await?;

        if !response.ok() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(gloo_net::Error::GlooError(format!(
                "HTTP Status {}: {}",
                status, text
            )));
        }

        let login_res = response.json::<serde_json::Value>().await?;
        let token = login_res
            .get("token")
            .and_then(|t| t.as_str())
            .map(|t| t.to_string())
            .ok_or_else(|| gloo_net::Error::GlooError("No token found in response".to_string()))?;

        Self::set_token(Some(token.clone()));
        Ok(token)
    }
}
