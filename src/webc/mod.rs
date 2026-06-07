mod error;
mod web_client;

pub use error::{Error, Result};
pub use web_client::{
    Body, BodyFormat, HeaderValue, WebClient, WebClientBuilder, WebGetParams, WebGetResponse,
};
