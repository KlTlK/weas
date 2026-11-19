use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficInfo {
    pub up: u64,
    pub down: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayResponse {
    pub delay: u64,
}

#[allow(dead_code)]
pub struct ClashApiClient {
    base_url: String,
    client: reqwest::Client,
}

#[allow(dead_code)]
impl ClashApiClient {
    pub fn new(port: u16) -> Self {
        Self {
            base_url: format!("http://127.0.0.1:{}", port),
            client: reqwest::Client::new(),
        }
    }

    pub async fn check_health(&self) -> bool {
        let url = format!("{}/version", self.base_url);
        self.client.get(url).send().await.map(|r| r.status().is_success()).unwrap_or(false)
    }

    pub async fn get_traffic(&self) -> Result<TrafficInfo> {
        let url = format!("{}/traffic", self.base_url);
        let resp = self.client.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("Clash API returned status {}", resp.status()));
        }
        let info = resp.json::<TrafficInfo>().await?;
        Ok(info)
    }

    pub async fn ping_proxy(&self, proxy_name: &str, test_url: Option<&str>) -> Result<u64> {
        let test_url = test_url.unwrap_or("https://www.google.com/generate_204");
        let url = format!(
            "{}/proxies/{}/delay?url={}&timeout=5000",
            self.base_url,
            urlencoding::encode(proxy_name),
            urlencoding::encode(test_url)
        );
        let resp = self.client.get(url).send().await?;
        let res = resp.json::<DelayResponse>().await?;
        Ok(res.delay)
    }
}
