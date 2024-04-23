use std::collections::HashMap;

use anyhow::Context;
use mcvm_shared::{lang::translate::LanguageMap, output::MCVMOutput};
use serde::Deserialize;

use crate::hooks::Hook;

/// A plugin
#[derive(Debug)]
pub struct Plugin {
	/// The plugin's manifest
	manifest: PluginManifest,
	/// The custom config for the plugin, serialized from JSON
	custom_config: Option<String>,
}

impl Plugin {
	/// Create a new plugin from a manifest
	pub fn new(manifest: PluginManifest) -> Self {
		Self {
			manifest,
			custom_config: None,
		}
	}

	/// Get the manifest of the plugin
	pub fn get_manifest(&self) -> &PluginManifest {
		&self.manifest
	}

	/// Call a hook on the plugin
	pub fn call_hook<H: Hook>(
		&self,
		hook: &H,
		arg: &H::Arg,
		o: &mut impl MCVMOutput,
	) -> anyhow::Result<Option<H::Result>> {
		let Some(handler) = self.manifest.hooks.get(hook.get_name()) else {
			return Ok(None);
		};
		match handler {
			HookHandler::Execute { executable, args } => hook
				.call(executable, arg, args, self.custom_config.clone(), o)
				.map(Some),
		}
	}

	/// Set the custom config of the plugin
	pub fn set_custom_config(&mut self, config: serde_json::Value) -> anyhow::Result<()> {
		let serialized =
			serde_json::to_string(&config).context("Failed to serialize custom plugin config")?;
		self.custom_config = Some(serialized);
		Ok(())
	}
}

/// Configuration for a plugin
#[derive(Deserialize, Debug)]
pub struct PluginManifest {
	/// The hook handlers for the plugin
	#[serde(default)]
	pub hooks: HashMap<String, HookHandler>,
	/// The lanugage map the plugin provides
	#[serde(default)]
	pub language_map: LanguageMap,
}

impl PluginManifest {
	/// Create a new PluginManifest
	pub fn new() -> Self {
		Self {
			hooks: HashMap::new(),
			language_map: LanguageMap::new(),
		}
	}
}

/// A handler for a single hook that a plugin uses
#[derive(Deserialize, Debug)]
#[serde(untagged)]
#[serde(rename_all = "snake_case")]
pub enum HookHandler {
	/// Handle this hook by running an executable
	Execute {
		/// The executable to run
		executable: String,
		/// Arguments for the executable
		#[serde(default)]
		args: Vec<String>,
	},
}
