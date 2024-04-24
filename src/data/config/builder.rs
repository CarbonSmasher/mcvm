use std::collections::HashMap;

use anyhow::bail;
use mcvm_core::auth_crate::mc::ClientId;
use mcvm_core::user::{User, UserManager};
use mcvm_core::util::versions::MinecraftVersionDeser;
use mcvm_plugin::plugin::PluginManifest;
use mcvm_shared::modifications::{ClientType, Modloader, Proxy, ServerType};
use mcvm_shared::output::MCVMOutput;
use mcvm_shared::pkg::{PackageID, PackageStability};
use mcvm_shared::Side;

use crate::data::id::{InstanceID, ProfileID};
use crate::data::instance::Instance;
use crate::data::profile::Profile;
use crate::io::snapshot;
use crate::pkg::eval::EvalPermissions;
use crate::pkg::reg::PkgRegistry;
use crate::pkg::repo::PkgRepo;

use super::instance::{
	read_instance_config, ClientWindowConfig, CommonInstanceConfig, FullInstanceConfig,
	InstanceConfig, LaunchConfig,
};
use super::package::{FullPackageConfig, PackageConfigDeser, PackageConfigSource};
use super::plugin::{PluginConfig, PluginManager};
use super::preferences::ConfigPreferences;
use super::profile::{ProfileConfig, ProfilePackageConfiguration};
use super::user::{UserConfig, UserVariant};
use super::Config;

/// Simple builder for config
pub struct ConfigBuilder {
	users: UserManager,
	profiles: HashMap<ProfileID, Profile>,
	packages: PkgRegistry,
	preferences: ConfigPreferences,
	global_packages: Vec<PackageConfigDeser>,
	plugins: PluginManager,
	default_user: Option<String>,
}

impl ConfigBuilder {
	/// Construct a new ConfigBuilder
	pub fn new(prefs: ConfigPreferences, repos: Vec<PkgRepo>) -> Self {
		let packages = PkgRegistry::new(repos, prefs.package_caching_strategy.clone());
		Self {
			users: UserManager::new(ClientId::new("".into())),
			profiles: HashMap::new(),
			packages,
			preferences: prefs,
			plugins: PluginManager::new(),
			global_packages: Vec::new(),
			default_user: None,
		}
	}

	/// Create a UserBuilder
	pub fn user(&mut self, id: String, kind: UserBuilderKind) -> UserBuilder {
		UserBuilder::with_parent(id, kind, Some(self))
	}

	/// Finish a UserBuilder
	fn build_user(&mut self, user: User) {
		self.users.add_user(user);
	}

	/// Create a ProfileBuilder
	pub fn profile(&mut self, id: ProfileID, version: MinecraftVersionDeser) -> ProfileBuilder {
		ProfileBuilder::with_parent(id, version, Some(self))
	}

	/// Finish a ProfileBuilder
	fn build_profile(&mut self, id: ProfileID, profile: Profile) {
		self.profiles.insert(id, profile);
	}

	/// Create a PackageBuilder
	pub fn package(
		&mut self,
		data: InitialPackageData,
	) -> PackageBuilder<PackageBuilderConfigParent<'_>> {
		let parent = PackageBuilderConfigParent(self);
		PackageBuilder::with_parent(data, parent)
	}

	/// Finish a PackageBuilder
	fn build_package(&mut self, package: FullPackageConfig) {
		let config = PackageConfigDeser::Full(package);
		self.global_packages.push(config);
	}

	/// Set the default user
	pub fn default_user(&mut self, user_id: String) -> &mut Self {
		self.default_user = Some(user_id);

		self
	}

	/// Add a plugin configuration
	pub fn add_plugin(
		&mut self,
		plugin: PluginConfig,
		manifest: PluginManifest,
		o: &mut impl MCVMOutput,
	) -> anyhow::Result<()> {
		self.plugins.add_plugin(plugin, manifest, o)
	}

	/// Finishes the builder
	pub fn build(mut self) -> anyhow::Result<Config> {
		if let Some(default_user_id) = &self.default_user {
			if self.users.user_exists(default_user_id) {
				self.users
					.choose_user(default_user_id)
					.expect("Default user should exist");
			} else {
				bail!("Provided default user '{default_user_id}' does not exist");
			}
		}

		let global_packages = self
			.global_packages
			.into_iter()
			.map(|x| x.to_package_config(PackageStability::default(), PackageConfigSource::Global))
			.collect();

		Ok(Config {
			users: self.users,
			profiles: self.profiles,
			packages: self.packages,
			global_packages,
			plugins: self.plugins,
			prefs: self.preferences,
		})
	}
}

/// Builder for a User
pub struct UserBuilder<'parent> {
	id: String,
	config: UserConfig,
	parent: Option<&'parent mut ConfigBuilder>,
}

impl<'parent> UserBuilder<'parent> {
	/// Construct a new UserBuilder
	pub fn new(id: String, kind: UserBuilderKind) -> Self {
		Self::with_parent(id, kind, None)
	}

	/// Construct with a parent
	fn with_parent(
		id: String,
		kind: UserBuilderKind,
		parent: Option<&'parent mut ConfigBuilder>,
	) -> Self {
		let variant = match kind {
			UserBuilderKind::Microsoft => UserVariant::Microsoft {},
			UserBuilderKind::Demo => UserVariant::Demo {},
		};
		Self {
			id,
			config: UserConfig { variant },
			parent,
		}
	}

	/// Finish the builder and go to the parent
	pub fn build(self) {
		let (user, parent) = self.build_self();
		if let Some(parent) = parent {
			parent.build_user(user);
		}
	}

	/// Finish the builder and return the self
	pub fn build_self(self) -> (User, Option<&'parent mut ConfigBuilder>) {
		let built = self.config.to_user(&self.id);
		(built, self.parent)
	}
}

/// User kind for a UserBuilder
#[derive(Copy, Clone)]
pub enum UserBuilderKind {
	/// A Microsoft user
	Microsoft,
	/// A demo user
	Demo,
}

/// Builder for a profile
pub struct ProfileBuilder<'parent> {
	id: ProfileID,
	config: ProfileConfig,
	instances: HashMap<InstanceID, InstanceConfig>,
	parent: Option<&'parent mut ConfigBuilder>,
}

impl<'parent> ProfileBuilder<'parent> {
	/// Construct a new ProfileBuilder
	pub fn new(id: ProfileID, version: MinecraftVersionDeser) -> Self {
		Self::with_parent(id, version, None)
	}

	/// Construct with a parent
	fn with_parent(
		id: ProfileID,
		version: MinecraftVersionDeser,
		parent: Option<&'parent mut ConfigBuilder>,
	) -> Self {
		let config = ProfileConfig {
			version,
			modloader: Modloader::Vanilla,
			client_type: ClientType::None,
			server_type: ServerType::None,
			proxy: Proxy::None,
			instances: HashMap::new(),
			packages: ProfilePackageConfiguration::Full {
				global: Vec::new(),
				client: Vec::new(),
				server: Vec::new(),
			},
			package_stability: PackageStability::default(),
		};

		Self {
			id,
			config,
			instances: HashMap::new(),
			parent,
		}
	}

	/// Create an InstanceBuilder
	pub fn instance<'this>(
		&'this mut self,
		id: InstanceID,
		side: Side,
	) -> InstanceBuilder<'this, 'parent> {
		InstanceBuilder::with_parent(id, side, Some(self))
	}

	/// Finish an InstanceBuilder
	fn build_instance(&mut self, id: InstanceID, instance: FullInstanceConfig) {
		self.instances.insert(id, InstanceConfig::Full(instance));
	}

	/// Create a PackageBuilder
	pub fn package<'this>(
		&'this mut self,
		group: ProfilePackageGroup,
		data: InitialPackageData,
	) -> PackageBuilder<PackageBuilderProfileParent<'this, 'parent>> {
		let parent = PackageBuilderProfileParent(group, self);
		PackageBuilder::with_parent(data, parent)
	}

	/// Finish a PackageBuilder
	fn build_package(&mut self, group: ProfilePackageGroup, package: FullPackageConfig) {
		let config = PackageConfigDeser::Full(package);
		match group {
			ProfilePackageGroup::Global => self.config.packages.add_global_package(config),
			ProfilePackageGroup::Client => self.config.packages.add_client_package(config),
			ProfilePackageGroup::Server => self.config.packages.add_server_package(config),
		}
	}

	/// Set the modloader of the profile
	pub fn modloader(&mut self, modloader: Modloader) -> &mut Self {
		self.config.modloader = modloader;
		self
	}

	/// Set the client type of the profile
	pub fn client_type(&mut self, client_type: ClientType) -> &mut Self {
		self.config.client_type = client_type;
		self
	}

	/// Set the server type of the profile
	pub fn server_type(&mut self, server_type: ServerType) -> &mut Self {
		self.config.server_type = server_type;
		self
	}

	/// Set the default package stability of the profile
	pub fn package_stability(&mut self, package_stability: PackageStability) -> &mut Self {
		self.config.package_stability = package_stability;
		self
	}

	/// Finish the builder and go to the parent
	pub fn build(self, o: &mut impl MCVMOutput) -> anyhow::Result<()> {
		let (id, profile, parent) = self.build_self(o)?;
		if let Some(parent) = parent {
			parent.build_profile(id, profile);
		}

		Ok(())
	}

	/// Finish the builder and return the self
	pub fn build_self(
		self,
		o: &mut impl MCVMOutput,
	) -> anyhow::Result<(ProfileID, Profile, Option<&'parent mut ConfigBuilder>)> {
		let mut built = self.config.to_profile(self.id.clone());

		let empty_global_packages = Vec::new();
		let global_packages = self
			.parent
			.as_ref()
			.map(|x| &x.global_packages)
			.unwrap_or(&empty_global_packages);

		let default_plugins = PluginManager::new();
		let plugins = if let Some(ref parent) = self.parent {
			&parent.plugins
		} else {
			&default_plugins
		};

		for (instance_id, instance) in self.instances.into_iter() {
			let instance = read_instance_config(
				instance_id,
				&instance,
				&built,
				global_packages,
				&HashMap::new(),
				plugins,
				o,
			)?;
			built.add_instance(instance);
		}

		Ok((self.id, built, self.parent))
	}

	/// Finish the builder and return the raw config
	pub fn build_inner(self) -> (ProfileID, ProfileConfig) {
		(self.id, self.config)
	}
}

/// Builder for an instance
pub struct InstanceBuilder<'parent, 'grandparent> {
	id: InstanceID,
	config: FullInstanceConfig,
	parent: Option<&'parent mut ProfileBuilder<'grandparent>>,
}

impl<'parent, 'grandparent> InstanceBuilder<'parent, 'grandparent> {
	/// Construct a new InstanceBuilder
	pub fn new(id: InstanceID, side: Side) -> Self {
		Self::with_parent(id, side, None)
	}

	/// Construct with a parent
	fn with_parent(
		id: InstanceID,
		side: Side,
		parent: Option<&'parent mut ProfileBuilder<'grandparent>>,
	) -> Self {
		let config = match side {
			Side::Client => FullInstanceConfig::Client {
				options: Default::default(),
				window: Default::default(),
				common: Default::default(),
			},
			Side::Server => FullInstanceConfig::Server {
				options: Default::default(),
				common: Default::default(),
			},
		};

		Self { id, config, parent }
	}

	/// Create a PackageBuilder
	pub fn package<'this>(
		&'this mut self,
		data: InitialPackageData,
	) -> PackageBuilder<PackageBuilderInstanceParent<'this, 'parent, 'grandparent>> {
		let parent = PackageBuilderInstanceParent(self);
		PackageBuilder::with_parent(data, parent)
	}

	/// Finish a PackageBuilder
	fn build_package(&mut self, package: FullPackageConfig) {
		let config = PackageConfigDeser::Full(package);
		match &mut self.config {
			FullInstanceConfig::Client {
				common: CommonInstanceConfig { packages, .. },
				..
			} => packages.push(config),
			FullInstanceConfig::Server {
				common: CommonInstanceConfig { packages, .. },
				..
			} => packages.push(config),
		};
	}

	/// Set the launch options of the instance
	pub fn launch_options(&mut self, launch_options: LaunchConfig) -> &mut Self {
		match &mut self.config {
			FullInstanceConfig::Client {
				common: CommonInstanceConfig { launch, .. },
				..
			} => *launch = launch_options,
			FullInstanceConfig::Server {
				common: CommonInstanceConfig { launch, .. },
				..
			} => *launch = launch_options,
		};

		self
	}

	/// Set the client window config of the instance
	pub fn window_config(&mut self, window_config: ClientWindowConfig) -> &mut Self {
		match &mut self.config {
			FullInstanceConfig::Client { window, .. } => *window = window_config,
			FullInstanceConfig::Server { .. } => {}
		};

		self
	}

	/// Set the datapack folder of the instance
	pub fn datapack_folder(&mut self, folder: String) -> &mut Self {
		match &mut self.config {
			FullInstanceConfig::Client {
				common: CommonInstanceConfig {
					datapack_folder, ..
				},
				..
			} => *datapack_folder = Some(folder),
			FullInstanceConfig::Server {
				common: CommonInstanceConfig {
					datapack_folder, ..
				},
				..
			} => *datapack_folder = Some(folder),
		};

		self
	}

	/// Set the snapshot config of the instance
	pub fn snapshot_config(&mut self, snapshot_config: snapshot::Config) -> &mut Self {
		match &mut self.config {
			FullInstanceConfig::Client {
				common: CommonInstanceConfig { snapshots, .. },
				..
			} => *snapshots = Some(snapshot_config),
			FullInstanceConfig::Server {
				common: CommonInstanceConfig { snapshots, .. },
				..
			} => *snapshots = Some(snapshot_config),
		};

		self
	}

	/// Finish the builder and go to the parent
	pub fn build(self) {
		if let Some(parent) = self.parent {
			parent.build_instance(self.id, self.config);
		}
	}

	/// Finish the builder and return the self
	pub fn build_self(
		self,
		profile: &Profile,
		o: &mut impl MCVMOutput,
	) -> anyhow::Result<(
		InstanceID,
		Instance,
		Option<&'parent mut ProfileBuilder<'grandparent>>,
	)> {
		let empty_global_packages = Vec::new();
		let global_packages = self
			.parent
			.as_ref()
			.and_then(|x| x.parent.as_ref())
			.map(|x| &x.global_packages)
			.unwrap_or(&empty_global_packages);

		let default_plugins = PluginManager::new();
		let plugins = if let Some(ref parent) = self.parent {
			if let Some(ref parent) = parent.parent {
				&parent.plugins
			} else {
				&default_plugins
			}
		} else {
			&default_plugins
		};
		let built = read_instance_config(
			self.id.clone(),
			&InstanceConfig::Full(self.config),
			profile,
			global_packages,
			&HashMap::new(),
			plugins,
			o,
		)?;

		Ok((self.id, built, self.parent))
	}
}

/// Builder for a package
pub struct PackageBuilder<Parent: PackageBuilderParent> {
	config: FullPackageConfig,
	parent: Parent,
}

impl<Parent> PackageBuilder<Parent>
where
	Parent: PackageBuilderParent,
{
	/// Construct with a parent
	fn with_parent(data: InitialPackageData, parent: Parent) -> Self {
		let config = FullPackageConfig {
			id: data.id,
			features: Default::default(),
			use_default_features: true,
			permissions: Default::default(),
			stability: Default::default(),
			worlds: Default::default(),
		};

		Self { config, parent }
	}

	/// Add to the package's features
	pub fn features(&mut self, features: Vec<String>) -> &mut Self {
		self.config.features.extend(features);
		self
	}

	/// Set the use_default_features setting of the package
	pub fn use_default_features(&mut self, value: bool) -> &mut Self {
		self.config.use_default_features = value;
		self
	}

	/// Set the permissions of the package
	pub fn permissions(&mut self, permissions: EvalPermissions) -> &mut Self {
		self.config.permissions = permissions;
		self
	}

	/// Set the configured stability of the package
	pub fn stability(&mut self, stability: PackageStability) -> &mut Self {
		self.config.stability = Some(stability);
		self
	}

	/// Set the configured worlds of the package
	pub fn worlds(&mut self, worlds: Vec<String>) -> &mut Self {
		self.config.worlds = worlds;
		self
	}

	/// Finish the builder and go to the parent
	pub fn build(self) {
		self.parent.build_package(self.config);
	}
}

impl PackageBuilder<PackageBuilderOrphan> {
	/// Construct a new PackageBuilder
	pub fn new(data: InitialPackageData) -> Self {
		Self::with_parent(data, PackageBuilderOrphan)
	}
}

/// Initial data for a PackageBuilder
pub struct InitialPackageData {
	id: PackageID,
}

/// Trait for a parent builder that can have a PackageBuilder added
pub trait PackageBuilderParent {
	/// Add the package to the parent
	fn build_package(self, package: FullPackageConfig);
}

/// Data for a PackageBuilder with no parent
pub struct PackageBuilderOrphan;

impl PackageBuilderParent for PackageBuilderOrphan {
	fn build_package(self, _package: FullPackageConfig) {}
}

/// Data for a PackageBuilder that returns to a ProfileBuilder
pub struct PackageBuilderProfileParent<'profile, 'parent>(
	ProfilePackageGroup,
	&'profile mut ProfileBuilder<'parent>,
);

/// The different package groups that a PackageBuilder can return to
pub enum ProfilePackageGroup {
	/// Global
	Global,
	/// Client sided
	Client,
	/// Server sided
	Server,
}

impl<'profile, 'parent> PackageBuilderParent for PackageBuilderProfileParent<'profile, 'parent> {
	fn build_package(self, package: FullPackageConfig) {
		self.1.build_package(self.0, package)
	}
}

/// Data for a PackageBuilder that returns to a InstanceBuilder
pub struct PackageBuilderInstanceParent<'instance, 'parent, 'grandparent>(
	&'instance mut InstanceBuilder<'parent, 'grandparent>,
);

impl<'instance, 'parent, 'grandparent> PackageBuilderParent
	for PackageBuilderInstanceParent<'instance, 'parent, 'grandparent>
{
	fn build_package(self, package: FullPackageConfig) {
		self.0.build_package(package)
	}
}

/// Data for a PackageBuilder that returns to a ConfigBuilder
pub struct PackageBuilderConfigParent<'config>(&'config mut ConfigBuilder);

impl<'config> PackageBuilderParent for PackageBuilderConfigParent<'config> {
	fn build_package(self, package: FullPackageConfig) {
		self.0.build_package(package)
	}
}

#[cfg(test)]
mod tests {
	use mcvm_plugin::api::NoOp;
	use mcvm_shared::lang::Language;

	use crate::data::config::preferences::{PrefDeser, RepositoriesDeser};
	use crate::pkg::reg::CachingStrategy;

	use super::*;

	#[test]
	fn test_config_building() {
		let (prefs, repos) = get_prefs().expect("Failed to get preferences");
		let mut config = ConfigBuilder::new(prefs, repos);
		let mut profile = config.profile(
			"profile".into(),
			MinecraftVersionDeser::Version("1.19.3".into()),
		);
		modify_profile(&mut profile);
		config
			.package(InitialPackageData {
				id: "global-package".into(),
			})
			.build();
		config
			.user("user".into(), UserBuilderKind::Microsoft)
			.build();
		config.default_user("user".into());
		let config = config.build().expect("Failed to build config");
		assert!(config.users.user_exists("user"));
		assert_eq!(
			config.users.get_chosen_user().map(|x| x.get_id().clone()),
			Some("user".into())
		);
	}

	#[test]
	fn test_profile_building() {
		let mut profile = ProfileBuilder::new(
			"profile".into(),
			MinecraftVersionDeser::Version("1.19.3".into()),
		);
		modify_profile(&mut profile);

		let (profile_id, profile, ..) = profile
			.build_self(&mut NoOp)
			.expect("Failed to build profile");
		assert_eq!(profile_id, "profile".into());
		assert!(profile.instances.contains_key("instance"));
		assert_eq!(profile.modifications.client_type, ClientType::Fabric);
	}

	fn modify_profile(profile: &mut ProfileBuilder<'_>) {
		let mut instance = profile.instance("instance".into(), Side::Client);
		let package = instance.package(InitialPackageData {
			id: "instance-package".into(),
		});
		package.build();
		instance.launch_options(LaunchConfig::default());
		instance.build();
		profile.client_type(ClientType::Fabric);
		let mut package = profile.package(
			ProfilePackageGroup::Global,
			InitialPackageData {
				id: "profile-package".into(),
			},
		);
		package.features(vec!["hello".into(), "goodbye".into()]);
		package.build();
	}

	fn get_prefs() -> anyhow::Result<(ConfigPreferences, Vec<PkgRepo>)> {
		let deser = PrefDeser {
			repositories: RepositoriesDeser::default(),
			package_caching_strategy: CachingStrategy::default(),
			language: Language::default(),
		};
		ConfigPreferences::read(&deser)
	}
}
