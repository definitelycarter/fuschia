use async_trait::async_trait;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
  #[error("host '{host}' is not in allowed_hosts")]
  HostNotAllowed { host: String },
  #[error("request failed: {0}")]
  RequestFailed(String),
  #[error("invalid url: {0}")]
  InvalidUrl(String),
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
  pub method: String,
  pub url: String,
  pub headers: HashMap<String, String>,
  pub body: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
  pub status: u16,
  pub headers: HashMap<String, String>,
  pub body: String,
}

#[async_trait]
pub trait HttpClient: Send + Sync {
  async fn send(&self, req: HttpRequest) -> Result<HttpResponse, HttpError>;
}

/// Exact + wildcard-prefix allowed hosts policy.
///
/// Patterns:
/// - `*` — matches every host
/// - `*.example.com` — matches `example.com` and any subdomain
/// - `api.example.com` — exact match
#[derive(Debug, Clone, Default)]
pub struct AllowedHosts {
  patterns: Vec<String>,
}

impl AllowedHosts {
  pub fn new(patterns: impl IntoIterator<Item = impl Into<String>>) -> Self {
    Self {
      patterns: patterns.into_iter().map(Into::into).collect(),
    }
  }

  /// Allow every host. Useful for tests; not recommended in production.
  pub fn all() -> Self {
    Self {
      patterns: vec!["*".into()],
    }
  }

  pub fn is_allowed(&self, host: &str) -> bool {
    self.patterns.iter().any(|pattern| {
      if pattern == "*" {
        true
      } else if let Some(suffix) = pattern.strip_prefix("*.") {
        host == suffix || host.ends_with(&format!(".{suffix}"))
      } else {
        host == pattern
      }
    })
  }
}

/// `reqwest`-backed HTTP client with allowed-hosts enforcement.
pub struct ReqwestHttp {
  allowed: AllowedHosts,
  client: reqwest::Client,
}

impl ReqwestHttp {
  pub fn new(allowed: AllowedHosts) -> Self {
    Self {
      allowed,
      client: reqwest::Client::new(),
    }
  }

  pub fn with_client(allowed: AllowedHosts, client: reqwest::Client) -> Self {
    Self { allowed, client }
  }
}

#[async_trait]
impl HttpClient for ReqwestHttp {
  async fn send(&self, req: HttpRequest) -> Result<HttpResponse, HttpError> {
    let url: reqwest::Url = req
      .url
      .parse()
      .map_err(|e: url::ParseError| HttpError::InvalidUrl(e.to_string()))?;

    let host = url
      .host_str()
      .ok_or_else(|| HttpError::InvalidUrl("missing host".into()))?;

    if !self.allowed.is_allowed(host) {
      return Err(HttpError::HostNotAllowed {
        host: host.to_string(),
      });
    }

    let method: reqwest::Method = req
      .method
      .parse()
      .map_err(|_| HttpError::RequestFailed(format!("invalid method: {}", req.method)))?;

    let mut builder = self.client.request(method, url);
    for (k, v) in &req.headers {
      builder = builder.header(k, v);
    }
    if let Some(body) = req.body {
      builder = builder.body(body);
    }

    let response = builder
      .send()
      .await
      .map_err(|e| HttpError::RequestFailed(e.to_string()))?;

    let status = response.status().as_u16();
    let headers = response
      .headers()
      .iter()
      .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
      .collect();
    let body = response
      .text()
      .await
      .map_err(|e| HttpError::RequestFailed(e.to_string()))?;

    Ok(HttpResponse {
      status,
      headers,
      body,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn exact_host_match() {
    let allowed = AllowedHosts::new(["api.example.com"]);
    assert!(allowed.is_allowed("api.example.com"));
    assert!(!allowed.is_allowed("evil.com"));
  }

  #[test]
  fn wildcard_subdomain() {
    let allowed = AllowedHosts::new(["*.googleapis.com"]);
    assert!(allowed.is_allowed("storage.googleapis.com"));
    assert!(allowed.is_allowed("googleapis.com"));
    assert!(!allowed.is_allowed("evil.com"));
  }

  #[test]
  fn allow_all() {
    let allowed = AllowedHosts::all();
    assert!(allowed.is_allowed("anything.com"));
  }

  #[test]
  fn empty_denies_everything() {
    let allowed = AllowedHosts::default();
    assert!(!allowed.is_allowed("anything.com"));
  }
}
