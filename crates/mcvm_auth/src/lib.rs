#![warn(missing_docs)]
#![deny(unsafe_code)]

//! This library is used by MCVM to authenticate with Minecraft using Microsoft's APIs.
//! Although it provides the base functions for authentication, it does not string them
//! together for you. For an example of using this crate, look at the `user::auth` module in
//! the `mcvm_core` crate.

/// Authentication for Minecraft
pub mod mc;
/// Implementation of authentication with MSA for Minecraft auth
mod mc_msa;
