/// Generating arguments for the client
mod args;

use std::collections::HashMap;

use anyhow::Context;

use crate::data::instance::launch::LaunchProcessProperties;
use crate::data::instance::{InstKind, Instance};
use crate::data::user::UserManager;
use crate::io::files::paths::Paths;
use crate::skip_none;
use crate::util::json;
use mcvm_shared::versions::VersionInfo;
use mcvm_shared::versions::VersionPattern;

pub use args::create_quick_play_args;

impl Instance {
	/// Launch a client
	pub fn launch_client(
		&mut self,
		paths: &Paths,
		users: &UserManager,
		debug: bool,
		version_info: &VersionInfo,
	) -> anyhow::Result<()> {
		debug_assert!(matches!(self.kind, InstKind::Client { .. }));
		let java_path = self.java.get().path.get();
		let jre_path = java_path.join("bin/java");
		let client_dir = self.get_subdir(paths);
		let mut jvm_args = Vec::new();
		let mut game_args = Vec::new();
		let client_json = self.client_json.get();
		if let Some(classpath) = &self.classpath {
			let main_class = self
				.main_class
				.as_ref()
				.expect("Main class for client should exist");
			if let InstKind::Client { options: _, window } = &self.kind {
				if let Ok(args) = json::access_object(client_json, "arguments") {
					for arg in json::access_array(args, "jvm")? {
						for sub_arg in args::process_arg(
							self,
							arg,
							paths,
							users,
							classpath,
							&version_info.version,
							window,
						) {
							jvm_args.push(sub_arg);
						}
					}

					for arg in json::access_array(args, "game")? {
						for sub_arg in args::process_arg(
							self,
							arg,
							paths,
							users,
							classpath,
							&version_info.version,
							window,
						) {
							game_args.push(sub_arg);
						}
					}
				} else {
					// Behavior for versions prior to 1.12.2
					let args = json::access_str(client_json, "minecraftArguments")?;

					jvm_args.push(format!(
						"-Djava.library.path={}",
						paths
							.internal
							.join("versions")
							.join(&version_info.version)
							.join("natives")
							.to_str()
							.context("Failed to convert natives directory to a string")?
					));
					jvm_args.push(String::from("-cp"));
					jvm_args.push(classpath.get_str());

					for arg in args.split(' ') {
						game_args.push(skip_none!(args::replace_arg_placeholders(
							self,
							arg,
							paths,
							users,
							classpath,
							&version_info.version,
							window,
						)));
					}
				}
			}

			let mut env_vars = HashMap::new();
			// Compatability env var for old versions on Linux to prevent graphical issues
			#[cfg(target_os = "linux")]
			{
				if VersionPattern::from("1.8.9-").matches_info(version_info) {
					env_vars.insert("__GL_THREADED_OPTIMIZATIONS".to_string(), "0".to_string());
				}
			}

			let launch_properties = LaunchProcessProperties {
				cwd: &client_dir,
				command: jre_path
					.to_str()
					.context("Failed to convert java path to a string")?,
				jvm_args: &jvm_args,
				main_class: Some(main_class),
				game_args: &game_args,
				additional_env_vars: &env_vars,
			};

			self.launch_game_process(launch_properties, debug, version_info, paths)
				.context("Failed to launch game process")?;
		}

		Ok(())
	}
}