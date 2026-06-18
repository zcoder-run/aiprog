use std::collections::HashMap;
use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::select;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::{Error, Result};

/// Snapshot of a single HTTP request, for use in validator closures.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RequestSnapshot {
	pub method: String,
	pub path: String,
	pub headers: HashMap<String, String>,
	pub body: String,
}

/// A local HTTP test server.
///
/// The server listens on a random port, spawns a background task to accept
/// connections, and responds to every received request with a configured
/// response. Request validation callbacks can be attached through the builder.
pub struct TestServer {
	addr: SocketAddr,
	handle: Option<JoinHandle<()>>,
	shutdown_tx: Option<oneshot::Sender<()>>,
}

impl TestServer {
	/// Returns the base URL (e.g. `http://127.0.0.1:<port>`).
	pub fn url(&self) -> String {
		format!("http://{}", self.addr)
	}

	/// Convenience: returns `{base_url}{path}`.
	pub fn path_url(&self, path: &str) -> String {
		format!("{}{path}", self.url())
	}

	/// Gracefully shuts down the server and waits for the background task.
	pub async fn close(mut self) -> Result<()> {
		if let Some(tx) = self.shutdown_tx.take() {
			let _ = tx.send(());
		}
		if let Some(handle) = self.handle.take() {
			handle.await.map_err(|e| Error::cc("TestServer task panicked", e))?;
		}
		Ok(())
	}
}

pub type TestValidator = Box<dyn Fn(&RequestSnapshot) + Send + Sync + 'static>;

/// Builder for `TestServer`.
pub struct TestServerBuilder {
	status: u16,
	headers: Vec<(String, String)>,
	body: Vec<u8>,
	validator: Option<TestValidator>,
}

impl TestServerBuilder {
	pub fn new() -> Self {
		Self {
			status: 200,
			headers: Vec::new(),
			body: Vec::new(),
			validator: None,
		}
	}

	pub fn status(mut self, code: u16) -> Self {
		self.status = code;
		self
	}

	pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
		self.headers.push((name.into(), value.into()));
		self
	}
#[allow(dead_code)]
	pub fn body_bytes(mut self, data: impl Into<Vec<u8>>) -> Self {
		self.body = data.into();
		self
	}

	pub fn body(mut self, text: impl Into<String>) -> Self {
		self.body = text.into().into_bytes();
		self
	}

	/// Attach a validation closure that receives a parsed snapshot of the
	/// incoming request. The closure can assert or collect information.
	pub fn validate(mut self, f: impl Fn(&RequestSnapshot) + Send + Sync + 'static) -> Self {
		self.validator = Some(Box::new(f));
		self
	}

	/// Start the server and return the running instance.
	pub async fn start(self) -> Result<TestServer> {
		let listener = TcpListener::bind("127.0.0.1:0").await?;
		let addr = listener.local_addr()?;

		let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
		let status = self.status;
		let headers = self.headers;
		let body = self.body;
		let validator = self.validator;

		let handle = tokio::spawn(async move {
			run_server(listener, shutdown_rx, status, headers, body, validator).await;
		});

		Ok(TestServer {
			addr,
			handle: Some(handle),
			shutdown_tx: Some(shutdown_tx),
		})
	}
}

// region:    --- Support

async fn run_server(
	listener: TcpListener,
	mut shutdown_rx: oneshot::Receiver<()>,
	status: u16,
	headers: Vec<(String, String)>,
	body: Vec<u8>,
	validator: Option<TestValidator>,
) {
	loop {
		select! {
			_ = &mut shutdown_rx => {
				break;
			}
			result = listener.accept() => {
				match result {
					Ok((stream, _)) => {
						let validator_ref: Option<&(dyn Fn(&RequestSnapshot) + Send + Sync)> =
							validator.as_deref();
						if let Err(e) = handle_connection(
							stream,
							status,
							&headers,
							&body,
							validator_ref,
						)
						.await
						{
							// Log the error but do not crash the server loop.
							eprintln!("TestServer connection error: {e}");
						}
					}
					Err(e) => {
						// If the listener is closed (e.g., shutdown), break.
						eprintln!("TestServer accept error: {e}");
						break;
					}
				}
			}
		}
	}
}

async fn handle_connection(
	mut stream: tokio::net::TcpStream,
	status: u16,
	headers: &[(String, String)],
	body: &[u8],
	validator: Option<&(dyn Fn(&RequestSnapshot) + Send + Sync)>,
) -> Result<()> {
	// Read the entire request into a buffer (assumes request fits in 8 KiB).
	let mut buf = [0u8; 8192];
	let n = stream.read(&mut buf).await?;
	let request = parse_request(&buf[..n])?;

	// Call the validator if one is configured.
	if let Some(v) = validator {
		v(&request);
	}

	// Build and write the response.
	let mut response = Vec::new();
	// Status line
	response.extend_from_slice(format!("HTTP/1.1 {status} OK\r\n").as_bytes());
	// Headers
	for (name, value) in headers {
		response.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
	}
	// Content-Length
	response.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
	response.extend_from_slice(b"\r\n");
	// Body
	response.extend_from_slice(body);

	stream.write_all(&response).await?;

	Ok(())
}

fn parse_request(buf: &[u8]) -> Result<RequestSnapshot> {
	// Find the end of headers (double CRLF)
	let headers_end = buf
		.windows(4)
		.position(|w| w == b"\r\n\r\n")
		.ok_or_else(|| Error::custom("Invalid HTTP request: missing double CRLF"))?;

	let headers_bytes = &buf[..headers_end];
	let body_bytes = &buf[headers_end + 4..];

	let headers_str = std::str::from_utf8(headers_bytes).map_err(|_| Error::custom("Non-UTF8 request headers"))?;

	let mut lines = headers_str.lines();
	let request_line = lines.next().ok_or_else(|| Error::custom("Empty request"))?;
	let mut parts = request_line.split_whitespace();
	let method = parts.next().ok_or_else(|| Error::custom("Missing method"))?;
	let path = parts.next().ok_or_else(|| Error::custom("Missing path"))?;

	let mut headers = HashMap::new();
	for line in lines {
		if line.is_empty() {
			break;
		}
		if let Some((name, value)) = line.split_once(':') {
			headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
		}
	}

	// Determine the body content from Content-Length.
	let content_length = headers
		.get("content-length")
		.and_then(|cl| cl.parse::<usize>().ok())
		.unwrap_or(0);
	let body = if content_length > 0 {
		let end = std::cmp::min(content_length, body_bytes.len());
		String::from_utf8_lossy(&body_bytes[..end]).to_string()
	} else {
		String::new()
	};

	Ok(RequestSnapshot {
		method: method.to_string(),
		path: path.to_string(),
		headers,
		body,
	})
}

// endregion: --- Support
