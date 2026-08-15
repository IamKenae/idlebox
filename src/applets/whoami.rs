#[cfg(unix)]
use crate::core::unix_ffi::raw_getpwuid;
use crate::core::Applet;
#[cfg(unix)]
use std::ffi::{c_char, CStr};
use std::io::{self, Write};

pub struct WhoamiApplet;

impl Applet for WhoamiApplet {
    fn name(&self) -> &'static str {
        "whoami"
    }

    fn description(&self) -> &'static str {
        "Print effective user name"
    }

    #[cfg(unix)]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let uid = unsafe { raw_geteuid() };
        match get_username_by_uid(uid) {
            Some(name) => {
                let stdout = io::stdout();
                let mut out = stdout.lock();
                writeln!(out, "{}", name)?;
                Ok(0)
            }
            None => {
                eprintln!("whoami: cannot find name for user ID {}", uid);
                Ok(1)
            }
        }
    }

    #[cfg(windows)]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let username = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "unknown".to_string());
        let stdout = io::stdout();
        let mut out = stdout.lock();
        writeln!(out, "{}", username)?;
        Ok(0)
    }

    #[cfg(not(any(unix, windows)))]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        eprintln!("whoami: not supported on this platform");
        Ok(1)
    }

    fn help(&self) {
        println!("Usage: {}", self.name());
        println!();
        println!("{}", self.description());
        println!();
        println!("Print the effective user name (same as `id -un`).");
    }
}

#[cfg(unix)]
fn get_username_by_uid(uid: u32) -> Option<String> {
    let ptr = unsafe { raw_getpwuid(uid) };
    if ptr.is_null() {
        return None;
    }
    unsafe {
        let pw_name = (*ptr).pw_name;
        if pw_name.is_null() {
            return None;
        }
        Some(c_char_to_string(pw_name))
    }
}

#[cfg(unix)]
fn c_char_to_string(ptr: *const c_char) -> String {
    unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}

#[cfg(unix)]
extern "C" {
    #[link_name = "geteuid"]
    fn raw_geteuid() -> u32;
}
