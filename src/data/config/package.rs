use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::package::{eval::EvalPermissions, reg::{PkgRequest, PkgRequestSource}, PkgProfileConfig};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageType {
	Local,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
#[serde(rename_all = "snake_case")]
pub enum FullPackageConfig {
	Local {
		r#type: PackageType,
		id: String,
		version: u32,
		path: String,
		#[serde(default)]
		features: Vec<String>,
		#[serde(default)]
		permissions: EvalPermissions,
	},
	Remote {
		id: String,
		version: Option<u32>,
		#[serde(default)]
		features: Vec<String>,
		#[serde(default)]
		permissions: EvalPermissions,
	},
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum PackageConfig {
	Basic(String),
	Full(FullPackageConfig),
}

impl Display for PackageConfig {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Basic(id) => id,
				Self::Full(FullPackageConfig::Local { id, .. }) => id,
				Self::Full(FullPackageConfig::Remote { id, .. }) => id,
			}
		)
	}
}

impl PackageConfig {
	/// Convert this package config into a PkgProfileConfig
	pub fn to_profile_config(&self) -> anyhow::Result<PkgProfileConfig> {
		let package = match self {
			PackageConfig::Basic(id) => PkgProfileConfig {
				req: PkgRequest::new(id, PkgRequestSource::UserRequire),
				features: vec![],
				permissions: EvalPermissions::Standard,
			},
			PackageConfig::Full(FullPackageConfig::Local {
				r#type: _,
				id,
				version: _,
				path: _,
				features,
				permissions,
			}) => PkgProfileConfig {
				req: PkgRequest::new(id, PkgRequestSource::UserRequire),
				features: features.clone(),
				permissions: permissions.clone(),
			},
			PackageConfig::Full(FullPackageConfig::Remote {
				id,
				version: _,
				features,
				permissions,
			}) => PkgProfileConfig {
				req: PkgRequest::new(id, PkgRequestSource::UserRequire),
				features: features.clone(),
				permissions: permissions.clone(),
			},
		};

		Ok(package)
	}
}
