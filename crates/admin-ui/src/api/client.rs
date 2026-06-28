use crate::models::{
    collection::{Collection, CollectionListResponse, CreateCollectionRequest, RecordsResponse},
    record::CreateRecordRequest,
};
use gloo_net::http::{Request, RequestBuilder};

pub struct ApiClient {
    base_url: String,
    token: Option<String>,
}

impl ApiClient {
    pub fn new(base_url: String, token: Option<String>) -> Self {
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

        if let Some(ref jwt) = self.token {
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
}
