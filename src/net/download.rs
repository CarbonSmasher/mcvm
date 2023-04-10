use std::path::Path;

use anyhow::Context;
use cfg_match::cfg_match;
use reqwest::{Client, Url};

// Sensible open file descriptor limit for asynchronous transfers
cfg_match! {
	target_os = "windows" => {
		pub static FD_SENSIBLE_LIMIT: usize = 64;
	}
	_ => {
		pub static FD_SENSIBLE_LIMIT: usize = 16;
	}
}

/// Downloads a file
pub async fn download(url: &str) -> anyhow::Result<reqwest::Response> {
	let resp = Client::new()
		.get(url)
		.send()
		.await
		.context("Failed to send request")?
		.error_for_status()
		.context("Server reported an error")?;

	Ok(resp)
}

/// Downloads and returns text
pub async fn download_text(url: &str) -> anyhow::Result<String> {
	let text = download(url)
		.await
		.context("Failed to download")?
		.text()
		.await
		.context("Failed to convert download to text")?;

	Ok(text)
}

/// Downloads and returns bytes
pub async fn download_bytes(url: &str) -> anyhow::Result<bytes::Bytes> {
	let bytes = download(url)
		.await
		.context("Failed to download")?
		.bytes()
		.await
		.context("Failed to convert download to raw bytes")?;

	Ok(bytes)
}

/// Downloads and puts the contents in a file
pub async fn download_file(url: &str, path: &Path) -> anyhow::Result<()> {
	let bytes = download_bytes(url)
		.await
		.context("Failed to download data")?;
	tokio::fs::write(path, bytes).await.with_context(|| {
		format!(
			"Failed to write downloaded contents to path {}",
			path.display()
		)
	})?;

	Ok(())
}

/// Validates a URL with a helpful error message
pub fn validate_url(url: &str) -> anyhow::Result<()> {
	Url::parse(url)
		.context("It may help to make sure that either http:// or https:// is before the domain name")?;

	Ok(())
}
