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

use crate::ScriptResult;
use crate::registry::AipRegistry;
use crate::script::AipApiResult;
use crate::script::LuaJsonExt;
use crate::script::script_error;
use crate::script::{AipApiError, AipFromLua, AipIntoLua, LuaExt, ScriptEngine};
use crate::webc;
use mlua::Lua;
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
	fn from_lua(_lua: &Lua, value: mlua::Value) -> ScriptResult<Self> {
		let table = value.as_table().ok_or("Expected table")?;
		let data: String = table.get("data")?;

		let user_agent: Option<AipWebUserAgent> = table.get::<mlua::Value>("user_agent").ok().and_then(|v| match v {
			mlua::Value::Boolean(b) => Some(AipWebUserAgent::Bool(b)),
			mlua::Value::String(s) => Some(AipWebUserAgent::String(s.to_string_lossy().to_string())),
			_ => None,
		});

		let headers: Option<HashMap<String, AipWebHeaderValue>> =
			table.get::<mlua::Value>("headers").ok().and_then(|v| {
				if v.x_is_null() {
					return None;
				}
				let t = v.as_table()?;
				let mut map = HashMap::new();
				for pair in t.pairs::<String, mlua::Value>() {
					let (key, val) = pair.ok()?;
					let header_val = match val {
						mlua::Value::String(s) => AipWebHeaderValue::Single(s.to_string_lossy().to_string()),
						mlua::Value::Table(arr) => {
							let mut vec = Vec::new();
							for i in 1..=arr.len().unwrap_or(0) {
								if let Ok(s) = arr.get::<String>(i) {
									vec.push(s);
								}
							}
							if vec.is_empty() {
								return None;
							}
							AipWebHeaderValue::Many(vec)
						}
						_ => return None,
					};
					map.insert(key, header_val);
				}
				Some(map)
			});

		let redirect_limit: Option<usize> = table
			.get::<mlua::Value>("redirect_limit")
			.ok()
			.and_then(|v| v.as_integer().map(|n| n as usize));

		let parse: Option<bool> = table.get::<mlua::Value>("parse").ok().and_then(|v| v.as_boolean());

		Ok(AipWebGetParams {
			data,
			user_agent,
			headers,
			redirect_limit,
			parse,
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

async fn aip_web_get_handler(params: AipWebGetParams) -> AipApiResult<AipWebGetResult> {
	let client = build_webc_client(&params)?;
	let web_params = build_web_get_params(&params);
	let url_clone = params.data.clone();

	let response = client
		.web_get(web_params)
		.await
		.map_err(|err| webc_error_to_aip(&url_clone, err))?;

	let status = response.status;
	let url = response.url.clone();
	let headers = response.headers;
	let content_type = if response.content_type.is_empty() {
		None
	} else {
		Some(response.content_type.clone())
	};

	let should_parse = params.parse.unwrap_or(false) && content_type.as_deref().is_some_and(is_json_content_type);

	let body_text = match response.body {
		webc::Body::Text(text) => text,
		webc::Body::Json(value) => {
			let success = response.success;
			let error = if success {
				None
			} else {
				Some(format!("HTTP request failed with status {}", status))
			};
			return Ok(AipWebGetResult {
				data: value,
				success,
				status,
				url,
				content_type,
				headers,
				error,
			});
		}
		webc::Body::Binary(_) => {
			return Err(webc_error_to_aip(
				&url_clone,
				webc::Error::custom("Unexpected binary body for text request"),
			));
		}
	};

	let data = if should_parse {
		serde_json::from_str(&body_text).map_err(|err| {
			aip_web_error(
				"PARSE_FAILED",
				format!("aip.web.get failed to parse JSON response for url: {}", params.data),
				None,
				Some(err.to_string()),
			)
		})?
	} else {
		serde_json::Value::String(body_text)
	};

	let success = response.success;
	let error = if success {
		None
	} else {
		Some(format!("HTTP request failed with status {}", status))
	};

	Ok(AipWebGetResult {
		data,
		success,
		status,
		url,
		content_type,
		headers,
		error,
	})
}

// endregion: --- aip.web.get

// region:    --- AipWebRResult

impl AipIntoLua for AipWebGetResult {
	fn into_lua(self, lua: &Lua) -> script_error::ScriptResult<mlua::Value> {
		let table = lua.create_table()?;

		let data_lua = mlua::Value::x_from_json_value(lua, self.data)?;
		table.set("data", data_lua)?;
		table.set("success", self.success)?;
		table.set("status", self.status)?;
		table.set("url", self.url.as_str())?;
		if let Some(content_type) = self.content_type {
			table.set("content_type", content_type.as_str())?;
		}
		if let Some(error) = self.error {
			table.set("error", error.as_str())?;
		}
		let headers_table = lua.create_table()?;
		for (key, value) in self.headers.iter() {
			headers_table.set(key.as_str(), value.as_str())?;
		}
		table.set("headers", headers_table)?;
		Ok(mlua::Value::Table(table))
	}
}

// endregion: --- AipWebRResult

// region:    --- Support

fn build_webc_client(params: &AipWebGetParams) -> AipApiResult<webc::WebClient> {
	let mut builder = webc::WebClientBuilder::new();

	if let Some(limit) = params.redirect_limit {
		builder = builder.with_redirect_limit(limit);
	}

	let has_user_agent_header = params.headers.as_ref().is_some_and(has_user_agent_header);

	match &params.user_agent {
		Some(AipWebUserAgent::Bool(true)) => {
			builder = builder.with_default_user_agent(DEFAULT_UA_AIPROG);
		}
		Some(AipWebUserAgent::Bool(false)) => {
			// no default UA
		}
		Some(AipWebUserAgent::String(ua)) => {
			builder = builder.with_default_user_agent(ua.as_str());
		}
		None if !has_user_agent_header => {
			builder = builder.with_default_user_agent(DEFAULT_UA_AIPROG);
		}
		None => {
			// no default UA
		}
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

fn build_web_get_params(params: &AipWebGetParams) -> webc::WebGetParams {
	let headers = params.headers.as_ref().map(|h| {
		h.iter()
			.map(|(name, value)| {
				let header_value = match value {
					AipWebHeaderValue::Single(v) => webc::HeaderValue::Single(v.clone()),
					AipWebHeaderValue::Many(vals) => webc::HeaderValue::Many(vals.clone()),
				};
				(name.clone(), header_value)
			})
			.collect::<HashMap<String, webc::HeaderValue>>()
	});

	webc::WebGetParams {
		url: params.data.clone(),
		user_agent: None,
		headers,
		body_format: webc::BodyFormat::Text,
	}
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

fn webc_error_to_aip(url: &str, err: webc::Error) -> AipApiError {
	match err {
		webc::Error::BuildFailed(e) => aip_web_error(
			"CLIENT_BUILD_FAILED",
			format!("aip.web.get failed for url: {url}"),
			None,
			Some(e),
		),
		webc::Error::RequestFailed(e) => aip_web_error(
			"REQUEST_FAILED",
			format!("aip.web.get failed for url: {url}"),
			None,
			Some(e),
		),
		webc::Error::BodyParseFailed(e) => aip_web_error(
			"PARSE_FAILED",
			format!("aip.web.get failed to parse response for url: {url}"),
			None,
			Some(e),
		),
		webc::Error::Custom(e) => aip_web_error(
			"REQUEST_FAILED",
			format!("aip.web.get failed for url: {url}"),
			None,
			Some(e),
		),
	}
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
#[path = "aip_web_tests.rs"]
mod tests;

// endregion: --- Tests
