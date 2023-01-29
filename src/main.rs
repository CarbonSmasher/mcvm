mod commands;
mod data;
mod io;
mod net;
mod lib;
mod package;
mod user;

use std::env;

use commands::run_command;
use commands::help;
use io::files::Paths;

fn main() {
	let argv: Vec<String> = env::args().collect();
	let argc: usize = argv.len();
	match argc {
		0 => debug_assert!(false),
		1 => help::help(),
		_ => {
			let paths = Paths::new();
			let mut config = data::config::Config::new(&paths.config.join("mcvm.json"));
			match config.load() {
				Ok(..) => {},
				Err(err) => color_print::cprintln!("<r>{}", err)
			}
			// let version = lib::versions::MinecraftVersion::from("1.5");
			// let (doc, _) = match net::game_files::get_version_json(
			// 	&version, &paths, true
			// ) {
			// 	Ok(val) => val,
			// 	Err(err) => panic!("{}", err)
			// };
			// if let Err(err) = net::game_files::get_libraries(&doc, &paths, &version, true, false) {
			// 	eprintln!("{err}");
			// }
			// if let Err(err) = net::game_files::get_assets(&doc, &paths, &version, true, false) {
			// 	eprintln!("{err}");
			// }
			// let mut reg = data::profile::InstanceRegistry::new();
			// let mut client = reg.insert(
			// 	"client".to_string(),
			// 	Box::new(data::profile::Client::new("client"))
			// ).unwrap() as Box<data::profile::Client>;
			// let mut prof = data::profile::Profile::new("main", &version);
			// prof.add_instance(client.name);

			let argv_slice = &argv[2..];
			let argc_slice = argc - 2;
			run_command(&argv[1], argc_slice, argv_slice, &paths);
		}
	}
}