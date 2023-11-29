use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use mcvm_shared::output::MCVMOutput;
use mcvm_shared::Side;

use crate::io::files::paths::Paths;
use crate::io::files::update_hardlink;
use crate::io::java::classpath::Classpath;
use crate::io::java::install::{JavaInstallParameters, JavaInstallation};
use crate::io::persistent::PersistentData;
use crate::io::update::UpdateManager;
use crate::launch::{LaunchConfiguration, LaunchParameters};
use crate::net::game_files::client_meta::ClientMeta;
use crate::net::game_files::version_manifest::VersionManifestAndList;
use crate::net::game_files::{game_jar, libraries};
use crate::user::UserManager;
use crate::util::versions::VersionName;
use crate::version::{ClientAssetsAndLibraries, ClientAssetsAndLibsParameters};
use crate::InstanceHandle;

/// The default main class for the server
pub const DEFAULT_SERVER_MAIN_CLASS: &str = "net.minecraft.server.Main";

/// An instance of a version which can be launched
pub struct Instance<'params> {
	params: InstanceParameters<'params>,
	config: InstanceConfiguration,
	java: JavaInstallation,
	jar_path: PathBuf,
	classpath: Classpath,
	main_class: String,
}

impl<'params> Instance<'params> {
	pub(crate) async fn load(
		config: InstanceConfiguration,
		params: InstanceParameters<'params>,
		o: &mut impl MCVMOutput,
	) -> anyhow::Result<Instance<'params>> {
		// Start setting up the instance
		std::fs::create_dir_all(&config.path).context("Failed to create instance directory")?;
		if !config.path.is_dir() {
			bail!("Instance directory path is not a directory");
		}

		// Install Java
		let java_vers = &params.client_meta.java_info.major_version;
		let java_params = JavaInstallParameters {
			paths: params.paths,
			update: params.update_manager,
			persistent: params.persistent,
			req_client: params.req_client,
		};
		let java = JavaInstallation::install(
			config.launch.java.clone(),
			java_vers.clone(),
			java_params,
			o,
		)
		.await
		.context("Failed to install or update Java")?;

		params.persistent.finish(params.paths).await?;

		// Get the game jar
		game_jar::get(
			config.side.get_side(),
			params.client_meta,
			&params.version,
			params.paths,
			&params.update_manager,
			params.req_client,
			o,
		)
		.await
		.context("Failed to get the game JAR file")?;

		let mut jar_path = if let Some(jar_path) = &config.jar_path {
			jar_path.clone()
		} else {
			crate::io::minecraft::game_jar::get_path(
				config.side.get_side(),
				&params.version,
				params.paths,
			)
		};
		if !jar_path.exists() {
			bail!("Game JAR does not exist");
		}
		// For the server, the jar file has to be in the launch directory, so we hardlink it
		if let Side::Server = config.side.get_side() {
			let new_jar_path = config.path.join("server.jar");
			// Update the hardlink
			if params.update_manager.should_update_file(&new_jar_path) {
				update_hardlink(&jar_path, &new_jar_path)
					.context("Failed to hardlink server.jar")?;
				params.update_manager.add_file(new_jar_path.clone());
			}
			jar_path = new_jar_path;
		}

		// Load assets and libs for client
		if let Side::Client = config.side.get_side() {
			let sub_params = ClientAssetsAndLibsParameters {
				client_meta: params.client_meta,
				version: params.version,
				paths: params.paths,
				req_client: params.req_client,
				version_manifest: params.version_manifest,
				update_manager: params.update_manager,
			};
			params
				.client_assets_and_libs
				.load(sub_params, o)
				.await
				.context("Failed to load client assets and libraries")?;
		}

		// Classpath
		let mut classpath = Classpath::new();
		if let Side::Client = config.side.get_side() {
			let lib_classpath = libraries::get_classpath(params.client_meta, params.paths)
				.context("Failed to extract classpath from game library list")?;
			classpath.extend(lib_classpath);
		}
		classpath.add_path(&jar_path);

		// Main class
		let main_class = if let Some(main_class) = &config.main_class {
			main_class.clone()
		} else {
			match config.side.get_side() {
				Side::Client => params.client_meta.main_class.clone(),
				Side::Server => DEFAULT_SERVER_MAIN_CLASS.into(),
			}
		};

		// Server EULA
		let eula_path = config.path.join("eula.txt");
		if !eula_path.exists() {
			std::fs::write(eula_path, "eula = true\n").context("Failed to create eula.txt")?;
		}

		Ok(Self {
			config,
			params,
			java,
			jar_path,
			classpath,
			main_class,
		})
	}

	/// Launch the instance and block until the process is finished
	pub async fn launch(&mut self, o: &mut impl MCVMOutput) -> anyhow::Result<()> {
		let mut handle = self.launch_with_handle(o).await?;
		handle.wait().context("Failed to wait instance process")?;
		Ok(())
	}

	/// Launch the instance and get the handle
	pub async fn launch_with_handle(
		&mut self,
		o: &mut impl MCVMOutput,
	) -> anyhow::Result<InstanceHandle> {
		let params = LaunchParameters {
			version: self.params.version,
			version_manifest: self.params.version_manifest,
			side: &self.config.side,
			launch_dir: &self.config.path,
			java: &self.java,
			classpath: &self.classpath,
			main_class: &self.main_class,
			launch_config: &self.config.launch,
			paths: self.params.paths,
			req_client: self.params.req_client,
			client_meta: self.params.client_meta,
			users: self.params.users,
		};
		let handle = crate::launch::launch(params, o)
			.await
			.context("Failed to run launch routine")?;
		Ok(handle)
	}

	/// Get the JAR path of the instance
	pub fn get_jar_path(&self) -> &Path {
		&self.jar_path
	}
}

/// Configuration for an instance
pub struct InstanceConfiguration {
	/// Configuration for the instance side
	pub side: InstanceKind,
	/// The directory where the instance will be stored and launched from
	pub path: PathBuf,
	/// Launch options for the instance
	pub launch: LaunchConfiguration,
	/// JAR path override. If this is set, the default JAR file will not be downloaded
	pub jar_path: Option<PathBuf>,
	/// Java main class override
	pub main_class: Option<String>,
	/// Additional libraries to add to the classpath.
	/// These must be absolute paths to Java libraries already installed on the
	/// system, and will not be installed automatically
	pub additional_libs: Vec<PathBuf>,
}

/// Configuration for what side an instance is, along with configuration
/// specific to that side
pub enum InstanceKind {
	/// Client-side
	Client {
		/// Configuration for the client window
		window: ClientWindowConfig,
	},
	/// Server-side
	Server {},
}

impl InstanceKind {
	/// Get the side of this kind
	pub fn get_side(&self) -> Side {
		match self {
			Self::Client { .. } => Side::Client,
			Self::Server { .. } => Side::Server,
		}
	}
}

/// Configuration for the client window
#[derive(Default, Clone, Debug)]
pub struct ClientWindowConfig {
	/// The resolution of the window
	pub resolution: Option<WindowResolution>,
}

/// Resolution for a client window
#[derive(Clone, Debug, Copy)]
pub struct WindowResolution {
	/// The width of the window
	pub width: u32,
	/// The height of the window
	pub height: u32,
}

/// Container struct for parameters for an instance
pub(crate) struct InstanceParameters<'a> {
	pub version: &'a VersionName,
	pub version_manifest: &'a VersionManifestAndList,
	pub paths: &'a Paths,
	pub req_client: &'a reqwest::Client,
	pub persistent: &'a mut PersistentData,
	pub update_manager: &'a mut UpdateManager,
	pub client_meta: &'a ClientMeta,
	pub users: &'a mut UserManager,
	pub client_assets_and_libs: &'a mut ClientAssetsAndLibraries,
}