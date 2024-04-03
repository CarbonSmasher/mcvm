use std::time::Duration;

use anyhow::{anyhow, Context};
use clap::Subcommand;
use color_print::cprintln;
use inquire::Select;
use itertools::Itertools;
use mcvm::data::config::Config;
use mcvm::data::id::{InstanceRef, ProfileID};

use mcvm::shared::Side;
use reqwest::Client;

use super::CmdData;
use crate::output::HYPHEN_POINT;
use crate::secrets::get_ms_client_id;

#[derive(Debug, Subcommand)]
pub enum InstanceSubcommand {
	#[command(about = "List all instances in all profiles")]
	#[clap(alias = "ls")]
	List {
		/// Whether to remove formatting and warnings from the output
		#[arg(short, long)]
		raw: bool,
		/// Filter by instance side
		#[arg(short, long)]
		side: Option<Side>,
		/// Filter by profile
		#[arg(short, long)]
		profile: Option<String>,
	},
	#[command(about = "Launch instances to play the game")]
	Launch {
		/// An optional user to choose when launching
		#[arg(short, long)]
		user: Option<String>,
		/// The instance to launch, as an instance reference (profile:instance)
		instance: Option<String>,
	},
}

pub async fn run(command: InstanceSubcommand, data: &mut CmdData) -> anyhow::Result<()> {
	match command {
		InstanceSubcommand::List { raw, side, profile } => list(data, raw, side, profile).await,
		InstanceSubcommand::Launch { user, instance } => launch(instance, user, data).await,
	}
}

async fn list(
	data: &mut CmdData,
	raw: bool,
	side: Option<Side>,
	profile: Option<String>,
) -> anyhow::Result<()> {
	data.ensure_config(!raw).await?;
	let config = data.config.get_mut();

	let profile = if let Some(profile) = profile {
		let profile = ProfileID::from(profile);
		Some(
			config
				.profiles
				.get(&profile)
				.ok_or(anyhow!("Profile '{profile}' does not exist"))?,
		)
	} else {
		None
	};

	for (id, instance) in config.instances.iter().sorted_by_key(|x| x.0) {
		if let Some(side) = side {
			if instance.get_side() != side {
				continue;
			}
		}

		if let Some(profile) = profile {
			if !profile.instances.contains(&id.instance) {
				continue;
			}
		}

		if raw {
			println!("{id}");
		} else {
			match instance.get_side() {
				Side::Client => cprintln!("{}<y!>{}", HYPHEN_POINT, id),
				Side::Server => cprintln!("{}<c!>{}", HYPHEN_POINT, id),
			}
		}
	}

	Ok(())
}

pub async fn launch(
	instance: Option<String>,
	user: Option<String>,
	data: &mut CmdData,
) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get_mut();

	let instance = pick_instance(instance, config).context("Failed to pick instance")?;

	let instance = config
		.instances
		.get_mut(&instance)
		.ok_or(anyhow!("Unknown instance '{instance}'"))?;
	let (.., profile) = config
		.profiles
		.iter_mut()
		.find(|(.., profile)| profile.instances.contains(instance.get_id()))
		.expect("Instance does not belong to any profiles");

	if let Some(user) = user {
		config
			.users
			.choose_user(&user)
			.context("Failed to choose user")?;
	}

	// Launch the proxy first
	let proxy_handle = if let Side::Server = instance.get_side() {
		let client = Client::new();
		profile
			.launch_proxy(&client, &data.paths, &mut data.output)
			.await
			.context("Failed to launch profile proxy")?
	} else {
		None
	};

	let mut instance_handle = instance
		.launch(
			&data.paths,
			&mut config.users,
			&profile.version,
			get_ms_client_id(),
			&mut data.output,
		)
		.await
		.context("Instance failed to launch")?;

	// Await both asynchronously if the proxy is present
	if let Some(mut proxy_handle) = proxy_handle {
		let proxy = async move {
			proxy_handle
				.wait()
				.context("Failed to wait for proxy child process")?;

			Ok::<(), anyhow::Error>(())
		};

		let instance = async move {
			// Wait for the proxy to start up
			tokio::time::sleep(Duration::from_secs(5)).await;
			instance_handle
				.wait()
				.context("Failed to wait for instance child process")?;

			Ok::<(), anyhow::Error>(())
		};

		tokio::try_join!(proxy, instance).context("Failed to launch proxy and instance")?;
	} else {
		// Otherwise, just wait for the instance
		instance_handle
			.wait()
			.context("Failed to wait for instance child process")?;
	}

	Ok(())
}

/// Pick which instance to launch
fn pick_instance(instance: Option<String>, config: &Config) -> anyhow::Result<InstanceRef> {
	if let Some(instance) = instance {
		InstanceRef::parse(instance).context("Failed to parse instance reference")
	} else {
		let options: Vec<InstanceRef> = config.instances.keys().cloned().collect();
		let selection = Select::new("Choose an instance to launch", options)
			.prompt()
			.context("Prompt failed")?;

		Ok(selection)
	}
}
