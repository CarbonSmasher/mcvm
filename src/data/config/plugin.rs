use crate::io::files::paths::Paths;
use anyhow::Context;
use mcvm_core::io::json_from_file;
use mcvm_shared::output::MCVMOutput;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use mcvm_plugin::hooks::Hook;
use mcvm_plugin::plugin::{Plugin, PluginManifest};
use mcvm_plugin::PluginManager as LoadedPluginManager;

/// User configuration for a plugin
#[derive(Debug)]
pub struct PluginConfig {
	/// The name of the plugin
	pub name: String,
	/// The custom config for the plugin
	pub custom_config: Option<serde_json::Value>,
}

/// Deserialized format for a plugin configuration
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum PluginConfigDeser {
	/// Simple configuration with just the plugin name
	Simple(String),
	/// Full configuration
	Full {
		/// The name of the plugin
		name: String,
		/// The custom config for the plugin
		#[serde(default)]
		custom_config: Option<serde_json::Value>,
	},
}

impl PluginConfigDeser {
	/// Convert this deserialized plugin config to the final version
	pub fn to_config(&self) -> PluginConfig {
		let name = match self {
			Self::Simple(name) | Self::Full { name, .. } => name.clone(),
		};
		let custom_config = match self {
			Self::Simple(..) => None,
			Self::Full { custom_config, .. } => custom_config.clone(),
		};

		PluginConfig {
			name,
			custom_config,
		}
	}
}

/// Manager for plugin configs and the actual loaded plugin manager
#[derive(Debug)]
pub struct PluginManager {
	manager: LoadedPluginManager,
	configs: Vec<PluginConfig>,
}

impl PluginManager {
	/// Create a new PluginManager
	pub fn new() -> Self {
		Self {
			manager: LoadedPluginManager::new(),
			configs: Vec::new(),
		}
	}

	/// Add a plugin to the manager
	pub fn add_plugin(
		&mut self,
		plugin: PluginConfig,
		manifest: PluginManifest,
		o: &mut impl MCVMOutput,
	) -> anyhow::Result<()> {
		let custom_config = plugin.custom_config.clone();
		self.configs.push(plugin);
		let mut plugin = Plugin::new(manifest);
		if let Some(custom_config) = custom_config {
			plugin.set_custom_config(custom_config)?;
		}

		self.manager.add_plugin(plugin, o)?;

		Ok(())
	}

	/// Load a plugin from the plugin directory
	pub fn load_plugin(
		&mut self,
		plugin: PluginConfig,
		paths: &Paths,
		o: &mut impl MCVMOutput,
	) -> anyhow::Result<()> {
		// Get the path for the manifest
		let path = paths.plugins.join(format!("{}.json", plugin.name));
		let path = if path.exists() {
			path
		} else {
			paths.plugins.join(&plugin.name).join("plugin.json")
		};
		let manifest = json_from_file(path).context("Failed to read plugin manifest from file")?;

		self.add_plugin(plugin, manifest, o)?;

		Ok(())
	}

	/// Call a plugin hook on the manager and collects the results into a Vec
	pub fn call_hook<H: Hook>(
		&self,
		hook: H,
		arg: &H::Arg,
		o: &mut impl MCVMOutput,
	) -> anyhow::Result<Vec<H::Result>> {
		self.manager.call_hook(hook, arg, o)
	}
}

impl Default for PluginManager {
	fn default() -> Self {
		Self::new()
	}
}