use oauth2::ClientId;

use crate::util::secrets::get_ms_client_id;

/// Configuration for different parts of the core library
pub struct Configuration {
	/// The Microsoft client ID to use for Microsoft authentication
	pub(crate) ms_client_id: ClientId,
	/// Whether to force the reinstallation of files
	pub(crate) force_reinstall: bool,
	/// Whether to allow offline installs
	pub(crate) allow_offline: bool,
	/// Whether to censor user credentials in output messages and logs
	pub(crate) censor_secrets: bool,
	/// Whether to use file copies instead of hardlinks. Useful if you
	/// are on a filesystem that doesn't like hardlinks
	pub(crate) disable_hardlinks: bool,
}

impl Default for Configuration {
	fn default() -> Self {
		Self::new()
	}
}

impl Configuration {
	/// Construct the default configuration
	pub fn new() -> Self {
		Self {
			ms_client_id: get_ms_client_id(),
			force_reinstall: false,
			allow_offline: false,
			censor_secrets: true,
			disable_hardlinks: false,
		}
	}

	/// Get a builder for the configuration
	pub fn builder() -> ConfigBuilder {
		ConfigBuilder::new()
	}
}

/// Simple builder for the configuration
pub struct ConfigBuilder {
	config: Configuration,
}

impl ConfigBuilder {
	/// Start a new ConfigBuilder with default configuration
	pub fn new() -> Self {
		Self {
			config: Configuration::new(),
		}
	}

	/// Finish building and get the configuration
	pub fn build(self) -> Configuration {
		self.config
	}

	/// Set the Microsoft client ID to use for Microsoft / XBox Live authentication
	pub fn ms_client_id(mut self, ms_client_id: ClientId) -> Self {
		self.config.ms_client_id = ms_client_id;
		self
	}

	/// Set whether to force the reinstall of files
	pub fn force_reinstall(mut self, force_reinstall: bool) -> Self {
		self.config.force_reinstall = force_reinstall;
		self
	}

	/// Set whether to allow offline installs
	pub fn allow_offline(mut self, allow_offline: bool) -> Self {
		self.config.allow_offline = allow_offline;
		self
	}

	/// Set whether to censor user credentials in output messages and logs
	pub fn censor_secrets(mut self, censor_secrets: bool) -> Self {
		self.config.censor_secrets = censor_secrets;
		self
	}

	/// Set whether to disable the use of hardlinks
	pub fn disable_hardlinks(mut self, disable_hardlinks: bool) -> Self {
		self.config.disable_hardlinks = disable_hardlinks;
		self
	}
}

impl Default for ConfigBuilder {
	fn default() -> Self {
		Self::new()
	}
}
