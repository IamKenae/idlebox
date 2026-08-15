mod applet;
mod dispatcher;
pub mod install;
#[cfg(unix)]
pub(crate) mod unix_ffi;

pub use applet::Applet;
pub use dispatcher::Dispatcher;
