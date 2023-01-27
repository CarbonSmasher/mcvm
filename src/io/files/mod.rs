pub mod files;

use files::create_existing_dir;

use std::path::PathBuf;
use std::env::var;

pub struct Paths {
	pub home: PathBuf,
	pub data: PathBuf,
	pub internal: PathBuf,
	pub assets: PathBuf,
	pub java: PathBuf,
	pub cache: PathBuf,
	pub config: PathBuf,
	pub run: PathBuf,
}

impl Paths {
	pub fn new() -> Paths {
		// TODO: Non-Linux paths
		let home: PathBuf = match var("XDG_HOME") {
			Ok(path) => PathBuf::from(&path),
			Err(_) => PathBuf::from(&var("HOME").unwrap())
		};

		let data: PathBuf = match var("XDG_DATA_HOME") {
			Ok(path) => PathBuf::from(&path).join("mcvm"),
			Err(_) => home.join(".local/share/mcvm")
		};
		
		let internal: PathBuf = data.join("internal");
		let assets: PathBuf = internal.join("assets");
		let java: PathBuf = internal.join("java");
		
		let cache: PathBuf = match var("XDG_CACHE_HOME") {
			Ok(path) => PathBuf::from(&path).join("mcvm"),
			Err(_) => home.join(".config/mcvm")
		};
		
		let config: PathBuf = match var("XDG_CACHE_HOME") {
			Ok(path) => PathBuf::from(&path).join("mcvm"),
			Err(_) => home.join(".cache/mcvm")
		};
		
		let run: PathBuf = match var("XDG_CACHE_HOME") {
			Ok(path) => PathBuf::from(&path).join("mcvm"),
			Err(_) => match var("UID") {
				Ok(uid) => home.join("/run/user").join(uid),
				Err(_) => internal.join("run")
			}
		};
		
		create_existing_dir(&data).unwrap();
		create_existing_dir(&internal).unwrap();
		create_existing_dir(&assets).unwrap();
		create_existing_dir(&java).unwrap();
		create_existing_dir(&cache).unwrap();
		create_existing_dir(&config).unwrap();
		create_existing_dir(&run).unwrap();
		
		Paths {
			home: home.into(),
			data: data.into(),
			internal: internal.into(),
			assets: assets.into(),
			java: java.into(),
			cache: cache.into(),
			config: config.into(),
			run: run.into(),
		}
	}
}
