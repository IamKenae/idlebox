use crate::core::Applet;
use std::io::{self, Write};

pub struct WhoamiApplet;

impl Applet for WhoamiApplet {
    fn name(&self) -> &'static str {
        "whoami"
    }

    fn description(&self) -> &'static str {
        "Print effective user name"
    }

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

    fn help(&self) {
        println!("Usage: {}", self.name());
        println!();
        println!("{}", self.description());
        println!();
        println!("Print the effective user name (same as `id -un`).");
    }
}

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

fn c_char_to_string(ptr: *const i8) -> String {
    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr as *const u8, len);
        String::from_utf8_lossy(slice).to_string()
    }
}

#[repr(C)]
struct Passwd {
    pw_name: *const i8,
    pw_passwd: *const i8,
    pw_uid: u32,
    pw_gid: u32,
    pw_gecos: *const i8,
    pw_dir: *const i8,
    pw_shell: *const i8,
}

extern "C" {
    #[link_name = "geteuid"]
    fn raw_geteuid() -> u32;

    #[link_name = "getpwuid"]
    fn raw_getpwuid(uid: u32) -> *const Passwd;
}
