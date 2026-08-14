use crate::core::Applet;
use std::io::{self, Write};

pub struct IdApplet;

impl Applet for IdApplet {
    fn name(&self) -> &'static str {
        "id"
    }

    fn description(&self) -> &'static str {
        "Print real and effective user and group IDs"
    }

    #[cfg(unix)]
    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut uid_only = false;
        let mut gid_only = false;
        let mut groups_only = false;
        let mut name_only = false;
        let mut user: Option<&str> = None;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-u" | "--user" => uid_only = true,
                "-g" | "--group" => gid_only = true,
                "-G" | "--groups" => groups_only = true,
                "-n" | "--name" => name_only = true,
                _ if args[i].starts_with('-') && args[i].len() > 1 => {
                    let mut combined = true;
                    for ch in args[i][1..].chars() {
                        match ch {
                            'u' | 'g' | 'G' | 'n' => {}
                            _ => {
                                combined = false;
                                break;
                            }
                        }
                    }
                    if combined {
                        for ch in args[i][1..].chars() {
                            match ch {
                                'u' => uid_only = true,
                                'g' => gid_only = true,
                                'G' => groups_only = true,
                                'n' => name_only = true,
                                _ => {}
                            }
                        }
                    } else {
                        eprintln!("id: invalid option -- '{}'", &args[i]);
                        return Ok(1);
                    }
                }
                _ => {
                    user = Some(&args[i]);
                }
            }
            i += 1;
        }

        if uid_only && gid_only {
            eprintln!("id: cannot print only UID and only GID");
            return Ok(1);
        }
        if uid_only && groups_only {
            eprintln!("id: cannot print only UID and only groups");
            return Ok(1);
        }
        if gid_only && groups_only {
            eprintln!("id: cannot print only GID and only groups");
            return Ok(1);
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();

        match user {
            Some(u) => {
                let pw = match get_passwd_by_name(u) {
                    Some(p) => p,
                    None => {
                        eprintln!("id: '{}': no such user", u);
                        return Ok(1);
                    }
                };

                if uid_only {
                    if name_only {
                        writeln!(out, "{}", pw.name)?;
                    } else {
                        writeln!(out, "{}", pw.uid)?;
                    }
                } else if gid_only {
                    if name_only {
                        let gname = get_group_name_by_gid(pw.gid).unwrap_or(pw.gid.to_string());
                        writeln!(out, "{}", gname)?;
                    } else {
                        writeln!(out, "{}", pw.gid)?;
                    }
                } else if groups_only {
                    let gids = get_supplementary_gids_by_name(&pw.name);
                    if name_only {
                        let names: Vec<String> = gids
                            .iter()
                            .map(|&g| get_group_name_by_gid(g).unwrap_or(g.to_string()))
                            .collect();
                        writeln!(out, "{}", names.join(" "))?;
                    } else {
                        let strs: Vec<String> = gids.iter().map(|g| g.to_string()).collect();
                        writeln!(out, "{}", strs.join(" "))?;
                    }
                } else {
                    let gname = get_group_name_by_gid(pw.gid).unwrap_or(pw.gid.to_string());
                    let gids = get_supplementary_gids_by_name(&pw.name);
                    let groups_str = format_groups(&gids, name_only);
                    writeln!(
                        out,
                        "uid={}({}) gid={}({}) groups={}",
                        pw.uid, pw.name, pw.gid, gname, groups_str
                    )?;
                }
            }
            None => {
                let ruid = unsafe { raw_getuid() };
                let euid = unsafe { raw_geteuid() };
                let rgid = unsafe { raw_getgid() };
                let egid = unsafe { raw_getegid() };

                if uid_only {
                    if name_only {
                        let name = get_username_by_uid(euid).unwrap_or(euid.to_string());
                        writeln!(out, "{}", name)?;
                    } else {
                        writeln!(out, "{}", euid)?;
                    }
                } else if gid_only {
                    if name_only {
                        let name = get_group_name_by_gid(egid).unwrap_or(egid.to_string());
                        writeln!(out, "{}", name)?;
                    } else {
                        writeln!(out, "{}", egid)?;
                    }
                } else if groups_only {
                    let gids = get_groups();
                    if name_only {
                        let names: Vec<String> = gids
                            .iter()
                            .map(|&g| get_group_name_by_gid(g).unwrap_or(g.to_string()))
                            .collect();
                        writeln!(out, "{}", names.join(" "))?;
                    } else {
                        let strs: Vec<String> = gids.iter().map(|g| g.to_string()).collect();
                        writeln!(out, "{}", strs.join(" "))?;
                    }
                } else {
                    let runame = get_username_by_uid(ruid).unwrap_or(ruid.to_string());
                    let euname = get_username_by_uid(euid).unwrap_or(euid.to_string());
                    let rgname = get_group_name_by_gid(rgid).unwrap_or(rgid.to_string());
                    let egname = get_group_name_by_gid(egid).unwrap_or(egid.to_string());
                    let gids = get_groups();
                    let groups_str = format_groups(&gids, name_only);

                    if ruid == euid {
                        write!(out, "uid={}({})", ruid, runame)?;
                    } else {
                        write!(out, "uid={}({}) euid={}({})", ruid, runame, euid, euname)?;
                    }
                    if rgid == egid {
                        write!(out, " gid={}({})", rgid, rgname)?;
                    } else {
                        write!(out, " gid={}({}) egid={}({})", rgid, rgname, egid, egname)?;
                    }
                    writeln!(out, " groups={}", groups_str)?;
                }
            }
        }

        Ok(0)
    }

    #[cfg(windows)]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let username = std::env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string());
        let stdout = io::stdout();
        let mut out = stdout.lock();
        writeln!(out, "uid=0({}) gid=0", username)?;
        Ok(0)
    }

    #[cfg(not(any(unix, windows)))]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        eprintln!("id: not supported on this platform");
        Ok(1)
    }

    fn help(&self) {
        println!("Usage: id [OPTION]... [USER]");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -u, --user      Print only the effective user ID");
        println!("  -g, --group     Print only the effective group ID");
        println!("  -G, --groups    Print all group IDs");
        println!("  -n, --name      Print a name instead of a number (requires -u/g/G)");
    }
}

#[cfg(unix)]
struct PasswdInfo {
    name: String,
    uid: u32,
    gid: u32,
}

#[cfg(unix)]
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

#[cfg(unix)]
#[repr(C)]
struct Group {
    gr_name: *const i8,
    gr_passwd: *const i8,
    gr_gid: u32,
    gr_mem: *const *const i8,
}

#[cfg(unix)]
fn c_char_to_string(ptr: *const i8) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr as *const u8, len);
        String::from_utf8_lossy(slice).to_string()
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
fn get_passwd_by_name(name: &str) -> Option<PasswdInfo> {
    let c_name = std::ffi::CString::new(name).ok()?;
    let ptr = unsafe { raw_getpwnam(c_name.as_ptr()) };
    if ptr.is_null() {
        return None;
    }
    unsafe {
        Some(PasswdInfo {
            name: c_char_to_string((*ptr).pw_name),
            uid: (*ptr).pw_uid,
            gid: (*ptr).pw_gid,
        })
    }
}

#[cfg(unix)]
fn get_group_name_by_gid(gid: u32) -> Option<String> {
    let ptr = unsafe { raw_getgrgid(gid) };
    if ptr.is_null() {
        return None;
    }
    unsafe {
        let gr_name = (*ptr).gr_name;
        if gr_name.is_null() {
            return None;
        }
        Some(c_char_to_string(gr_name))
    }
}

#[cfg(unix)]
fn get_groups() -> Vec<u32> {
    let mut ngroups: i32 = 64;
    let mut buf: Vec<u32> = vec![0; ngroups as usize];
    let ret = unsafe { raw_getgroups(&mut ngroups, buf.as_mut_ptr()) };
    if ret < 0 {
        return vec![];
    }
    buf.truncate(ret as usize);
    buf
}

#[cfg(unix)]
fn get_supplementary_gids_by_name(username: &str) -> Vec<u32> {
    let c_name = match std::ffi::CString::new(username) {
        Ok(n) => n,
        Err(_) => return vec![],
    };
    let mut ngroups: i32 = 64;
    let mut buf: Vec<i32> = vec![0; ngroups as usize];
    let ret = unsafe { raw_getgrouplist(c_name.as_ptr(), 0, buf.as_mut_ptr(), &mut ngroups) };
    if ret < 0 {
        return vec![];
    }
    buf.truncate(ret as usize);
    buf.iter().map(|&g| g as u32).collect()
}

#[cfg(unix)]
fn format_groups(gids: &[u32], name_only: bool) -> String {
    let parts: Vec<String> = gids
        .iter()
        .map(|&g| {
            if name_only {
                let name = get_group_name_by_gid(g).unwrap_or(g.to_string());
                format!("{}({})", g, name)
            } else {
                let name = get_group_name_by_gid(g).unwrap_or_default();
                if name.is_empty() {
                    g.to_string()
                } else {
                    format!("{}({})", g, name)
                }
            }
        })
        .collect();
    parts.join(",")
}

#[cfg(unix)]
extern "C" {
    #[link_name = "getuid"]
    fn raw_getuid() -> u32;

    #[link_name = "geteuid"]
    fn raw_geteuid() -> u32;

    #[link_name = "getgid"]
    fn raw_getgid() -> u32;

    #[link_name = "getegid"]
    fn raw_getegid() -> u32;

    #[link_name = "getpwuid"]
    fn raw_getpwuid(uid: u32) -> *const Passwd;

    #[link_name = "getpwnam"]
    fn raw_getpwnam(name: *const i8) -> *const Passwd;

    #[link_name = "getgrgid"]
    fn raw_getgrgid(gid: u32) -> *const Group;

    #[link_name = "getgroups"]
    fn raw_getgroups(size: *mut i32, list: *mut u32) -> i32;

    #[link_name = "getgrouplist"]
    fn raw_getgrouplist(user: *const i8, group: u32, groups: *mut i32, ngroups: *mut i32) -> i32;
}
