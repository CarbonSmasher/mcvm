pub mod eval;
pub mod reg;
pub mod repo;

use crate::io::files::{self, paths::Paths};
use crate::net::download::{Download, DownloadError};

use std::fs;
use std::path::PathBuf;

use self::eval::eval::{EvalError, EvalPermissions};
use self::eval::parse::{ParseError, Parsed};
use self::reg::{PkgIdentifier, PkgRequest};
use self::repo::RepoError;

static PKG_EXTENSION: &str = ".pkg.txt";

#[derive(Debug, thiserror::Error)]
pub enum PkgError {
	#[error("File operation failed:\n{}", .0)]
	Io(#[from] std::io::Error),
	#[error("Download failed:\n{}", .0)]
	Download(#[from] DownloadError),
	#[error("Error in repository:\n{}", .0)]
	Repo(#[from] RepoError),
	#[error("Failed to parse package:\n{}", .0)]
	Parse(#[from] ParseError),
	#[error("Failed to evaluate package:\n{}", .0)]
	Eval(#[from] EvalError),
}

// Data pertaining to the contents of a package
#[derive(Debug)]
pub struct PkgData {
	contents: String,
	parsed: Option<Parsed>,
}

impl PkgData {
	pub fn new(contents: &str) -> Self {
		Self {
			contents: contents.to_owned(),
			parsed: None,
		}
	}

	pub fn get_contents(&self) -> String {
		self.contents.clone()
	}
}

// Type of a package
#[derive(Debug, Clone)]
pub enum PkgKind {
	Local(PathBuf),         // Contained on the local filesystem
	Remote(Option<String>), // Contained on an external repository
}

// An installable package that loads content into your game
#[derive(Debug)]
pub struct Package {
	pub id: PkgIdentifier,
	pub kind: PkgKind,
	pub data: Option<PkgData>,
}

impl Package {
	pub fn new(name: &str, version: &str, kind: PkgKind) -> Self {
		Self {
			id: PkgIdentifier::new(name, version),
			kind,
			data: None,
		}
	}

	// Get the cached file name of the package
	pub fn filename(&self) -> String {
		self.id.name.clone() + "_" + &self.id.version + PKG_EXTENSION
	}

	// Get the cached path of the package
	pub fn cached_path(&self, paths: &Paths) -> PathBuf {
		let cache_dir = paths.project.cache_dir().join("pkg");
		cache_dir.join(self.filename())
	}

	// Remove the cached package file
	pub fn remove_cached(&self, paths: &Paths) -> Result<(), PkgError> {
		let path = self.cached_path(paths);
		if path.exists() {
			fs::remove_file(path)?;
		}
		Ok(())
	}

	// Ensure the raw contents of the package
	pub fn ensure_loaded(&mut self, paths: &Paths, force: bool) -> Result<(), PkgError> {
		if self.data.is_none() {
			match &self.kind {
				PkgKind::Local(path) => {
					self.data = Some(PkgData::new(&fs::read_to_string(path)?));
				}
				PkgKind::Remote(url) => {
					let cache_dir = paths.project.cache_dir().join("pkg");
					files::create_dir(&cache_dir)?;
					let path = self.cached_path(paths);
					if !force && path.exists() {
						self.data = Some(PkgData::new(&fs::read_to_string(path)?));
					} else {
						let url = url.as_ref().expect("URL for remote package missing");
						let mut dwn = Download::new();
						dwn.url(url)?;
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
		let package = Package::new("sodium", "latest", PkgKind::Remote(None));
		assert_eq!(
			package.filename(),
			"sodium_latest".to_owned() + PKG_EXTENSION
		);

		let package = Package::new("fabriclike-api", "1.3.2", PkgKind::Remote(None));
		assert_eq!(
			package.filename(),
			"fabriclike-api_1.3.2".to_owned() + PKG_EXTENSION
		);
	}
}

// Config for a package, stored in a profile
#[derive(Debug)]
pub struct PkgConfig {
	pub req: PkgRequest,
	pub features: Vec<String>,
	pub permissions: EvalPermissions,
}
