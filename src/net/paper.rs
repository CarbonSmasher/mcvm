use std::path::PathBuf;

use anyhow::{anyhow, Context};
use mcvm_core::net::download;
use mcvm_shared::Side;
use reqwest::Client;
use serde::Deserialize;

use crate::io::files::paths::Paths;

/// Get the newest build number of Paper
pub async fn get_newest_build(version: &str, client: &Client) -> anyhow::Result<u16> {
	let url = format!("https://api.papermc.io/v2/projects/paper/versions/{version}");
	let resp =
		serde_json::from_str::<VersionInfoResponse>(&client.get(url).send().await?.text().await?)?;

	let build = resp
		.builds
		.last()
		.ok_or(anyhow!("Could not find a valid Paper version"))?;

	Ok(*build)
}

#[derive(Deserialize)]
struct VersionInfoResponse {
	builds: Vec<u16>,
}

/// Get the name of the Paper JAR file in the API.
/// This does not represent the name of the file when downloaded
/// as it will be stored in the core JAR location
pub async fn get_jar_file_name(
	version: &str,
	build_num: u16,
	client: &Client,
) -> anyhow::Result<String> {
	let num_str = build_num.to_string();
	let url =
		format!("https://api.papermc.io/v2/projects/paper/versions/{version}/builds/{num_str}");
	let resp = serde_json::from_str::<BuildInfoResponse>(&download::text(&url, client).await?)?;

	Ok(resp.downloads.application.name)
}

#[derive(Deserialize)]
struct BuildInfoResponse {
	downloads: BuildInfoDownloads,
}

#[derive(Deserialize)]
struct BuildInfoDownloads {
	application: BuildInfoApplication,
}

#[derive(Deserialize)]
struct BuildInfoApplication {
	name: String,
}

/// Download the Paper server jar
pub async fn download_server_jar(
	version: &str,
	build_num: u16,
	file_name: &str,
	paths: &Paths,
	client: &Client,
) -> anyhow::Result<()> {
	let num_str = build_num.to_string();
	let url = format!("https://api.papermc.io/v2/projects/paper/versions/{version}/builds/{num_str}/downloads/{file_name}");

	let file_path = get_local_jar_path(version, paths);
	download::file(&url, &file_path, client)
		.await
		.context("Failed to download file")?;

	Ok(())
}

/// Get the path to the stored Paper JAR file
pub fn get_local_jar_path(version: &str, paths: &Paths) -> PathBuf {
	mcvm_core::io::minecraft::game_jar::get_path(Side::Server, version, Some("paper"), &paths.core)
}
