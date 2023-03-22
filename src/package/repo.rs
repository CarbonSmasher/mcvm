use crate::io::files::paths::Paths;
use crate::net::download::download_text;
use crate::skip_fail;

use serde::Deserialize;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum RepoError {
	#[error("File operation failed:\n{}", .0)]
	Io(#[from] std::io::Error),
	#[error("Failed to parse json file:\n{}", .0)]
	Parse(#[from] serde_json::Error),
	#[error("Download failed:\n{}", .0)]
	Download(#[from] reqwest::Error),
}

// An entry in the index that specifies what package versions are available
#[derive(Debug, Deserialize)]
pub struct PkgEntry {
	// The latest package version available from this repository.
	version: String,
	url: String,
}

// JSON format for a repository index
#[derive(Debug, Deserialize)]
pub struct RepoIndex {
	packages: HashMap<String, PkgEntry>,
}

// A remote source for mcvm packages
#[derive(Debug)]
pub struct PkgRepo {
	pub id: String,
	url: String,
	index: Option<RepoIndex>,
}

impl PkgRepo {
	pub fn new(id: &str, url: &str) -> Self {
		Self {
			id: id.to_owned(),
			url: url.to_owned(),
			index: None,
		}
	}

	// The cached path of the index
	pub fn get_path(&self, paths: &Paths) -> PathBuf {
		paths.pkg_index_cache.join(&self.id)
	}

	// Set the index to serialized json text
	fn set_index(&mut self, index: &str) -> Result<(), RepoError> {
		let parsed = serde_json::from_str::<RepoIndex>(index)?;
		self.index = Some(parsed);
		Ok(())
	}

	// Update the currently cached index file
	pub async fn sync(&mut self, paths: &Paths) -> Result<(), RepoError> {
		let text = download_text(&self.index_url()).await?;
		fs::write(&self.get_path(paths), &text)?;
		self.set_index(&text)?;

		Ok(())
	}

	// Make sure that the repository index is downloaded
	pub async fn ensure_index(&mut self, paths: &Paths) -> Result<(), RepoError> {
		if self.index.is_none() {
			let path = self.get_path(paths);
			if path.exists() {
				match self.set_index(&fs::read_to_string(&path)?) {
					Ok(..) => {}
					Err(..) => {
						self.sync(paths).await?;
						self.set_index(&fs::read_to_string(&path)?)?;
					}
				};
			} else {
				self.sync(paths).await?;
			}
		}
		Ok(())
	}

	fn index_url(&self) -> String {
		self.url.clone() + "/api/mcvm/index.json"
	}

	// Ask if the index has a package and return the url for that package if it exists
	pub async fn query(
		&mut self,
		id: &str,
		paths: &Paths,
	) -> Result<Option<(String, String)>, RepoError> {
		self.ensure_index(paths).await?;
		if let Some(index) = &self.index {
			if let Some(entry) = index.packages.get(id) {
				return Ok(Some((entry.url.clone(), entry.version.clone())));
			}
		}
		Ok(None)
	}
}

// Query a list of repos
pub async fn query_all(
	repos: &mut [PkgRepo],
	name: &str,
	paths: &Paths,
) -> Result<Option<(String, String)>, RepoError> {
	for repo in repos {
		if let Some(result) = skip_fail!(repo.query(name, paths).await) {
			return Ok(Some(result));
		}
	}
	Ok(None)
}
