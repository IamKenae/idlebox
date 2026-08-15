mod applet;
mod dispatcher;
pub(crate) mod file_ops;
pub mod install;
#[cfg(unix)]
pub(crate) mod unix_ffi;

pub use applet::Applet;
pub use dispatcher::Dispatcher;
