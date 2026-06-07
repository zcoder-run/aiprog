use crate::webc::error::{Error, Result};
use reqwest::Client as ReqwestClient;
use std::collections::HashMap;

// -- Types

/// Desired response body format.
#[derive(Debug, Clone, Default)]
pub enum BodyFormat {
	#[default]
	Text,
	Json,
	Binary,
}

/// Per-request header value. `Single` sets one value; `Many` sends multiple
/// values for the same header name.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum HeaderValue {
	Single(String),
	Many(Vec<String>),
}

/// Parsed response body.
#[derive(Debug, Clone)]
pub enum Body {
	Text(String),
	Json(serde_json::Value),
	Binary(Vec<u8>),
}

/// Parameters for a web_get request.
pub struct WebGetParams {
	pub url: String,
	/// Per-request User-Agent override. `None` uses the client's default.
	pub user_agent: Option<String>,
	/// Additional headers to merge with the client's defaults.
	pub headers: Option<HashMap<String, HeaderValue>>,
	/// Desired format for the response body. Defaults to `Text`.
	pub body_format: BodyFormat,
}

/// Response from a web_get call.
pub struct WebGetResponse {
	/// The HTTP status code.
	pub status: u16,
	/// `true` when `status` is 2xx.
	pub success: bool,
	/// The final URL after any redirects.
	pub url: String,
	/// Response headers, keys are lower-case.
	pub headers: HashMap<String, String>,
	/// The `Content-Type` header value. Empty string if the header is absent.
	pub content_type: String,
	/// The response body in the format requested via `BodyFormat`.
	pub body: Body,
}

// -- WebClientBuilder

pub struct WebClientBuilder {
	default_user_agent: Option<String>,
	redirect_limit: Option<usize>,
}

impl WebClientBuilder {
	pub fn new() -> Self {
		WebClientBuilder {
			default_user_agent: None,
			redirect_limit: None,
		}
	}

	pub fn with_default_user_agent(mut self, ua: impl Into<String>) -> Self {
		self.default_user_agent = Some(ua.into());
		self
	}

	pub fn with_redirect_limit(mut self, limit: usize) -> Self {
		self.redirect_limit = Some(limit);
		self
	}

	pub fn build(self) -> Result<WebClient> {
		let mut builder = ReqwestClient::builder();

		if let Some(ua) = self.default_user_agent {
			builder = builder.user_agent(ua);
		}

		if let Some(limit) = self.redirect_limit {
			builder = builder.redirect(reqwest::redirect::Policy::limited(limit));
		}

		let inner = builder.build().map_err(|e| Error::build_failed(e.to_string()))?;

		Ok(WebClient { inner })
	}
}

impl Default for WebClientBuilder {
	fn default() -> Self {
		Self::new()
	}
}

// -- WebClient

pub struct WebClient {
	inner: ReqwestClient,
}

impl WebClient {
	pub async fn web_get(&self, params: WebGetParams) -> Result<WebGetResponse> {
		let mut request = self.inner.request(reqwest::Method::GET, &params.url);

		// User-Agent override
		if let Some(ua) = params.user_agent.as_ref() {
			request = request.header("User-Agent", ua.clone());
		}

		// Additional headers
		if let Some(headers) = params.headers.as_ref() {
			for (name, value) in headers {
				match value {
					HeaderValue::Single(v) => {
						request = request.header(name.as_str(), v.clone());
					}
					HeaderValue::Many(vals) => {
						for v in vals {
							request = request.header(name.as_str(), v.clone());
						}
					}
				}
			}
		}

		let response = request.send().await.map_err(|e| Error::request_failed(e.to_string()))?;

		// Collect headers
		let mut resp_headers = HashMap::new();
		for (name, value) in response.headers() {
			let key = name.as_str().to_lowercase();
			let val_str = value.to_str().unwrap_or("").to_string();
			resp_headers
				.entry(key)
				.and_modify(|existing: &mut String| {
					existing.push_str(", ");
					existing.push_str(&val_str);
				})
				.or_insert(val_str);
		}

		let status = response.status().as_u16();
		let url = response.url().to_string();
		let content_type = response
			.headers()
			.get("content-type")
			.and_then(|v| v.to_str().ok())
			.unwrap_or("")
			.to_string();

		let success = (200..300).contains(&status);

		// Determine body format
		let body_format = params.body_format;

		let body = match body_format {
			BodyFormat::Text => {
				let text = response.text().await.map_err(|e| Error::request_failed(e.to_string()))?;
				Body::Text(text)
			}
			BodyFormat::Json => {
				let value = response
					.json::<serde_json::Value>()
					.await
					.map_err(|e| Error::body_parse_failed(format!("JSON parse error: {e}")))?;
				Body::Json(value)
			}
			BodyFormat::Binary => {
				let bytes = response.bytes().await.map_err(|e| Error::request_failed(e.to_string()))?;
				Body::Binary(bytes.to_vec())
			}
		};

		Ok(WebGetResponse {
			status,
			success,
			url,
			headers: resp_headers,
			content_type,
			body,
		})
	}
}
