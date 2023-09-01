#![warn(missing_docs)]

//! This is the library for MCVM and pretty much all of the features that the
//! CLI uses.

pub use mcvm_parse as parse;
pub use mcvm_pkg as pkg;
pub use mcvm_shared as shared;

/// Dealing with MCVM's data constructs, like instances and profiles
pub mod data;
/// File and data format input / output
pub mod io;
/// API wrappers and networking utilities
pub mod net;
/// Dealing with packages
pub mod package;
/// Common utilities that can't live anywhere else
pub mod util;
