//! Shared declarations for Unix account database functions.
//!
//! Keeping these declarations in one module guarantees that every applet uses
//! the same ABI-compatible Rust type for each C function.

#[cfg(any(target_os = "linux", target_os = "android"))]
#[repr(C)]
pub(crate) struct Passwd {
    pub(crate) pw_name: *const i8,
    pub(crate) pw_passwd: *const i8,
    pub(crate) pw_uid: u32,
    pub(crate) pw_gid: u32,
    pub(crate) pw_gecos: *const i8,
    pub(crate) pw_dir: *const i8,
    pub(crate) pw_shell: *const i8,
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
#[repr(C)]
pub(crate) struct Passwd {
    pub(crate) pw_name: *const i8,
    pub(crate) pw_passwd: *const i8,
    pub(crate) pw_uid: u32,
    pub(crate) pw_gid: u32,
    pub(crate) pw_change: i64,
    pub(crate) pw_class: *const i8,
    pub(crate) pw_gecos: *const i8,
    pub(crate) pw_dir: *const i8,
    pub(crate) pw_shell: *const i8,
    pub(crate) pw_expire: i64,
    pub(crate) pw_fields: i32,
}

#[repr(C)]
pub(crate) struct Group {
    pub(crate) gr_name: *const i8,
    pub(crate) gr_passwd: *const i8,
    pub(crate) gr_gid: u32,
    pub(crate) gr_mem: *const *const i8,
}

extern "C" {
    #[link_name = "getpwuid"]
    pub(crate) fn raw_getpwuid(uid: u32) -> *const Passwd;

    #[link_name = "getpwnam"]
    pub(crate) fn raw_getpwnam(name: *const i8) -> *const Passwd;

    #[link_name = "getgrgid"]
    pub(crate) fn raw_getgrgid(gid: u32) -> *const Group;

    #[link_name = "getgrnam"]
    pub(crate) fn raw_getgrnam(name: *const i8) -> *const Group;
}
