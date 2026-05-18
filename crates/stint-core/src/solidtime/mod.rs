pub mod dto;

use crate::{Error, Result};
use dto::*;
use reqwest::{Client, StatusCode};

pub struct SolidtimeClient {
    base_url: String,
    token: String,
    http: Client,
    org_id: Option<String>,
}

impl SolidtimeClient {
    pub fn new(base_url: &str, token: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.to_string(),
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("reqwest client builds"),
            org_id: None,
        }
    }

    pub fn with_org(mut self, org_id: impl Into<String>) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    pub async fn test_connection(&self) -> Result<UserMe> {
        let url = format!("{}/api/v1/users/me", self.base_url);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await?;
        let status = resp.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(Error::SolidtimeAuth);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Solidtime {
                status: status.as_u16(),
                body,
            });
        }
        let wrapper: Wrapper<UserMe> = resp.json().await?;
        Ok(wrapper.data)
    }

    pub(crate) fn org(&self) -> Result<&str> {
        self.org_id
            .as_deref()
            .ok_or(Error::MissingConfig("solidtime.org_id"))
    }
}
