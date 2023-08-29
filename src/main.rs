mod cli;

use std::process::ExitCode;

use color_print::cformat;
use cli::commands::run_cli;

#[tokio::main]
async fn main() -> ExitCode {
	let mut data = cli::commands::CmdData::new();
	match run_cli(&mut data).await {
		Ok(()) => {}
		Err(e) => {
			eprintln!("{}", cformat!("<r>{:?}", e));
			return ExitCode::FAILURE;
		}
	}

	ExitCode::SUCCESS
}
