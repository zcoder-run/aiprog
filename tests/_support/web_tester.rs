use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct RequestSnapshot {
	pub method: String,
	pub path: String,
	pub headers: HashMap<String, String>,
	pub body: String,
}

pub struct TestServer {
	addr: SocketAddr,
	handle: JoinHandle<io::Result<()>>,
	request: Arc<Mutex<Option<RequestSnapshot>>>,
}

impl TestServer {
	pub fn url(&self) -> String {
		format!("http://{}", self.addr)
	}

	pub fn path_url(&self, path: &str) -> String {
		format!("{}{path}", self.url())
	}

	pub fn request(&self) -> io::Result<RequestSnapshot> {
		let request = self
			.request
			.lock()
			.map_err(|_| io::Error::other("Test server request lock was poisoned"))?;
		request
			.clone()
			.ok_or_else(|| io::Error::other("Test server did not receive a request"))
	}

	pub async fn close(self) -> io::Result<()> {
		self.handle
			.await
			.map_err(|err| io::Error::other(format!("Test server task failed: {err}")))?
	}
}

#[derive(Default)]
pub struct TestServerBuilder {
	status: u16,
	headers: Vec<(String, String)>,
	body: Vec<u8>,
}

impl TestServerBuilder {
	pub fn with_status(mut self, status: u16) -> Self {
		self.status = status;
		self
	}

	pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
		self.headers.push((name.into(), value.into()));
		self
	}

	pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
		self.body = body.into();
		self
	}

	pub async fn start(self) -> io::Result<TestServer> {
		let listener = TcpListener::bind("127.0.0.1:0").await?;
		let addr = listener.local_addr()?;
		let request = Arc::new(Mutex::new(None));
		let request_for_task = Arc::clone(&request);
		let status = if self.status == 0 { 200 } else { self.status };

		let handle = tokio::spawn(async move {
			let (stream, _) = listener.accept().await?;
			let snapshot = read_request(stream, status, &self.headers, &self.body).await?;
			let mut captured_request = request_for_task
				.lock()
				.map_err(|_| io::Error::other("Test server request lock was poisoned"))?;
			*captured_request = Some(snapshot);
			Ok(())
		});

		Ok(TestServer { addr, handle, request })
	}
}

async fn read_request(
	mut stream: TcpStream,
	status: u16,
	headers: &[(String, String)],
	body: &[u8],
) -> io::Result<RequestSnapshot> {
	let mut request_bytes = Vec::new();
	let mut buffer = [0_u8; 4096];

	loop {
		let read_count = stream.read(&mut buffer).await?;
		if read_count == 0 {
			return Err(io::Error::new(
				io::ErrorKind::UnexpectedEof,
				"HTTP request ended before its complete body was received",
			));
		}

		request_bytes.extend_from_slice(&buffer[..read_count]);

		if request_bytes.len() > 65_536 {
			return Err(io::Error::new(
				io::ErrorKind::InvalidData,
				"HTTP request exceeds the test server size limit",
			));
		}

		if let Some(headers_end) = request_bytes.windows(4).position(|window| window == b"\r\n\r\n") {
			let content_length = request_content_length(&request_bytes[..headers_end])?;
			if request_bytes.len() >= headers_end + 4 + content_length {
				let snapshot = parse_request(&request_bytes[..headers_end + 4 + content_length])?;
				write_response(&mut stream, status, headers, body).await?;
				return Ok(snapshot);
			}
		}
	}
}

fn request_content_length(headers: &[u8]) -> io::Result<usize> {
	let headers = std::str::from_utf8(headers)
		.map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Request headers are not UTF-8"))?;

	for line in headers.lines().skip(1) {
		if let Some((name, value)) = line.split_once(':')
			&& name.trim().eq_ignore_ascii_case("content-length")
		{
			return value
				.trim()
				.parse::<usize>()
				.map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Request Content-Length is invalid"));
		}
	}

	Ok(0)
}

fn parse_request(request: &[u8]) -> io::Result<RequestSnapshot> {
	let headers_end = request
		.windows(4)
		.position(|window| window == b"\r\n\r\n")
		.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Request headers are incomplete"))?;
	let headers_text = std::str::from_utf8(&request[..headers_end])
		.map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Request headers are not UTF-8"))?;

	let mut lines = headers_text.lines();
	let request_line = lines
		.next()
		.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Request line is missing"))?;
	let mut request_parts = request_line.split_whitespace();
	let method = request_parts
		.next()
		.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Request method is missing"))?;
	let path = request_parts
		.next()
		.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Request path is missing"))?;

	let mut headers = HashMap::new();
	for line in lines {
		if let Some((name, value)) = line.split_once(':') {
			headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
		}
	}

	Ok(RequestSnapshot {
		method: method.to_string(),
		path: path.to_string(),
		headers,
		body: String::from_utf8_lossy(&request[headers_end + 4..]).to_string(),
	})
}

async fn write_response(
	stream: &mut TcpStream,
	status: u16,
	headers: &[(String, String)],
	body: &[u8],
) -> io::Result<()> {
	let mut response = format!(
		"HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n",
		body.len()
	);

	for (name, value) in headers {
		response.push_str(&format!("{name}: {value}\r\n"));
	}

	response.push_str("\r\n");
	stream.write_all(response.as_bytes()).await?;
	stream.write_all(body).await
}
