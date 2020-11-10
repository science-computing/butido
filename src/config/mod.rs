//! The configuration handling code
//!
//! This module contains all code for the configuration of butido itself.
//!
//! Please note that the `not_validated` module is the "entry point".
//! A "NotValidatedConfiguration" is loaded from the filesystem and then transformed into a
//! `Configuration` object via the `validate()` method.
//!
//! This mechanism is chosen because we might want to be able to do validation on the configuration
//! that is not possible to do with TOML itself.
//!

mod configuration;
pub use configuration::*;

mod container_config;
pub use container_config::*;

mod docker_config;
pub use docker_config::*;

mod endpoint_config;
pub use endpoint_config::*;

mod not_validated;
pub use not_validated::*;

mod util;
