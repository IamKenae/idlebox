mod applet;
mod dispatcher;
pub(crate) mod file_ops;
pub mod install;
mod size_format;
#[cfg(unix)]
pub(crate) mod unix_ffi;

pub use applet::Applet;
pub use dispatcher::Dispatcher;
pub(crate) use size_format::human_size;
#[cfg(any(target_os = "linux", windows))]
pub(crate) use size_format::rounded_percentage;

pub fn banner() -> &'static str {
    concat!(
        "IdleBox v",
        env!("CARGO_PKG_VERSION"),
        " - A lightweight multi-call toolbox"
    )
}
