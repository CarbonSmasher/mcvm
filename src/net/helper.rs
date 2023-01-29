use core::panic;
use std::{io::Write, string::FromUtf8Error};

use curl::easy::Easy;
pub enum DownloadMode {
	File(std::fs::File)
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
	#[error("Transfer failed: {}", .0)]
	Curl(#[from] curl::Error),
	#[error("Failed to open file: {}", .0)]
	File(#[from] std::io::Error),
	#[error("Failed to write data")]
	_Write,
	#[error("Failed to convert string to UTF-8")]
	StringConvert(#[from] FromUtf8Error)
}

pub struct Download {
	modes: Vec<DownloadMode>,
	string: Option<Vec<u8>>,
	pub easy: Easy
}

impl Download {
	pub fn new() -> Self {
		let easy = Easy::new();
		Download{modes: vec![], string: None, easy}
	}

	pub fn url(&mut self, url: &str) -> Result<(), DownloadError> {
		self.easy.url(url)?;
		Ok(())
	}

	pub fn add_file(&mut self, path: &std::path::Path) -> Result<(), DownloadError> {
		let file = std::fs::File::create(path)?;
		self.modes.push(DownloadMode::File(file));
		Ok(())
	}

	pub fn add_str(&mut self) {
		self.string = Some(Vec::new());
	}

	pub fn reset(&mut self) {
		self.modes.clear();
		self.string = None;
	}

	pub fn perform(&mut self) -> Result<(), DownloadError> {
		let mut transfer = self.easy.transfer();
		transfer.write_function(|data| {
			for mode in self.modes.iter_mut() {
				match mode {
					DownloadMode::File(file) => if file.write_all(data).is_err() {
						return Err(curl::easy::WriteError::Pause);
					}
				};
			}
			if let Some(string) = &mut self.string {
				string.extend_from_slice(data);
			}
			Ok(data.len())
		})?;
		transfer.perform()?;
		Ok(())
	}

	pub fn get_str(&mut self) -> Result<String, DownloadError> {
		match &mut self.string {
			Some(string) => {
				let out = String::from_utf8(string.to_vec())?;
				Ok(out)
			},
			None => panic!("String not set to write into")
		}
	}
}

// pub fn MultiDownload

// #[derive(Debug, thiserror::Error)]
// enum MultiDownloadError {
// 	#[error("When downloading: {}", .0)]
// 	Download(DownloadError),
// 	#[error("When performing multiple downloads: {}", .0)]
// 	Multi(#[from] curl::MultiError)
// }

// pub struct MultiDownload {
// 	handles: Vec<Box<Easy>>,
// 	multi: Multi
// }

// impl MultiDownload {
// 	pub fn new() -> Self {
// 		MultiDownload { handles: Vec::new(), multi: Multi::new() }
// 	}

// 	pub fn download(&mut self, easy: Box<Easy>) -> Result<(), MultiDownloadError> {
// 		self.multi.add(*easy);
// 		self.handles.push(easy);
// 		Ok(())
// 	}

// 	pub fn perform(&mut self) -> Result<(), MultiDownloadError> {
// 		let perform = self.multi.perform()?;
// 		Ok(())
// 	}
// }