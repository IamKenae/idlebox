use crate::core::Applet;
use std::ffi::CString;
use std::path::Path;

pub struct ChownApplet;

impl Applet for ChownApplet {
    fn name(&self) -> &'static str {
        "chown"
    }

    fn description(&self) -> &'static str {
        "Change file owner and group"
    }

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
                eprintln!("chown: missing operand");
                return Ok(1);
            }
        };

        if paths.is_empty() {
            eprintln!("chown: missing operand after '{}'", owner_spec);
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

    fn help(&self) {
        println!("Usage: chown [OPTION]... [OWNER][:[GROUP]] FILE...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -R, --recursive   Change files and directories recursively");
        println!();
        println!("OWNER and GROUP may be numeric IDs or names.");
        println!("Examples: user, user:group, :group, 1000:1000");
    }
}

fn parse_owner_spec(spec: &str) -> Result<(u32, u32), String> {
    let uid = unsafe { raw_getuid() };
    let gid = unsafe { raw_getgid() };

    if spec.is_empty() {
        return Err("invalid owner spec".to_string());
    }

    if let Some(colon_pos) = spec.find(':') {
        let user_part = &spec[..colon_pos];
        let group_part = &spec[colon_pos + 1..];

        let resolved_uid = if user_part.is_empty() {
            uid
        } else {
            resolve_uid(user_part)?
        };

        let resolved_gid = if group_part.is_empty() {
            gid
        } else {
            resolve_gid(group_part)?
        };

        Ok((resolved_uid, resolved_gid))
    } else {
        let resolved_uid = resolve_uid(spec)?;
        Ok((resolved_uid, gid))
    }
}

fn resolve_uid(s: &str) -> Result<u32, String> {
    if let Ok(n) = s.parse::<u32>() {
        return Ok(n);
    }
    let c_name = CString::new(s).map_err(|_| format!("invalid user name: '{}'", s))?;
    let ptr = unsafe { raw_getpwnam(c_name.as_ptr()) };
    if ptr.is_null() {
        return Err(format!("invalid user: '{}'", s));
    }
    unsafe { Ok((*ptr).pw_uid) }
}

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

fn apply_chown(path: &str, uid: u32, gid: u32, recursive: bool) -> Result<(), std::io::Error> {
    let p = Path::new(path);
    let c_path = CString::new(path).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let is_symlink = p.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);

    let ret = if is_symlink {
        unsafe { raw_lchown(c_path.as_ptr(), uid, gid) }
    } else {
        unsafe { raw_chown(c_path.as_ptr(), uid, gid) }
    };

    if ret != 0 {
        let err = std::io::Error::last_os_error();
        return Err(err);
    }

    if recursive && p.is_dir() {
        for entry in std::fs::read_dir(p)? {
            let entry = entry?;
            let entry_path = entry.path();
            let entry_str = entry_path.to_string_lossy().to_string();
            apply_chown(&entry_str, uid, gid, true)?;
        }
    }

    Ok(())
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

#[repr(C)]
struct Group {
    gr_name: *const i8,
    gr_passwd: *const i8,
    gr_gid: u32,
    gr_mem: *const *const i8,
}

extern "C" {
    #[link_name = "getuid"]
    fn raw_getuid() -> u32;

    #[link_name = "getgid"]
    fn raw_getgid() -> u32;

    #[link_name = "getpwnam"]
    fn raw_getpwnam(name: *const i8) -> *const Passwd;

    #[link_name = "getgrnam"]
    fn raw_getgrnam(name: *const i8) -> *const Group;

    #[link_name = "chown"]
    fn raw_chown(path: *const i8, owner: u32, group: u32) -> i32;

    #[link_name = "lchown"]
    fn raw_lchown(path: *const i8, owner: u32, group: u32) -> i32;
}
