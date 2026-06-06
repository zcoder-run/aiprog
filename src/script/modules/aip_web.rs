//! Defines the `web` module, used in the lua engine.
//!
//! ---
//!
//! ## Lua documentation
//!
//! The `aip.web` module exposes functions to make HTTP requests.
//!
//! ### Functions
//!
//! - `aip.web.get(params: { data: string, user_agent?: string | boolean, headers?: table, redirect_limit?: number, parse?: boolean }) -> { data: string | table, success: boolean, status: number, url: string, content_type?: string, headers: table, error?: string }`
//!
//! ### Constants
//!
//! - `aip.web.UA_BROWSER: string`: Default browser User Agent string.
//! - `aip.web.UA_AIPROG: string`: Default aipROG User Agent string (`aipROG`).
//!
//! ---
//!

use crate::Result;
use crate::registry::AipRegistry;
use crate::script::{AipApiError, AipFromLua, AipToLua, ScriptEngine};
use mlua::Lua;
use reqwest::header::CONTENT_TYPE;
use reqwest::{Client, RequestBuilder};
use std::collections::HashMap;

const DEFAULT_UA_AIPROG: &str = "aiprog";
const DEFAULT_UA_BROWSER: &str =
	"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36";

pub fn register(registry: &mut AipRegistry) -> crate::Result<()> {
	registry.register_async::<_, _, _, _>("aip.web.get", aip_web_get_handler)?;
	Ok(())
}

/// Install the web module constants (`UA_AIPROG`, `UA_BROWSER`) into Lua.
///
/// This must be called **after** the handler has been registered and the engine
/// has populated the `aip.web` table.
pub fn install_constants(engine: &ScriptEngine) -> mlua::Result<()> {
	let lua = engine.lua();
	engine.set_value_at_path(
		"aip.web.UA_AIPROG",
		mlua::Value::String(lua.create_string(DEFAULT_UA_AIPROG)?),
	)?;
	engine.set_value_at_path(
		"aip.web.UA_BROWSER",
		mlua::Value::String(lua.create_string(DEFAULT_UA_BROWSER)?),
	)?;
	Ok(())
}

// region:    --- aip.web.get

/// Parameters for the `get` function.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct AipWebGetParams {
	/// The URL to request.
	pub data: String,

	/// User-Agent behavior. `true` uses `aipROG`, `false` disables the default, and a string is used as-is.
	pub user_agent: Option<AipWebUserAgent>,

	/// Request headers.
	pub headers: Option<HashMap<String, AipWebHeaderValue>>,

	/// Number of redirects to follow.
	pub redirect_limit: Option<usize>,

	/// If true, attempts to parse JSON response data when the Content-Type is JSON.
	pub parse: Option<bool>,
}

impl AipFromLua for AipWebGetParams {
	fn from_lua(_lua: &Lua, value: mlua::Value) -> std::result::Result<Self, crate::script::HandlerError> {
		let table = value
			.as_table()
			.ok_or_else(|| crate::script::HandlerError::new("Expected table".to_string()))?;
		let data: String = table.get("data").map_err(|e| crate::script::HandlerError::new(e.to_string()))?;
		Ok(AipWebGetParams {
			data,
			user_agent: None,
			headers: None,
			redirect_limit: None,
			parse: None,
		})
	}
}

/// User-Agent option for the `get` function.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum AipWebUserAgent {
	Bool(bool),
	String(String),
}

/// Header value option for the `get` function.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum AipWebHeaderValue {
	Single(String),
	Many(Vec<String>),
}

/// Result of the `get` function.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct AipWebGetResult {
	/// The response body as a string, or parsed JSON when `parse` is true and the response is JSON.
	pub data: serde_json::Value,

	/// Indicates if the HTTP status code is successful.
	pub success: bool,

	/// The HTTP status code.
	pub status: u16,

	/// The final response URL.
	pub url: String,

	/// The response Content-Type header, if present.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub content_type: Option<String>,

	/// Response headers keyed by lower-case header name.
	pub headers: HashMap<String, String>,

	/// Status error text for non-success HTTP status codes.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
}

async fn aip_web_get_handler(params: AipWebGetParams) -> core::result::Result<AipWebGetResult, AipApiError> {
	let client = build_client(&params)?;
	let request = client.get(&params.data);
	let request = apply_request_headers(request, params.headers.as_ref());

	let response = request.send().await.map_err(|err| {
		aip_web_error(
			"REQUEST_FAILED",
			format!("aip.web.get failed for url: {}", params.data),
			None,
			Some(err.to_string()),
		)
	})?;

	let status = response.status();
	let url = response.url().as_str().to_string();
	let headers = collect_response_headers(response.headers());
	let content_type = response
		.headers()
		.get(CONTENT_TYPE)
		.and_then(|value| value.to_str().ok())
		.map(ToOwned::to_owned);
	let should_parse = params.parse.unwrap_or(false) && content_type.as_deref().is_some_and(is_json_content_type);

	let body = response.text().await.map_err(|err| {
		aip_web_error(
			"READ_BODY_FAILED",
			format!("aip.web.get failed to read response body for url: {}", params.data),
			None,
			Some(err.to_string()),
		)
	})?;

	let data = if should_parse {
		serde_json::from_str(&body).map_err(|err| {
			aip_web_error(
				"PARSE_FAILED",
				format!("aip.web.get failed to parse JSON response for url: {}", params.data),
				None,
				Some(err.to_string()),
			)
		})?
	} else {
		serde_json::Value::String(body)
	};

	let success = status.is_success();
	let error = if success {
		None
	} else {
		Some(format!("HTTP request failed with status {}", status.as_u16()))
	};

	Ok(AipWebGetResult {
		data,
		success,
		status: status.as_u16(),
		url,
		content_type,
		headers,
		error,
	})
}

// endregion: --- aip.web.get

// region:    --- Support

fn build_client(params: &AipWebGetParams) -> core::result::Result<Client, AipApiError> {
	let mut builder = Client::builder();

	if let Some(redirect_limit) = params.redirect_limit {
		builder = builder.redirect(reqwest::redirect::Policy::limited(redirect_limit));
	}

	let has_user_agent_header = params.headers.as_ref().is_some_and(has_user_agent_header);

	match &params.user_agent {
		Some(AipWebUserAgent::Bool(true)) => {
			builder = builder.user_agent(DEFAULT_UA_AIPROG);
		}
		Some(AipWebUserAgent::Bool(false)) => {}
		Some(AipWebUserAgent::String(user_agent)) => {
			builder = builder.user_agent(user_agent);
		}
		None if !has_user_agent_header => {
			builder = builder.user_agent(DEFAULT_UA_AIPROG);
		}
		None => {}
	}

	builder.build().map_err(|err| {
		aip_web_error(
			"CLIENT_BUILD_FAILED",
			"aip.web.get failed to build HTTP client",
			None,
			Some(err.to_string()),
		)
	})
}

fn apply_request_headers(
	mut request: RequestBuilder,
	headers: Option<&HashMap<String, AipWebHeaderValue>>,
) -> RequestBuilder {
	let Some(headers) = headers else {
		return request;
	};

	for (name, value) in headers {
		match value {
			AipWebHeaderValue::Single(value) => {
				request = request.header(name.as_str(), value.as_str());
			}
			AipWebHeaderValue::Many(values) => {
				for value in values {
					request = request.header(name.as_str(), value.as_str());
				}
			}
		}
	}

	request
}

fn collect_response_headers(header_map: &reqwest::header::HeaderMap) -> HashMap<String, String> {
	let mut headers = HashMap::<String, String>::new();

	for (name, value) in header_map {
		let key = name.as_str().to_ascii_lowercase();
		let value = match value.to_str() {
			Ok(value) => value.to_string(),
			Err(_) => String::from_utf8_lossy(value.as_bytes()).into_owned(),
		};

		if let Some(existing) = headers.get_mut(&key) {
			existing.push_str(", ");
			existing.push_str(&value);
		} else {
			headers.insert(key, value);
		}
	}

	headers
}

fn has_user_agent_header(headers: &HashMap<String, AipWebHeaderValue>) -> bool {
	headers.keys().any(|key| key.eq_ignore_ascii_case("user-agent"))
}

fn is_json_content_type(content_type: &str) -> bool {
	let Some(mime) = content_type.split(';').next() else {
		return false;
	};

	let mime = mime.trim().to_ascii_lowercase();
	mime == "application/json" || mime.ends_with("+json")
}

fn aip_web_error(
	code: impl Into<String>,
	message: impl Into<String>,
	details: Option<String>,
	cause: Option<String>,
) -> AipApiError {
	AipApiError {
		code: code.into(),
		message: message.into(),
		details,
		cause,
	}
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
#[path = "aip_web_tests.rs"]
mod tests;

// endregion: --- Tests

impl AipToLua for AipWebGetResult {
	fn to_lua(self, lua: &Lua) -> std::result::Result<mlua::Value, crate::script::HandlerError> {
		let table = lua
			.create_table()
			.map_err(|e| crate::script::HandlerError::new(e.to_string()))?;

		let data_lua = crate::script::serde_value_to_lua_value(lua, self.data)
			.map_err(|e| crate::script::HandlerError::new(e.to_string()))?;
		table
			.set("data", data_lua)
			.map_err(|e| crate::script::HandlerError::new(e.to_string()))?;
		table
			.set("success", self.success)
			.map_err(|e| crate::script::HandlerError::new(e.to_string()))?;
		table
			.set("status", self.status)
			.map_err(|e| crate::script::HandlerError::new(e.to_string()))?;
		table
			.set("url", self.url.as_str())
			.map_err(|e| crate::script::HandlerError::new(e.to_string()))?;
		if let Some(content_type) = self.content_type {
			table
				.set("content_type", content_type.as_str())
				.map_err(|e| crate::script::HandlerError::new(e.to_string()))?;
		}
		if let Some(error) = self.error {
			table
				.set("error", error.as_str())
				.map_err(|e| crate::script::HandlerError::new(e.to_string()))?;
		}
		let headers_table = lua
			.create_table()
			.map_err(|e| crate::script::HandlerError::new(e.to_string()))?;
		for (key, value) in self.headers.iter() {
			headers_table
				.set(key.as_str(), value.as_str())
				.map_err(|e| crate::script::HandlerError::new(e.to_string()))?;
		}
		table
			.set("headers", headers_table)
			.map_err(|e| crate::script::HandlerError::new(e.to_string()))?;
		Ok(mlua::Value::Table(table))
	}
}
