use gloo_net::http::Request;
use serde::de::DeserializeOwned;
use serde::Serialize;

const BASE_URL: &str = "/api";

pub struct ApiClient {
    token: Option<String>,
}

impl ApiClient {
    pub fn new() -> Self {
        Self { token: None }
    }

    pub fn with_token(token: String) -> Self {
        Self { token: Some(token) }
    }

    fn build_url(path: &str) -> String {
        format!("{}{}", BASE_URL, path)
    }

    // ---- Static methods (used by most pages) ----

    pub async fn get<T: DeserializeOwned>(path: &str, token: Option<&str>) -> Result<T, String> {
        let url = Self::build_url(path);
        let mut req = Request::get(&url);
        if let Some(t) = token {
            req = req.header("Authorization", &format!("Bearer {}", t));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.ok() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, body));
        }
        resp.json::<T>().await.map_err(|e| e.to_string())
    }

    pub async fn post<T: DeserializeOwned, B: Serialize>(
        path: &str,
        body: &B,
        token: Option<&str>,
    ) -> Result<T, String> {
        let url = Self::build_url(path);
        let mut req = Request::post(&url)
            .header("Content-Type", "application/json");
        if let Some(t) = token {
            req = req.header("Authorization", &format!("Bearer {}", t));
        }
        let json_body = serde_json::to_string(body).map_err(|e| e.to_string())?;
        let resp = req.body(&json_body)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, body));
        }
        resp.json::<T>().await.map_err(|e| e.to_string())
    }

    pub async fn post_no_response<B: Serialize>(
        path: &str,
        body: &B,
        token: Option<&str>,
    ) -> Result<(), String> {
        let url = Self::build_url(path);
        let mut req = Request::post(&url)
            .header("Content-Type", "application/json");
        if let Some(t) = token {
            req = req.header("Authorization", &format!("Bearer {}", t));
        }
        let json_body = serde_json::to_string(body).map_err(|e| e.to_string())?;
        let resp = req.body(&json_body)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, body));
        }
        Ok(())
    }

    pub async fn put<T: DeserializeOwned, B: Serialize>(
        path: &str,
        body: &B,
        token: Option<&str>,
    ) -> Result<T, String> {
        let url = Self::build_url(path);
        let mut req = Request::put(&url)
            .header("Content-Type", "application/json");
        if let Some(t) = token {
            req = req.header("Authorization", &format!("Bearer {}", t));
        }
        let json_body = serde_json::to_string(body).map_err(|e| e.to_string())?;
        let resp = req.body(&json_body)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, body));
        }
        resp.json::<T>().await.map_err(|e| e.to_string())
    }

    pub async fn delete(path: &str, token: Option<&str>) -> Result<(), String> {
        let url = Self::build_url(path);
        let mut req = Request::delete(&url);
        if let Some(t) = token {
            req = req.header("Authorization", &format!("Bearer {}", t));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.ok() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, body));
        }
        Ok(())
    }

    // ---- Instance methods (used by billing page) ----

    pub async fn fetch<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        Self::get(path, self.token.as_deref()).await
    }

    pub async fn send<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T, String> {
        Self::post(path, body, self.token.as_deref()).await
    }

    pub async fn send_put<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T, String> {
        Self::put(path, body, self.token.as_deref()).await
    }
}
