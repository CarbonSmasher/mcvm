pub mod json;
pub mod mojang;
pub mod print;
pub mod versions;

/// Converts "yes" or "no" to a boolean
pub fn yes_no(string: &str) -> Option<bool> {
	match string {
		"yes" => Some(true),
		"no" => Some(false),
		_ => None,
	}
}

/// Skip in a loop if a result fails
#[macro_export]
macro_rules! skip_fail {
	($res:expr) => {
		match $res {
			Ok(val) => val,
			Err(..) => continue,
		}
	};
}

/// Skip in a loop if an option is none
#[macro_export]
macro_rules! skip_none {
	($res:expr) => {
		match $res {
			Some(val) => val,
			None => continue,
		}
	};
}

/// Validates a simple string identifier
pub fn validate_identifier(id: &str) -> bool {
	for c in id.chars() {
		if !c.is_ascii() {
			return false;
		}

		if c.is_ascii_punctuation() {
			match c {
				'_' | '-' | '.' => {}
				_ => return false,
			}
		}

		if c.is_ascii_whitespace() {
			return false;
		}
	}

	true
}

/// Capitalizes the first character of a string
pub fn cap_first_letter(string: &str) -> String {
	let mut c = string.chars();
	match c.next() {
		None => String::new(),
		Some(f) => f.to_uppercase().chain(c).collect(),
	}
}

pub trait ToInt {
	fn to_int(&self) -> i32;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_id_validation() {
		assert!(validate_identifier("hello"));
		assert!(validate_identifier("Hello"));
		assert!(validate_identifier("H3110"));
		assert!(validate_identifier("hello-world"));
		assert!(validate_identifier("hello_world"));
		assert!(validate_identifier("hello.world"));
		assert!(!validate_identifier("hello*world"));
		assert!(!validate_identifier("hello\nworld"));
		assert!(!validate_identifier("hello world"));
	}
}
