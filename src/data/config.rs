use super::user::{User, UserKind, AuthState, Auth};
use super::profile::{Profile, InstanceRegistry};
use super::instance::{Instance, InstKind};
use crate::util::{json, versions::MinecraftVersion};

use color_print::cprintln;
use serde_json::json;

use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;

#[derive(Debug)]
pub struct Config {
	pub auth: Auth,
	pub instances: InstanceRegistry,
	pub profiles: HashMap<String, Box<Profile>>
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
	#[error("{}", .0)]
	File(#[from] std::io::Error),
	#[error("Failed to parse json:\n{}", .0)]
	Json(#[from] json::JsonError),
	#[error("Json operation failed:\n{}", .0)]
	SerdeJson(#[from] serde_json::Error),
	#[error("Invalid config content:\n{}", .0)]
	Content(#[from] ContentError)
}

#[derive(Debug, thiserror::Error)]
pub enum ContentError {
	#[error("Unknown type {} for user {}", .0, .1)]
	UserType(String, String),
	#[error("Unknown type {} for instance {}", .0, .1)]
	InstType(String, String),
	#[error("Unknown default user '{}'", .0)]
	DefaultUserNotFound(String),
	#[error("Duplicate instance '{}'", .0)]
	DuplicateInstance(String)
}

impl Config {
	pub fn new() -> Self {
		Self {
			auth: Auth::new(),
			instances: InstanceRegistry::new(),
			profiles: HashMap::new()
		}
	}

	fn open(path: &PathBuf) -> Result<Box<json::JsonObject>, ConfigError> {
		if path.exists() {
			let doc = json::parse_object(&fs::read_to_string(path)?)?;
			Ok(doc)
		} else {
			let doc = json!(
				{
					"users": {},
					"profiles": {}
				}
			);
			fs::write(path, serde_json::to_string_pretty(&doc)?)?;
			Ok(Box::new(json::ensure_type(doc.as_object(), json::JsonType::Object)?.clone()))
		}
	}

	pub fn load(path: &PathBuf) -> Result<Self, ConfigError> {
		let mut config = Self::new();
		let doc = Self::open(path)?;

		// Users
		let users = json::access_object(&doc, "users")?;
		for (user_id, user_val) in users.iter() {
			let user_obj = json::ensure_type(user_val.as_object(), json::JsonType::Object)?;
			let kind = match json::access_str(user_obj, "type")? {
				"microsoft" => Ok(UserKind::Microsoft),
				"demo" => Ok(UserKind::Demo),
				typ => Err(ContentError::UserType(typ.to_string(), user_id.to_string()))
			}?;
			let mut user = User::new(kind, user_id, json::access_str(user_obj, "name")?);

			match user_obj.get("uuid") {
				Some(uuid) => user.set_uuid(json::ensure_type(uuid.as_str(), json::JsonType::Str)?),
				None => cprintln!("<y>Warning: It is recommended to have your uuid in the configuration for user {}", user_id)
			};
			
			config.auth.users.insert(user_id.to_string(), user);
		}

		if let Some(user_val) = doc.get("default_user") {
			let user_id = json::ensure_type(user_val.as_str(), json::JsonType::Str)?.to_string();
			match config.auth.users.get(&user_id) {
				Some(..) => config.auth.state = AuthState::Authed(user_id),
				None => return Err(ConfigError::from(ContentError::DefaultUserNotFound(user_id)))
			}
		} else if users.is_empty() {
			cprintln!("<y>Warning: Users are available but no default user is set. Starting in offline mode");
		} else {
			cprintln!("<y>Warning: No users are available. Starting in offline mode");
		}

		// Profiles
		let profiles = json::access_object(&doc, "profiles")?;
		for (profile_id, profile_val) in profiles.iter() {
			let profile_obj = json::ensure_type(profile_val.as_object(), json::JsonType::Object)?;
			let version =  MinecraftVersion::from(json::access_str(profile_obj, "version")?);

			let mut profile = Profile::new(profile_id, &version);
			
			// Instances
			if let Some(instances_val) = profile_obj.get("instances") {
				let instances = json::ensure_type(instances_val.as_object(), json::JsonType::Object)?;
				for (instance_id, instance_val) in instances.iter() {
					if config.instances.contains_key(instance_id) {
						return Err(ConfigError::from(ContentError::DuplicateInstance(instance_id.to_string())));
					}

					let instance_obj = json::ensure_type(instance_val.as_object(), json::JsonType::Object)?;
					let kind = match json::access_str(instance_obj, "type")? {
						"client" => {
							Ok(InstKind::Client)
						},
						"server" => {
							Ok(InstKind::Server)
						},
						typ => Err(ContentError::InstType(typ.to_string(), instance_id.to_string()))
					}?;

					let instance = Instance::new(kind, instance_id, &version);
					profile.add_instance(instance_id);
					config.instances.insert(instance_id.to_string(), instance);
				}
			}

			// TODO: Packages
			
			config.profiles.insert(profile_id.to_string(), Box::new(profile));
		}

		Ok(config)
	}
}
