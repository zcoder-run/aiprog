mod error;
mod web_client;

pub use error::{Error, Result};
pub use web_client::{
	Body, BodyFormat, HeaderValue, RequestBody, WebClient, WebClientBuilder, WebParams, WebPostParams, WebResponse,
};
