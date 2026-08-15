//! Shared declarations for Unix account database functions.
//!
//! Keeping these declarations in one module guarantees that every applet uses
//! the same ABI-compatible Rust type for each C function.

use std::ffi::c_char;

#[cfg(any(target_os = "linux", target_os = "android"))]
#[repr(C)]
pub(crate) struct Passwd {
    pub(crate) pw_name: *const c_char,
    pub(crate) pw_passwd: *const c_char,
    pub(crate) pw_uid: u32,
    pub(crate) pw_gid: u32,
    pub(crate) pw_gecos: *const c_char,
    pub(crate) pw_dir: *const c_char,
    pub(crate) pw_shell: *const c_char,
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
#[repr(C)]
pub(crate) struct Passwd {
    pub(crate) pw_name: *const c_char,
    pub(crate) pw_passwd: *const c_char,
    pub(crate) pw_uid: u32,
    pub(crate) pw_gid: u32,
    pub(crate) pw_change: i64,
    pub(crate) pw_class: *const c_char,
    pub(crate) pw_gecos: *const c_char,
    pub(crate) pw_dir: *const c_char,
    pub(crate) pw_shell: *const c_char,
    pub(crate) pw_expire: i64,
    pub(crate) pw_fields: i32,
}

#[repr(C)]
pub(crate) struct Group {
    pub(crate) gr_name: *const c_char,
    pub(crate) gr_passwd: *const c_char,
    pub(crate) gr_gid: u32,
    pub(crate) gr_mem: *const *const c_char,
}

extern "C" {
    #[link_name = "getpwuid"]
    pub(crate) fn raw_getpwuid(uid: u32) -> *const Passwd;

    #[link_name = "getpwnam"]
    pub(crate) fn raw_getpwnam(name: *const c_char) -> *const Passwd;

    #[link_name = "getgrgid"]
    pub(crate) fn raw_getgrgid(gid: u32) -> *const Group;

    #[link_name = "getgrnam"]
    pub(crate) fn raw_getgrnam(name: *const c_char) -> *const Group;
}
