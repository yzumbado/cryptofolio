// Allow certain clippy lints that would require significant refactoring
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::should_implement_trait)] // for from_str methods
#![allow(clippy::format_in_format_args)]
#![allow(clippy::manual_c_str_literals)]

pub mod ai;
pub mod blockchain;
pub mod cli;
pub mod config;
pub mod core;
pub mod db;
pub mod error;
pub mod exchange;
pub mod shell;
