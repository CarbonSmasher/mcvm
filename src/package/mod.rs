pub mod eval;
pub mod repo;

use crate::io::files::{self, paths::Paths};
use crate::net::download::{Download, DownloadError};
use eval::parse::PkgAst;
use crate::util::versions::VersionPattern;

use std::path::PathBuf;
use std::fs;

use self::repo::{PkgRepo, query_all, RepoError};

static PKG_EXTENSION: &str = ".pkg.txt";

// Data pertaining to the contents of a package
#[derive(Debug)]
pub struct PkgData {
	contents: String,
	ast: Option<PkgAst>
}

#[derive(Debug, thiserror::Error)]
pub enum PkgError {
	#[error("File operation failed:\n{}", .0)]
	Io(#[from] std::io::Error),
	#[error("Download failed:\n{}", .0)]
	Download(#[from] DownloadError),
	#[error("Package {} not found", .0)]
	NotFound(String),
	#[error("Error in repository:\n{}", .0)]
	Repo(#[from] RepoError)
}

impl PkgData {
	pub fn new(contents: &str) -> Self {
		Self {
			contents: contents.to_owned(),
			ast: None
		}
	}
}

// Type of a package
#[derive(Debug)]
pub enum PkgKind {
	Local(PathBuf), // Contained on the local filesystem
	Remote(Option<String>) // Contained on an external repository
}

// An installable package that loads content into your game
#[derive(Debug)]
pub struct Package {
	pub name: String,
	pub version: VersionPattern,
	pub kind: PkgKind,
	pub data: Option<PkgData>
}

impl Package {
	pub fn new(name: &str, version: VersionPattern, kind: PkgKind) -> Self {
		Self {
			name: name.to_owned(),
			version,
			kind,
			data: None
		}
	}
	
	// Get the combined name and version of the package
	pub fn full_name(&self) -> String {
		self.name.clone() + ":" + self.version.as_string()
	}

	// Get the cached file name of the package
	pub fn filename(&self) -> String {
		self.name.clone() + "_" + self.version.as_string() + PKG_EXTENSION
	}

	// Ensure the raw contents of the package
	pub fn ensure_loaded(&mut self, paths: &Paths, repos: &mut [PkgRepo]) -> Result<(), PkgError> {
		if self.data.is_none() {
			match &self.kind {
				PkgKind::Local(path) => {
					self.data = Some(PkgData::new(&fs::read_to_string(path)?));
				}
				PkgKind::Remote(url) => {
					let cache_dir = paths.project.cache_dir().join("pkg");
					files::create_dir(&cache_dir)?;
					let path = cache_dir.join(self.filename());
					if path.exists() {
						self.data = Some(PkgData::new(&fs::read_to_string(path)?));
					} else {
						let url = match url {
							Some(url) => url.clone(),
							None => query_all(repos, &self.name, &self.version, paths)?
								.ok_or(PkgError::NotFound(self.full_name()))?
						};
						let mut dwn = Download::new();
						dwn.url(&url)?;
						dwn.add_file(&path)?;
						dwn.add_str();
						dwn.perform()?;
						self.data = Some(PkgData::new(&dwn.get_str()?));
					}
				}
			};
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_package_name() {
		let package = Package::new("sodium", VersionPattern::Latest(None), PkgKind::Remote(None));
		assert_eq!(package.full_name(), "sodium:latest");
		assert_eq!(package.filename(), "sodium_latest".to_owned() + PKG_EXTENSION);

		let package = Package::new("fabriclike-api", VersionPattern::Single("1.3.2".to_string()), PkgKind::Remote(None));
		assert_eq!(package.full_name(), "fabriclike-api:1.3.2");
		assert_eq!(package.filename(), "fabriclike-api_1.3.2".to_owned() + PKG_EXTENSION);
	}
}
