pub mod auth;
pub mod dto;

use crate::solidtime::auth::{ApiTokenProvider, TokenProvider};
use crate::{Error, Result};
use dto::*;
use reqwest::{Client, RequestBuilder, StatusCode};
use std::sync::Arc;

pub struct SolidtimeClient {
    base_url: String,
    tokens: Arc<dyn TokenProvider>,
    http: Client,
    org_id: Option<String>,
}

impl SolidtimeClient {
    pub fn new(base_url: &str, tokens: Arc<dyn TokenProvider>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            tokens,
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("reqwest client builds"),
            org_id: None,
        }
    }

    /// Convenience constructor for the common case of a static API token.
    /// Keeps call sites short and preserves the old behaviour.
    pub fn with_api_token(base_url: &str, token: &str) -> Self {
        Self::new(base_url, Arc::new(ApiTokenProvider::new(token.to_string())))
    }

    pub fn with_org(mut self, org_id: impl Into<String>) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    pub(crate) fn org(&self) -> Result<&str> {
        self.org_id
            .as_deref()
            .ok_or(Error::MissingConfig("solidtime.org_id"))
    }

    async fn authed(&self, builder: RequestBuilder) -> Result<RequestBuilder> {
        let token = self.tokens.access_token().await?;
        Ok(builder.bearer_auth(token))
    }

    pub async fn test_connection(&self) -> Result<UserMe> {
        let url = format!("{}/api/v1/users/me", self.base_url);
        let resp = self.authed(self.http.get(&url)).await?.send().await?;
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

    pub async fn list_projects(&self) -> Result<Vec<RemoteProject>> {
        let org = self.org()?;
        let url = format!("{}/api/v1/organizations/{org}/projects", self.base_url);
        self.get_list(&url).await
    }

    /// Returns the raw response body for `GET /projects`. Diagnostic only —
    /// callers should prefer `list_projects()` for parsed data.
    pub async fn list_projects_raw(&self) -> Result<String> {
        let org = self.org()?;
        let url = format!("{}/api/v1/organizations/{org}/projects", self.base_url);
        let resp = self.authed(self.http.get(&url)).await?.send().await?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if status == StatusCode::UNAUTHORIZED {
            return Err(Error::SolidtimeAuth);
        }
        if !status.is_success() {
            return Err(Error::Solidtime {
                status: status.as_u16(),
                body,
            });
        }
        Ok(body)
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

    pub async fn list_clients(&self) -> Result<Vec<RemoteClient>> {
        let org = self.org()?;
        let url = format!("{}/api/v1/organizations/{org}/clients", self.base_url);
        self.get_list(&url).await
    }

    pub async fn list_memberships(&self) -> Result<Vec<Membership>> {
        let url = format!("{}/api/v1/users/me/memberships", self.base_url);
        self.get_list(&url).await
    }

    async fn get_list<T: for<'de> serde::Deserialize<'de>>(&self, url: &str) -> Result<Vec<T>> {
        let resp = self.authed(self.http.get(url)).await?.send().await?;
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
            .authed(self.http.post(&url))
            .await?
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
            .authed(self.http.put(&url))
            .await?
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
        let resp = self.authed(self.http.delete(&url)).await?.send().await?;
        let status = resp.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(Error::SolidtimeAuth);
        }
        // 404 is a success: the entry is gone either way. Treating it as
        // an error here would cause the queue to retry forever when a user
        // deletes an entry directly in Solidtime web after stint had
        // queued a delete.
        if !status.is_success()
            && status != StatusCode::NO_CONTENT
            && status != StatusCode::NOT_FOUND
        {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Solidtime {
                status: status.as_u16(),
                body,
            });
        }
        Ok(())
    }

    pub async fn list_time_entries(
        &self,
        member_id: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<RemoteTimeEntry>> {
        let org = self.org()?;
        let url = format!("{}/api/v1/organizations/{org}/time-entries", self.base_url);
        let resp = self
            .authed(self.http.get(&url))
            .await?
            .query(&[("member_ids[]", member_id), ("start", from), ("end", to)])
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
        let wrapper: Wrapper<Vec<RemoteTimeEntry>> = resp.json().await?;
        Ok(wrapper.data)
    }

    /// Currently-running entries for the given member (Solidtime's
    /// `?active=true` filter — entries with `end IS NULL`). Used by the
    /// overlap-adoption diagnostic to surface a stale remote timer that
    /// our small `start`-window query can't see (Solidtime filters on the
    /// `start` column strictly, so a still-running entry started hours ago
    /// won't show up in a 3-second window).
    pub async fn list_active_time_entries(&self, member_id: &str) -> Result<Vec<RemoteTimeEntry>> {
        let org = self.org()?;
        let url = format!("{}/api/v1/organizations/{org}/time-entries", self.base_url);
        let resp = self
            .authed(self.http.get(&url))
            .await?
            .query(&[("member_ids[]", member_id), ("active", "true")])
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
        let wrapper: Wrapper<Vec<RemoteTimeEntry>> = resp.json().await?;
        Ok(wrapper.data)
    }

    pub async fn get_time_entry(&self, id: &str) -> Result<Option<RemoteTimeEntry>> {
        let org = self.org()?;
        let url = format!(
            "{}/api/v1/organizations/{org}/time-entries/{id}",
            self.base_url
        );
        let resp = self.authed(self.http.get(&url)).await?.send().await?;
        let status = resp.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
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
        Ok(Some(wrapper.data))
    }
}
