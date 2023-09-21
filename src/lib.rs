#![warn(missing_docs)]

//! This is the library for MCVM and pretty much all of the features that the
//! CLI uses.
//!
//! # Features
//!
//! - `arc`: MCVM uses Rc's in a couple places. Although these are more performant than Arc's, they
//! may not be compatible with some async runtimes. With this feature enabled, these Rc's will be replaced with
//! Arc's where possible.

pub use mcvm_parse as parse;
pub use mcvm_pkg as pkg_crate;
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

/// The global struct used as an Rc, depending on the `arc` feature
#[cfg(feature = "arc")]
pub type RcType<T> = std::sync::Arc<T>;
/// The global struct used as an Rc, depending on the `arc` feature
#[cfg(not(feature = "arc"))]
pub type RcType<T> = std::rc::Rc<T>;
