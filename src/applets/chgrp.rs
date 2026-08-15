#[cfg(unix)]
use crate::core::unix_ffi::raw_getgrnam;
use crate::core::Applet;

#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::path::Path;

pub struct ChgrpApplet;

impl Applet for ChgrpApplet {
    fn name(&self) -> &'static str {
        "chgrp"
    }

    fn description(&self) -> &'static str {
        "Change group ownership"
    }

    #[cfg(unix)]
    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut recursive = false;
        let mut group_spec: Option<&str> = None;
        let mut paths: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-R" | "--recursive" => recursive = true,
                _ if args[i].starts_with('-') && args[i].len() > 1 && group_spec.is_none() => {
                    let mut combined = true;
                    for ch in args[i][1..].chars() {
                        if ch != 'R' {
                            combined = false;
                            break;
                        }
                    }
                    if combined {
                        recursive = true;
                    } else {
                        group_spec = Some(&args[i]);
                    }
                }
                _ if group_spec.is_none() => {
                    group_spec = Some(&args[i]);
                }
                _ => {
                    paths.push(&args[i]);
                }
            }
            i += 1;
        }

        let group_spec = match group_spec {
            Some(s) => s,
            None => {
                eprintln!("chgrp: missing operand");
                return Ok(1);
            }
        };

        if paths.is_empty() {
            eprintln!("chgrp: missing operand after '{}'", group_spec);
            return Ok(1);
        }

        let gid = match resolve_gid(group_spec) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("chgrp: {}", e);
                return Ok(1);
            }
        };

        let uid = (-1i32) as u32;

        let mut exit_code = 0;
        for path in &paths {
            if let Err(e) = apply_chgrp(path, uid, gid, recursive) {
                eprintln!("chgrp: cannot access '{}': {}", path, e);
                exit_code = 1;
            }
        }

        Ok(exit_code)
    }

    #[cfg(not(unix))]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        eprintln!("chgrp: not supported on this platform");
        Ok(1)
    }

    fn help(&self) {
        println!("Usage: chgrp [OPTION]... GROUP FILE...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -R, --recursive   Change files and directories recursively");
        println!();
        println!("GROUP may be a numeric GID or a group name.");
        #[cfg(not(unix))]
        println!();
        #[cfg(not(unix))]
        println!("Note: this applet is not supported on this platform.");
    }
}

#[cfg(unix)]
fn resolve_gid(s: &str) -> Result<u32, String> {
    if let Ok(n) = s.parse::<u32>() {
        return Ok(n);
    }
    let c_name = CString::new(s).map_err(|_| format!("invalid group name: '{}'", s))?;
    let ptr = unsafe { raw_getgrnam(c_name.as_ptr()) };
    if ptr.is_null() {
        return Err(format!("invalid group: '{}'", s));
    }
    unsafe { Ok((*ptr).gr_gid) }
}

#[cfg(unix)]
fn apply_chgrp(path: &str, uid: u32, gid: u32, recursive: bool) -> Result<(), std::io::Error> {
    let p = Path::new(path);
    let c_path =
        CString::new(path).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let metadata = p.symlink_metadata()?;
    let is_symlink = metadata.file_type().is_symlink();

    let ret = if is_symlink {
        unsafe { raw_lchown(c_path.as_ptr(), uid, gid) }
    } else {
        unsafe { raw_chown(c_path.as_ptr(), uid, gid) }
    };

    if ret != 0 {
        let err = std::io::Error::last_os_error();
        return Err(err);
    }

    if recursive && metadata.is_dir() {
        for entry in std::fs::read_dir(p)? {
            let entry = entry?;
            let entry_path = entry.path();
            let entry_str = entry_path.to_string_lossy().to_string();
            apply_chgrp(&entry_str, uid, gid, true)?;
        }
    }

    Ok(())
}

#[cfg(unix)]
extern "C" {
    #[link_name = "chown"]
    fn raw_chown(path: *const i8, owner: u32, group: u32) -> i32;

    #[link_name = "lchown"]
    fn raw_lchown(path: *const i8, owner: u32, group: u32) -> i32;
}
