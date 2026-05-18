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
        let resp = self.http.get(&url).bearer_auth(&self.token).send().await?;
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

    pub async fn list_projects(&self) -> Result<Vec<RemoteProject>> {
        let org = self.org()?;
        let url = format!("{}/api/v1/organizations/{org}/projects", self.base_url);
        self.get_list(&url).await
    }

    pub async fn list_tasks(&self) -> Result<Vec<RemoteTask>> {
        let org = self.org()?;
        let url = format!("{}/api/v1/organizations/{org}/tasks", self.base_url);
        self.get_list(&url).await
    }

    pub async fn list_tags(&self) -> Result<Vec<RemoteTag>> {
        let org = self.org()?;
        let url = format!("{}/api/v1/organizations/{org}/tags", self.base_url);
        self.get_list(&url).await
    }

    async fn get_list<T: for<'de> serde::Deserialize<'de>>(&self, url: &str) -> Result<Vec<T>> {
        let resp = self.http.get(url).bearer_auth(&self.token).send().await?;
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
        let wrapper: Wrapper<Vec<T>> = resp.json().await?;
        Ok(wrapper.data)
    }

    pub async fn create_time_entry(&self, req: &CreateEntryRequest<'_>) -> Result<RemoteTimeEntry> {
        let org = self.org()?;
        let url = format!("{}/api/v1/organizations/{org}/time-entries", self.base_url);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.token)
            .json(req)
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
        let wrapper: Wrapper<RemoteTimeEntry> = resp.json().await?;
        Ok(wrapper.data)
    }

    pub async fn update_time_entry(
        &self,
        id: &str,
        req: &CreateEntryRequest<'_>,
    ) -> Result<RemoteTimeEntry> {
        let org = self.org()?;
        let url = format!(
            "{}/api/v1/organizations/{org}/time-entries/{id}",
            self.base_url
        );
        let resp = self
            .http
            .put(&url)
            .bearer_auth(&self.token)
            .json(req)
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
        let wrapper: Wrapper<RemoteTimeEntry> = resp.json().await?;
        Ok(wrapper.data)
    }

    pub async fn delete_time_entry(&self, id: &str) -> Result<()> {
        let org = self.org()?;
        let url = format!(
            "{}/api/v1/organizations/{org}/time-entries/{id}",
            self.base_url
        );
        let resp = self
            .http
            .delete(&url)
            .bearer_auth(&self.token)
            .send()
            .await?;
        let status = resp.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(Error::SolidtimeAuth);
        }
        if !status.is_success() && status != StatusCode::NO_CONTENT {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Solidtime {
                status: status.as_u16(),
                body,
            });
        }
        Ok(())
    }
}
