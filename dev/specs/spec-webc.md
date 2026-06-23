# WebClient Module (webc)

## Intent

Provide a reusable, async HTTP client abstraction (`WebClient`) for use across the crate. The initial scope is limited to performing HTTP GET requests. The implementation is built on top of `reqwest`, but the abstraction hides `reqwest` internals behind crate-owned types so that callers do not depend on a specific `reqwest` version directly.

`WebClient` supports multiple response body formats: plain text, JSON (parsed into `serde_json::Value`), and raw binary. Callers select the desired format per request via `BodyFormat`.

The primary consumer is the `aip_web` Lua module (`src/script/modules/aip_web.rs`), but the design is generic enough to serve other modules in the future.

## Public API

### Error and Result

```rust
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Display, From)]
pub enum Error {
    #[from(String, &String, &str)]
    #[display("Error: {_0}")]
    Custom(String),

    #[display("Build failed: {_0}")]
    BuildFailed(String),

    #[display("Request failed: {_0}")]
    RequestFailed(String),

    #[display("Body parse failed: {_0}")]
    BodyParseFailed(String),
}

impl Error {
    pub fn custom(val: impl Into<String>) -> Self;
    pub fn build_failed(val: impl Into<String>) -> Self;
    pub fn request_failed(val: impl Into<String>) -> Self;
    pub fn body_parse_failed(val: impl Into<String>) -> Self;
    pub fn custom_from_err(err: impl std::error::Error) -> Self;
}

impl std::error::Error for Error {}
```

### Client and builder

```rust
pub struct WebClient { /* private inner reqwest::Client */ }

pub struct WebClientBuilder { /* fields */ }

impl WebClientBuilder {
    pub fn new() -> Self;
    pub fn with_default_user_agent(self, ua: impl Into<String>) -> Self;
    pub fn with_redirect_limit(self, limit: usize) -> Self;
    pub fn build(self) -> Result<WebClient>;
}

impl Default for WebClientBuilder {
    fn default() -> Self { Self::new() }
}
```

### Request types

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum HeaderValue {
    Single(String),
    Many(Vec<String>),
}

/// Request body to send in a POST (or other) request.
#[derive(Debug, Clone)]
pub enum RequestBody {
    Json(serde_json::Value),
    Text(String),
}

#[derive(Debug, Clone, Default)]
pub enum BodyFormat {
    #[default]
    Text,
    Json,
    Binary,
}

/// Parameters for a GET web request.
pub struct WebParams {
    pub url: String,
    pub user_agent: Option<String>,
    pub headers: Option<HashMap<String, HeaderValue>>,
    pub body_format: BodyFormat,
}

/// Parameters for a POST web request.
pub struct WebPostParams {
    pub url: String,
    /// Per-request User-Agent override. `None` uses the client's default.
    pub user_agent: Option<String>,
    /// Additional headers to merge with the client's defaults.
    pub headers: Option<HashMap<String, HeaderValue>>,
    /// Request body. `None` means no body is sent.
    pub body: Option<RequestBody>,
    /// Desired format for the response body. Defaults to `Text`.
    pub body_format: BodyFormat,
}
```

### Response types

```rust
#[derive(Debug, Clone)]
pub enum Body {
    Text(String),
    Json(serde_json::Value),
    Binary(Vec<u8>),
}

pub struct WebResponse {
    pub status: u16,
    pub success: bool,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub content_type: String,
    pub body: Body,
}
```

### WebClient method

```rust
impl WebClient {
    pub async fn web_get(&self, params: WebParams) -> Result<WebResponse>;
    pub async fn web_post(&self, params: WebPostParams) -> Result<WebResponse>;
}
```

## Design Considerations

- **Thin abstraction**: `WebClient` only exposes `web_get`. Additional methods (`web_post`, etc.) can be added later without breaking changes.
- **JSON parsing in WebClient**: Callers request JSON parsing per-request via `BodyFormat::Json`. `WebClient` handles the parsing internally and returns a `Body::Json(serde_json::Value)`. This avoids duplicating JSON parsing logic across callers and lets the Lua module focus on converting `serde_json::Value` into Lua types. Callers that need raw text or binary use `BodyFormat::Text` or `BodyFormat::Binary`.
- **Binary body support**: `BodyFormat::Binary` returns raw bytes as `Body::Binary(Vec<u8>)`. The initial `aip_web` consumer does not require binary responses, but the capability is available for future callers.
- **Body parse failures**: When `BodyFormat::Json` is requested but the response body is not valid JSON, `WebClient` returns `Err(Error::BodyParseFailed)`. This distinguishes transport-level errors from content-level errors.
- **POST support**: `web_post` accepts a `WebPostParams` with an optional `RequestBody` that can carry JSON or plain text. The response is handled identically to `web_get` (via a shared `WebResponse` type).
- **`WebParams` vs `WebPostParams`**: `web_get` uses `WebParams`; `web_post` uses `WebPostParams` which adds a `body` field. The GET parameter type is simply `WebParams` (not `WebGetParams`), while POST uses `WebPostParams` for clarity.
- **`redirect_limit` is a client-level setting**: `reqwest` configures redirect policy on the `Client`. The `WebClient` builder accepts `with_redirect_limit`. If per-request redirect limits are needed, the caller can create separate `WebClient` instances. The initial scope does not require per-request redirect limits.
- **Non-success HTTP responses are not errors**: HTTP 4xx/5xx responses are returned as `Ok(WebGetResponse)` with `success: false`. Only transport-level or client-build failures produce `Err(Error)`. Callers (like the Lua module) can generate an `error` message from the status code if needed.
- **User-Agent defaults**: The `WebClient` builder accepts a default user-agent string. The Lua module's `UA_AIPROG` and `UA_BROWSER` constants remain Lua-specific.
- **`reqwest` isolation**: `reqwest` types are not exposed in the public API. `WebClient` re-maps `reqwest::Error` to `Error` variants internally.
- **Thread safety and async**: `reqwest::Client` is `Send + Sync`. `WebClient` follows the same pattern. A single instance can be shared across tasks (e.g., via `Arc<WebClient>`).
- **Builder infallibility**: Builder `with_*` methods are infallible; only `build()` may fail (e.g., TLS backend initialization errors).
- **Header value multiplicity**: `HeaderValue` supports both single and multiple values per header name, matching HTTP semantics. Response headers with multiple values are joined with `", "` in the response map.
- **Potential future extensions**: `web_post`, `web_put`, request timeouts, proxy support, TLS configuration, cookie store, etc.

## Implementation

The module is implemented in the following files:

- `src/webc/error.rs` – `Error` enum, `Result` alias, helper constructors.
- `src/webc/web_client.rs` – `WebClient`, `WebClientBuilder`, request/response types, `web_get`.
- `src/webc/mod.rs` – module declarations and re-exports.

The crate root (`src/lib.rs`) exposes the module via `pub mod webc;`.
