#[cfg(unix)]
use crate::core::unix_ffi::{lock_account_db, raw_getgrnam, raw_getpwnam, raw_getpwuid};
use crate::core::{banner, Applet};

#[cfg(unix)]
use std::ffi::{c_char, CString};
#[cfg(unix)]
use std::path::Path;

pub struct ChownApplet;

impl Applet for ChownApplet {
    fn name(&self) -> &'static str {
        "chown"
    }

    fn description(&self) -> &'static str {
        "Change file owner and group"
    }

    #[cfg(unix)]
    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut recursive = false;
        let mut owner_spec: Option<&str> = None;
        let mut paths: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-R" | "--recursive" => recursive = true,
                _ if args[i].starts_with('-') && args[i].len() > 1 && owner_spec.is_none() => {
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
                        owner_spec = Some(&args[i]);
                    }
                }
                _ if owner_spec.is_none() => {
                    owner_spec = Some(&args[i]);
                }
                _ => {
                    paths.push(&args[i]);
                }
            }
            i += 1;
        }

        let owner_spec = match owner_spec {
            Some(s) => s,
            None => {
                self.print_usage();
                return Ok(1);
            }
        };

        if paths.is_empty() {
            self.print_usage();
            return Ok(1);
        }

        let (uid, gid) = match parse_owner_spec(owner_spec) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("chown: {}", e);
                return Ok(1);
            }
        };

        let mut exit_code = 0;
        for path in &paths {
            if let Err(e) = apply_chown(path, uid, gid, recursive) {
                eprintln!("chown: cannot access '{}': {}", path, e);
                exit_code = 1;
            }
        }

        Ok(exit_code)
    }

    #[cfg(not(unix))]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        eprintln!("chown: not supported on this platform");
        Ok(1)
    }

    fn help(&self) {
        println!("{}", banner());
        println!();
        println!("Usage: chown [OPTION]... [OWNER][:[GROUP]] FILE...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -R, --recursive   Change files and directories recursively");
        println!();
        println!("OWNER and GROUP may be numeric IDs or names.");
        println!("Examples: user, user:group, :group, 1000:1000");
        #[cfg(not(unix))]
        println!();
        #[cfg(not(unix))]
        println!("Note: this applet is not supported on this platform.");
    }
}

#[cfg(unix)]
impl ChownApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: chown [OPTION]... [OWNER][:[GROUP]] FILE...");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -R, --recursive   Change files and directories recursively");
        eprintln!();
        eprintln!("OWNER and GROUP may be numeric IDs or names.");
        eprintln!("Examples: user, user:group, :group, 1000:1000");
    }
}

#[cfg(unix)]
fn parse_owner_spec(spec: &str) -> Result<(u32, u32), String> {
    if spec.is_empty() {
        return Err("invalid owner spec".to_string());
    }

    if let Some(colon_pos) = spec.find(':') {
        let user_part = &spec[..colon_pos];
        let group_part = &spec[colon_pos + 1..];

        if user_part.is_empty() && group_part.is_empty() {
            return Err("invalid owner spec".to_string());
        }

        let (resolved_uid, primary_gid) = if user_part.is_empty() {
            (u32::MAX, None)
        } else {
            let (uid, gid) = resolve_user(user_part, group_part.is_empty())?;
            (uid, gid)
        };

        let resolved_gid = if group_part.is_empty() {
            primary_gid.ok_or_else(|| "missing owner before ':'".to_string())?
        } else {
            resolve_gid(group_part)?
        };

        Ok((resolved_uid, resolved_gid))
    } else {
        let (resolved_uid, _) = resolve_user(spec, false)?;
        Ok((resolved_uid, u32::MAX))
    }
}

#[cfg(unix)]
fn resolve_user(s: &str, need_primary_gid: bool) -> Result<(u32, Option<u32>), String> {
    if let Ok(n) = s.parse::<u32>() {
        if !need_primary_gid {
            return Ok((n, None));
        }
        let _account_db_guard = lock_account_db();
        let ptr = unsafe { raw_getpwuid(n) };
        let primary_gid = if ptr.is_null() {
            None
        } else {
            Some(unsafe { (*ptr).pw_gid })
        };
        return Ok((n, primary_gid));
    }
    let c_name = CString::new(s).map_err(|_| format!("invalid user name: '{}'", s))?;
    let _account_db_guard = lock_account_db();
    let ptr = unsafe { raw_getpwnam(c_name.as_ptr()) };
    if ptr.is_null() {
        return Err(format!("invalid user: '{}'", s));
    }
    unsafe { Ok(((*ptr).pw_uid, Some((*ptr).pw_gid))) }
}

#[cfg(unix)]
fn resolve_gid(s: &str) -> Result<u32, String> {
    if let Ok(n) = s.parse::<u32>() {
        return Ok(n);
    }
    let c_name = CString::new(s).map_err(|_| format!("invalid group name: '{}'", s))?;
    let _account_db_guard = lock_account_db();
    let ptr = unsafe { raw_getgrnam(c_name.as_ptr()) };
    if ptr.is_null() {
        return Err(format!("invalid group: '{}'", s));
    }
    unsafe { Ok((*ptr).gr_gid) }
}

#[cfg(unix)]
fn apply_chown(path: &str, uid: u32, gid: u32, recursive: bool) -> Result<(), std::io::Error> {
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
            apply_chown(&entry_str, uid, gid, true)?;
        }
    }

    Ok(())
}

#[cfg(unix)]
extern "C" {
    #[link_name = "chown"]
    fn raw_chown(path: *const c_char, owner: u32, group: u32) -> i32;

    #[link_name = "lchown"]
    fn raw_lchown(path: *const c_char, owner: u32, group: u32) -> i32;
}

#[cfg(all(test, unix))]
mod tests {
    use super::{parse_owner_spec, resolve_user};
    use std::sync::{Arc, Barrier};

    #[test]
    fn owner_only_keeps_group_unchanged() {
        assert_eq!(parse_owner_spec("12345").unwrap(), (12345, u32::MAX));
    }

    #[test]
    fn group_only_keeps_owner_unchanged() {
        assert_eq!(parse_owner_spec(":23456").unwrap(), (u32::MAX, 23456));
    }

    #[test]
    fn explicit_owner_and_group_are_both_applied() {
        assert_eq!(parse_owner_spec("12345:23456").unwrap(), (12345, 23456));
    }

    #[test]
    fn serializes_legacy_account_lookups() {
        const THREADS: usize = 8;
        const LOOKUPS_PER_THREAD: usize = 64;

        let barrier = Arc::new(Barrier::new(THREADS));
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..LOOKUPS_PER_THREAD {
                        let _ = resolve_user("0", true);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
