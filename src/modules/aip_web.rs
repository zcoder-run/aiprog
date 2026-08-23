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
//! - `aip.web.get(params: AipWebGetParams) -> { data: string | table, success: boolean, status: number, url: string, content_type?: string, headers: table, error?: string }`
//! - `aip.web.post(params: AipWebPostParams) -> { data: string | table, success: boolean, status: number, url: string, content_type?: string, headers: table, error?: string }`
//!
//! ### Constants
//!
//! - `aip.web.UA_BROWSER: string`: Default browser User Agent string.
//! - `aip.web.UA_AIPROG: string`: Default AIProg User Agent string (`AIProg`).
//!
//! ---
//!

use crate::{AipModule, LuaJsonExt, NativeFunctionSet};
use crate::base::webc;
use crate::registry::{HandlerError, HandlerResult};
use crate::{AipFromLua, AipIntoLua, HandlerCallContext, LuaExt};
use crate::AipRegistryBuilder;
use mlua::Lua;
use std::collections::HashMap;

const DEFAULT_UA_AIPROG: &str = "aiprog";
#[allow(dead_code)]
const DEFAULT_UA_BROWSER: &str =
	"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36";

#[derive(Debug, Clone, Copy, Default)]
pub struct WebModule;

impl AipModule for WebModule {
	fn register(builder: AipRegistryBuilder) -> crate::Result<AipRegistryBuilder> {
		register(builder)
	}
}

impl WebModule {
	#[allow(dead_code)]
	pub fn native_functions(&self) -> NativeFunctionSet {
		NativeFunctionSet::default().append_installer(native_function_installer())
	}
}

pub fn register(registry: AipRegistryBuilder) -> crate::Result<AipRegistryBuilder> {
	let registry = registry
		.register_async("aip.web.get", aip_web_get_handler)?
		.register_async("aip.web.post", aip_web_post_handler)?;
	Ok(registry)
}

// region:    --- aip.web.get

/// Parameters for `aip.web.get`.
///
/// The `query_params` property is a table of string values or arrays of string values:
/// `{[name:string]: string | string[]}`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipWebGetParams {
	/// The URL to request.
	pub url: String,

	/// User-Agent behavior. `true` uses `AIProg`, `false` disables the default, and a string is used as-is.
	pub user_agent: Option<AipWebUserAgent>,

	/// Request headers.
	/// Expected TypeScript shape: `{[name:string]: string | string[]}`.
	pub headers: Option<HashMap<String, AipWebHeaderValue>>,

	/// Query parameters appended to the request URL.
	/// Expected TypeScript shape: `{[name:string]: string | string[]}`.
	pub query_params: Option<HashMap<String, AipWebHeaderValue>>,

	/// Number of redirects to follow.
	pub redirect_limit: Option<usize>,

	/// If true, attempts to parse JSON response data when the Content-Type is JSON.
	pub parse: Option<bool>,
}

impl AipFromLua for AipWebGetParams {
	fn from_lua(_lua: &Lua, value: mlua::Value) -> crate::Result<Self> {
		let table = params_table(&value)?;
		let url = required_string(table, "url")?;

		let user_agent = lua_table_to_user_agent(table)?;

		let headers = lua_table_to_headers(table)?;

		let query_params = lua_table_to_query_params(table)?;

		let redirect_limit: Option<usize> = table.x_try_get_i64("redirect_limit")?.map(|n| n as usize);

		let parse: Option<bool> = table.x_try_get_bool("parse")?;

		Ok(AipWebGetParams {
			url,
			user_agent,
			headers,
			query_params,
			redirect_limit,
			parse,
		})
	}
}

impl crate::AipParams for AipWebGetParams {}

/// User-Agent option for the `get` function.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum AipWebUserAgent {
	Bool(bool),
	String(String),
}

/// Header value option for the `get` function.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum AipWebHeaderValue {
	Single(String),
	Many(Vec<String>),
}

/// Output of a web request.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipWebOutput {
	/// The response body as a string, or parsed JSON when `parse` is true and the response is JSON.
	pub data: serde_json::Value,

	/// Indicates if the HTTP status code is successful.
	pub success: bool,

	/// The HTTP status code.
	pub status: u16,

	/// The final response URL.
	pub url: String,

	/// The response Content-Type header, if present.
	pub content_type: Option<String>,

	/// Response headers keyed by lower-case header name.
	pub headers: HashMap<String, String>,

	/// Status error text for non-success HTTP status codes.
	pub error: Option<String>,
}

/// Performs an HTTP GET request and returns the response.
async fn aip_web_get_handler(_call: HandlerCallContext, params: AipWebGetParams) -> HandlerResult<AipWebOutput> {
	let client = build_webc_client(
		params.user_agent.as_ref(),
		params.headers.as_ref(),
		params.redirect_limit,
	)?;
	let web_params = build_web_get_params(&params);
	let url_clone = params.url.clone();

	let response = client
		.web_get(web_params)
		.await
		.map_err(|err| webc_error_to_aip(&url_clone, err))?;

	web_response_to_aip_output(response, params.parse.unwrap_or(false), &params.url)
}

// endregion: --- aip.web.get

// region:    --- aip.web.post

/// Body payload for POST requests.
///
/// Accepts a raw string (sent as-is) or a JSON object / table (JSON-encoded).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum AipWebBody {
	String(String),
	Json(serde_json::Value),
}

/// Parameters for `aip.web.post`.
///
/// The `query_params` property is a table of string values or arrays of string values:
/// `{[name:string]: string | string[]}`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde_with::skip_serializing_none]
pub struct AipWebPostParams {
	/// The URL to request.
	pub url: String,

	/// Request payload as raw string or JSON object/table (`string | object`).
	///
	/// - String: sent as raw text as-is. Inferred `Content-Type` is `text/plain` if `content_type` is not specified.
	/// - Object / Table: JSON-encoded before sending. Inferred `Content-Type` is `application/json` if `content_type` is not specified.
	pub body: Option<AipWebBody>,

	/// Optional Content-Type header override.
	///
	/// When provided, this sets the HTTP `Content-Type` header without changing body serialization.
	pub content_type: Option<String>,

	/// User-Agent behavior. `true` uses `AIProg`, `false` disables the default, and a string is used as-is.
	pub user_agent: Option<AipWebUserAgent>,

	/// Request headers.
	/// Expected TypeScript shape: `{[name:string]: string | string[]}`.
	pub headers: Option<HashMap<String, AipWebHeaderValue>>,

	/// Query parameters appended to the request URL.
	/// Expected TypeScript shape: `{[name:string]: string | string[]}`.
	pub query_params: Option<HashMap<String, AipWebHeaderValue>>,

	/// Number of redirects to follow.
	pub redirect_limit: Option<usize>,

	/// If true, attempts to parse JSON response data when the Content-Type is JSON.
	pub parse: Option<bool>,
}

impl AipFromLua for AipWebPostParams {
	fn from_lua(_lua: &Lua, value: mlua::Value) -> crate::Result<Self> {
		let table = params_table(&value)?;
		let url = required_string(table, "url")?;

		let body = match table.x_try_get_value("body")? {
			Some(v) if !v.x_is_null() => match v {
				mlua::Value::String(s) => Some(AipWebBody::String(s.to_str()?.to_string())),
				mlua::Value::Table(_) => {
					let json_val = v
						.x_to_json_value()
						.map_err(|e| crate::Error::custom(format!("Property 'body' is not a valid JSON value: {e}")))?
						.ok_or_else(|| crate::Error::custom("Property 'body' table cannot be converted to nil"))?;
					Some(AipWebBody::Json(json_val))
				}
				other => {
					return Err(crate::Error::custom(format!(
						"Property 'body' expected to be of type 'string or table', but was of type '{}'",
						other.type_name()
					)));
				}
			},
			_ => None,
		};

		let content_type = table.x_try_get_string("content_type")?;

		let user_agent = lua_table_to_user_agent(table)?;

		let headers = lua_table_to_headers(table)?;

		let query_params = lua_table_to_query_params(table)?;

		let redirect_limit: Option<usize> = table.x_try_get_i64("redirect_limit")?.map(|n| n as usize);

		let parse: Option<bool> = table.x_try_get_bool("parse")?;

		Ok(AipWebPostParams {
			url,
			body,
			content_type,
			user_agent,
			headers,
			query_params,
			redirect_limit,
			parse,
		})
	}
}

impl crate::AipParams for AipWebPostParams {}

/// Performs an HTTP POST request and returns the response.
async fn aip_web_post_handler(_call: HandlerCallContext, params: AipWebPostParams) -> HandlerResult<AipWebOutput> {
	let client = build_webc_client(
		params.user_agent.as_ref(),
		params.headers.as_ref(),
		params.redirect_limit,
	)?;

	let (body, inferred_content_type) = match params.body {
		Some(AipWebBody::Json(json_val)) => {
			let text = serde_json::to_string(&json_val).map_err(|err| {
				aip_web_error(
					"SERIALIZE_FAILED",
					"Failed to serialize JSON body",
					None,
					Some(err.to_string()),
				)
			})?;
			(Some(webc::RequestBody::Text(text)), Some("application/json"))
		}
		Some(AipWebBody::String(text)) => (Some(webc::RequestBody::Text(text)), Some("text/plain")),
		None => (None, None),
	};

	let mut headers = params.headers.unwrap_or_default();

	if let Some(ct) = params.content_type {
		headers.retain(|k, _| !k.eq_ignore_ascii_case("content-type"));
		headers.insert("Content-Type".to_string(), AipWebHeaderValue::Single(ct));
	} else if let Some(inferred_ct) = inferred_content_type {
		let has_ct = headers.keys().any(|k| k.eq_ignore_ascii_case("content-type"));
		if !has_ct {
			headers.insert(
				"Content-Type".to_string(),
				AipWebHeaderValue::Single(inferred_ct.to_string()),
			);
		}
	}

	let post_params = webc::WebPostParams {
		url: params.url.clone(),
		user_agent: None,
		headers: Some(
			headers
				.into_iter()
				.map(|(name, value)| {
					let header_value = match value {
						AipWebHeaderValue::Single(v) => webc::HeaderValue::Single(v),
						AipWebHeaderValue::Many(vals) => webc::HeaderValue::Many(vals),
					};
					(name, header_value)
				})
				.collect(),
		),
		query_params: params.query_params.map(|h| {
			h.into_iter()
				.map(|(name, value)| {
					let query_value = match value {
						AipWebHeaderValue::Single(v) => webc::HeaderValue::Single(v),
						AipWebHeaderValue::Many(vals) => webc::HeaderValue::Many(vals),
					};
					(name, query_value)
				})
				.collect()
		}),
		body,
		body_format: webc::BodyFormat::Text,
	};

	let url_clone = params.url.clone();

	let response = client
		.web_post(post_params)
		.await
		.map_err(|err| webc_error_to_aip(&url_clone, err))?;

	web_response_to_aip_output(response, params.parse.unwrap_or(false), &params.url)
}

// endregion: --- aip.web.post

// region:    --- AipWebOutput

impl AipIntoLua for AipWebOutput {
	fn into_lua(self, lua: &Lua) -> crate::Result<mlua::Value> {
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

impl crate::AipOutput for AipWebOutput {}

// endregion: --- AipWebOutput

// region:    --- Support

fn build_webc_client(
	user_agent: Option<&AipWebUserAgent>,
	headers: Option<&HashMap<String, AipWebHeaderValue>>,
	redirect_limit: Option<usize>,
) -> HandlerResult<webc::WebClient> {
	let mut builder = webc::WebClientBuilder::new();

	if let Some(limit) = redirect_limit {
		builder = builder.with_redirect_limit(limit);
	}

	let has_user_agent_header = headers.map(has_user_agent_header).unwrap_or(false);

	match user_agent {
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
			"Failed to build HTTP client",
			None,
			Some(err.to_string()),
		)
	})
}

fn web_response_to_aip_output(
	response: webc::WebResponse,
	parse: bool,
	error_url: &str,
) -> HandlerResult<AipWebOutput> {
	let status = response.status;
	let url = response.url.clone();
	let headers = response.headers;
	let content_type = if response.content_type.is_empty() {
		None
	} else {
		Some(response.content_type.clone())
	};

	let should_parse = parse && content_type.as_deref().is_some_and(is_json_content_type);

	let body_text = match response.body {
		webc::Body::Text(text) => text,
		webc::Body::Json(value) => {
			let success = response.success;
			let error = if success {
				None
			} else {
				Some(format!("HTTP request failed with status {}", status))
			};
			return Ok(AipWebOutput {
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
				error_url,
				webc::Error::custom("Unexpected binary body for text request"),
			));
		}
	};

	let data = if should_parse {
		serde_json::from_str(&body_text).map_err(|err| {
			aip_web_error(
				"PARSE_FAILED",
				format!("Failed to parse JSON response for url: {error_url}"),
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

	Ok(AipWebOutput {
		data,
		success,
		status,
		url,
		content_type,
		headers,
		error,
	})
}

fn build_web_get_params(params: &AipWebGetParams) -> webc::WebParams {
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

	let query_params = params.query_params.as_ref().map(|h| {
		h.iter()
			.map(|(name, value)| {
				let query_value = match value {
					AipWebHeaderValue::Single(v) => webc::HeaderValue::Single(v.clone()),
					AipWebHeaderValue::Many(vals) => webc::HeaderValue::Many(vals.clone()),
				};
				(name.clone(), query_value)
			})
			.collect::<HashMap<String, webc::HeaderValue>>()
	});

	webc::WebParams {
		url: params.url.clone(),
		user_agent: None,
		headers,
		query_params,
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
) -> HandlerError {
	let code = code.into();
	let message = message.into();
	let mut msg = format!("[{code}] {message}");
	if let Some(d) = &details {
		msg.push_str(&format!("\nDetails: {d}"));
	}
	if let Some(c) = &cause {
		msg.push_str(&format!("\nCause: {c}"));
	}
	HandlerError::custom(msg)
}

fn webc_error_to_aip(url: &str, err: webc::Error) -> HandlerError {
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

/// Extract the params table from a Lua value, failing with the actual type on mismatch.
fn params_table(value: &mlua::Value) -> crate::Result<&mlua::Table> {
	value.as_table().ok_or_else(|| {
		crate::Error::custom(format!(
			"Params expected to be a table, but was of type '{}'",
			value.type_name()
		))
	})
}

/// Get a required string property from a params table, failing loudly on wrong type or absence.
fn required_string(table: &mlua::Table, key: &str) -> crate::Result<String> {
	table
		.x_try_get_string(key)?
		.ok_or_else(|| crate::Error::custom(format!("Missing required property '{key}' of type 'string'")))
}

/// Parse the optional `user_agent` param (boolean or string), failing loudly on wrong type.
fn lua_table_to_user_agent(table: &mlua::Table) -> crate::Result<Option<AipWebUserAgent>> {
	let Some(val) = table.x_try_get_value("user_agent")? else {
		return Ok(None);
	};
	if val.x_is_null() {
		return Ok(None);
	}
	if let Some(b) = val.x_as_bool() {
		Ok(Some(AipWebUserAgent::Bool(b)))
	} else if let Some(s) = val.x_as_lua_str() {
		Ok(Some(AipWebUserAgent::String(s.to_string())))
	} else {
		Err(crate::Error::custom(format!(
			"Property 'user_agent' expected to be of type 'boolean or string', but was of type '{}'",
			val.type_name()
		)))
	}
}

/// Parse the optional `headers` param into a header map, failing loudly with dotted paths on wrong types.
fn lua_table_to_headers(table: &mlua::Table) -> crate::Result<Option<HashMap<String, AipWebHeaderValue>>> {
	lua_table_to_string_values(table, "headers")
}

fn lua_table_to_query_params(table: &mlua::Table) -> crate::Result<Option<HashMap<String, AipWebHeaderValue>>> {
	lua_table_to_string_values(table, "query_params")
}

fn lua_table_to_string_values(
	table: &mlua::Table,
	property_name: &str,
) -> crate::Result<Option<HashMap<String, AipWebHeaderValue>>> {
	let Some(val) = table.x_try_get_value(property_name)? else {
		return Ok(None);
	};
	if val.x_is_null() {
		return Ok(None);
	}
	let t = val.as_table().ok_or_else(|| {
		crate::Error::custom(format!(
			"Property '{property_name}' expected to be of type 'table', but was of type '{}'",
			val.type_name()
		))
	})?;
	let mut map = HashMap::new();
	for pair in t.pairs::<String, mlua::Value>() {
		let (key, entry_val) =
			pair.map_err(|e| crate::Error::cc(format!("Fail to read '{property_name}' entry"), e))?;
		let header_val = match entry_val {
			mlua::Value::String(s) => AipWebHeaderValue::Single(s.to_string_lossy().to_string()),
			mlua::Value::Table(arr) => {
				let mut vec = Vec::new();
				for item in arr.sequence_values::<mlua::Value>() {
					let item =
						item.map_err(|e| crate::Error::cc(format!("Fail to read '{property_name}.{key}' entry"), e))?;
					let s = item.x_as_lua_str().ok_or_else(|| {
						crate::Error::custom(format!(
							"Property '{property_name}.{key}' entries expected to be of type 'string', but got type '{}'",
							item.type_name()
						))
					})?;
					vec.push(s.to_string());
				}
				if vec.is_empty() {
					return Err(crate::Error::custom(format!(
						"Property '{property_name}.{key}' must not be an empty list"
					)));
				}
				AipWebHeaderValue::Many(vec)
			}
			other => {
				return Err(crate::Error::custom(format!(
					"Property '{property_name}.{key}' expected to be of type 'string or string[]', but was of type '{}'",
					other.type_name()
				)));
			}
		};
		map.insert(key, header_val);
	}
	Ok(Some(map))
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
#[path = "aip_web_tests.rs"]
mod tests;

// endregion: --- Tests

#[allow(dead_code)]
pub fn native_function_installer() -> crate::NativeFunctionInstaller {
	std::sync::Arc::new(|lua| {
		let aip: mlua::Table = lua.globals().get("aip")?;
		let web: mlua::Table = aip.get("web")?;
		web.set("UA_AIPROG", DEFAULT_UA_AIPROG)?;
		web.set("UA_BROWSER", DEFAULT_UA_BROWSER)?;
		Ok(())
	})
}

#[allow(dead_code)]
pub fn install_constants(engine: &crate::LuaEngine) -> crate::Result<()> {
	engine.set_value_at_path("aip.web.UA_AIPROG", DEFAULT_UA_AIPROG)?;
	engine.set_value_at_path("aip.web.UA_BROWSER", DEFAULT_UA_BROWSER)?;
	Ok(())
}
