use serde::{Deserialize, Serialize};

use crate::data::user::{User, UserKind};

#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum UserVariant {
	Microsoft { uuid: Option<String> },
	Demo { uuid: Option<String> },
	Unverified {},
}

impl UserVariant {
	fn to_user_kind(&self) -> UserKind {
		match self {
			Self::Microsoft { .. } => UserKind::Microsoft,
			Self::Demo { .. } => UserKind::Demo,
			Self::Unverified {} => UserKind::Unverified,
		}
	}
}

#[derive(Deserialize, Serialize, Clone)]
/// Configuration for a user
pub struct UserConfig {
	pub name: String,
	#[serde(flatten)]
	pub variant: UserVariant,
}

impl UserConfig {
	/// Creates a user from this user config
	pub fn to_user(&self, id: &str) -> User {
		let mut user = User::new(self.variant.to_user_kind(), id, &self.name);
		match &self.variant {
			UserVariant::Microsoft { uuid } | UserVariant::Demo { uuid } => {
				if let Some(uuid) = uuid {
					user.set_uuid(uuid);
				}
			}
			_ => {}
		}

		user
	}
}
