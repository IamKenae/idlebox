mod applet;
mod dispatcher;
pub(crate) mod file_ops;
pub mod install;
mod size_format;
#[cfg(unix)]
pub(crate) mod unix_ffi;

pub use applet::Applet;
pub use dispatcher::Dispatcher;
pub(crate) use size_format::{human_size, rounded_percentage};
