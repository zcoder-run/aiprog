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

/// Request body to send in a POST (or other) request.
#[derive(Debug, Clone)]
pub enum RequestBody {
	Json(serde_json::Value),
	Text(String),
}

/// Parameters for a web request.
pub struct WebParams {
	pub url: String,
	/// Per-request User-Agent override. `None` uses the client's default.
	pub user_agent: Option<String>,
	/// Additional headers to merge with the client's defaults.
	/// Expected TypeScript shape: `{[name:string]: string | string[]}`.
	pub headers: Option<HashMap<String, HeaderValue>>,
	/// Optional query parameters appended to the request URL.
	/// Expected TypeScript shape: `{[name:string]: string | string[]}`.
	pub query_params: Option<HashMap<String, HeaderValue>>,
	/// Desired format for the response body. Defaults to `Text`.
	pub body_format: BodyFormat,
}

/// Parameters for a POST web request.
pub struct WebPostParams {
	pub url: String,
	/// Per-request User-Agent override. `None` uses the client's default.
	pub user_agent: Option<String>,
	/// Additional headers to merge with the client's defaults.
	/// Expected TypeScript shape: `{[name:string]: string | string[]}`.
	pub headers: Option<HashMap<String, HeaderValue>>,
	/// Optional query parameters appended to the request URL.
	/// Expected TypeScript shape: `{[name:string]: string | string[]}`.
	pub query_params: Option<HashMap<String, HeaderValue>>,
	/// Request body. `None` means no body is sent.
	pub body: Option<RequestBody>,
	/// Desired format for the response body. Defaults to `Text`.
	pub body_format: BodyFormat,
}

/// Response from a web call.
pub struct WebResponse {
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
	pub async fn web_get(&self, params: WebParams) -> Result<WebResponse> {
		let request_url = append_query_params(&params.url, params.query_params.as_ref());
		let mut request = self.inner.request(reqwest::Method::GET, &request_url);

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

		Ok(WebResponse {
			status,
			success,
			url,
			headers: resp_headers,
			content_type,
			body,
		})
	}

	pub async fn web_post(&self, params: WebPostParams) -> Result<WebResponse> {
		let request_url = append_query_params(&params.url, params.query_params.as_ref());
		let mut request = self.inner.request(reqwest::Method::POST, &request_url);

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

		// Set body
		if let Some(body) = &params.body {
			match body {
				RequestBody::Json(val) => {
					request = request.json(val);
				}
				RequestBody::Text(text) => {
					request = request.body(text.clone());
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

		Ok(WebResponse {
			status,
			success,
			url,
			headers: resp_headers,
			content_type,
			body,
		})
	}
}

// region:    --- Support

fn append_query_params(url: &str, query_params: Option<&HashMap<String, HeaderValue>>) -> String {
	let Some(query_params) = query_params else {
		return url.to_string();
	};

	let mut query = url::form_urlencoded::Serializer::new(String::new());
	for (name, value) in query_params {
		match value {
			HeaderValue::Single(value) => {
				query.append_pair(name, value);
			}
			HeaderValue::Many(values) => {
				for value in values {
					query.append_pair(name, value);
				}
			}
		}
	}
	let encoded_query = query.finish();
	if encoded_query.is_empty() {
		return url.to_string();
	}

	let (base_url, fragment) = match url.split_once('#') {
		Some((base_url, fragment)) => (base_url, Some(fragment)),
		None => (url, None),
	};
	let separator = if base_url.contains('?') { '&' } else { '?' };

	let capacity = url.len().saturating_add(1).saturating_add(encoded_query.len());
	let mut result = String::with_capacity(capacity);
	result.push_str(base_url);
	result.push(separator);
	result.push_str(&encoded_query);
	if let Some(fragment) = fragment {
		result.push('#');
		result.push_str(fragment);
	}
	result
}

// endregion: --- Support
