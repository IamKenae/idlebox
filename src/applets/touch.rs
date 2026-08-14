use crate::core::Applet;
use std::fs::File;
use std::path::Path;
use std::time::SystemTime;

pub struct TouchApplet;

impl Applet for TouchApplet {
    fn name(&self) -> &'static str {
        "touch"
    }

    fn description(&self) -> &'static str {
        "Update file timestamps or create empty files"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut files: Vec<&str> = Vec::new();

        for arg in args {
            match arg.as_str() {
                "--" => {}
                _ if arg.starts_with('-') && arg.len() > 1 => {
                    return Err(format!("touch: invalid option -- '{}'", &arg[1..]).into());
                }
                _ => files.push(arg),
            }
        }

        if files.is_empty() {
            eprintln!("touch: missing file operand");
            return Ok(1);
        }

        let now = SystemTime::now();
        let mut had_error = false;

        for file in &files {
            let path = Path::new(file);

            if path.exists() {
                if let Err(e) = Self::update_timestamps(path, now) {
                    eprintln!("touch: failed to update timestamps for '{}': {}", file, e);
                    had_error = true;
                }
            } else {
                if let Err(e) = File::create(path) {
                    eprintln!("touch: cannot touch '{}': {}", file, e);
                    had_error = true;
                }
            }
        }

        if had_error { Ok(1) } else { Ok(0) }
    }

    fn help(&self) {
        println!("Usage: touch [OPTION]... FILE...");
        println!();
        println!("{}", self.description());
        println!();
        println!("If a FILE does not exist, it is created as an empty file.");
        println!("If a FILE exists, its access and modification times are updated to now.");
    }
}

impl TouchApplet {
    #[cfg(unix)]
    fn update_timestamps(path: &Path, time: SystemTime) -> std::io::Result<()> {
        unsafe {
            let tv = Self::system_time_to_timeval(time);
            let times = [tv, tv];
            let ret = utimes(Self::path_to_cstr(path).as_ptr(), times.as_ptr());
            if ret != 0 {
                return Err(std::io::Error::last_os_error());
            }
        }

        Ok(())
    }

    #[cfg(unix)]
    fn system_time_to_timeval(time: SystemTime) -> Timeval {
        let duration = time.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
        Timeval {
            tv_sec: duration.as_secs() as i64,
            tv_usec: duration.subsec_micros() as i64,
        }
    }

    #[cfg(unix)]
    fn path_to_cstr(path: &Path) -> std::ffi::CString {
        use std::os::unix::ffi::OsStrExt;
        std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap()
    }

    #[cfg(not(unix))]
    fn update_timestamps(_path: &Path, _time: SystemTime) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(unix)]
#[derive(Clone, Copy)]
#[repr(C)]
struct Timeval {
    tv_sec: i64,
    tv_usec: i64,
}

#[cfg(unix)]
extern "C" {
    fn utimes(filename: *const std::ffi::c_char, times: *const Timeval) -> std::ffi::c_int;
}
