use crate::models::collection::CollectionListResponse;
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

    pub async fn get_collections(&self) -> Result<CollectionListResponse, gloo_net::Error> {
        self.request("GET", "/collections")
            .send()
            .await?
            .json::<CollectionListResponse>()
            .await
    }
}
