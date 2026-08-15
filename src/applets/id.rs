#[cfg(unix)]
use crate::core::unix_ffi::{lock_account_db, raw_getgrgid, raw_getpwnam, raw_getpwuid};
use crate::core::Applet;
#[cfg(unix)]
use std::ffi::{c_char, CStr};
#[cfg(unix)]
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
                        eprintln!("id: invalid option -- '{}'", args[i]);
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
                    let gids = get_supplementary_gids_by_name(&pw.name, pw.gid);
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
                    let gids = get_supplementary_gids_by_name(&pw.name, pw.gid);
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
        eprintln!("id: not supported on this platform");
        Ok(1)
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
fn c_char_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}

#[cfg(unix)]
fn get_username_by_uid(uid: u32) -> Option<String> {
    let _account_db_guard = lock_account_db();
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
    let _account_db_guard = lock_account_db();
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
    let _account_db_guard = lock_account_db();
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
    let count = unsafe { raw_getgroups(0, std::ptr::null_mut()) };
    if count < 0 {
        return vec![];
    }

    let mut buf = vec![0; count as usize];
    let ret = unsafe { raw_getgroups(count, buf.as_mut_ptr()) };
    if ret < 0 {
        return vec![];
    }
    buf.truncate(ret as usize);

    let effective_gid = unsafe { raw_getegid() };
    if !buf.contains(&effective_gid) {
        buf.insert(0, effective_gid);
    }

    // Darwin's process credential list can omit extended directory-service
    // memberships. The system `id` command includes those memberships, so
    // merge them without discarding groups that are active on the process.
    #[cfg(target_os = "macos")]
    if let Some((username, passwd)) = get_username_by_uid(unsafe { raw_geteuid() })
        .and_then(|username| get_passwd_by_name(&username).map(|passwd| (username, passwd)))
    {
        for gid in get_supplementary_gids_by_name(&username, passwd.gid) {
            if !buf.contains(&gid) {
                buf.push(gid);
            }
        }
    }

    buf
}

#[cfg(all(unix, not(target_os = "macos")))]
fn get_supplementary_gids_by_name(username: &str, primary_gid: u32) -> Vec<u32> {
    let c_name = match std::ffi::CString::new(username) {
        Ok(n) => n,
        Err(_) => return vec![],
    };
    let mut ngroups = 0;
    unsafe {
        raw_getgrouplist(
            c_name.as_ptr(),
            primary_gid,
            std::ptr::null_mut(),
            &mut ngroups,
        );
    }
    if ngroups <= 0 {
        return vec![];
    }

    let mut buf: Vec<i32> = vec![0; ngroups as usize];
    let ret =
        unsafe { raw_getgrouplist(c_name.as_ptr(), primary_gid, buf.as_mut_ptr(), &mut ngroups) };
    if ret < 0 || ngroups < 0 {
        return vec![];
    }
    buf.truncate(ngroups as usize);
    buf.iter().map(|&g| g as u32).collect()
}

#[cfg(target_os = "macos")]
fn get_supplementary_gids_by_name(username: &str, primary_gid: u32) -> Vec<u32> {
    let c_name = match std::ffi::CString::new(username) {
        Ok(name) => name,
        Err(_) => return vec![],
    };
    let mut groups = std::ptr::null_mut();
    let count = unsafe { raw_getgrouplist_2(c_name.as_ptr(), primary_gid, &mut groups) };
    if count < 0 {
        if !groups.is_null() {
            unsafe {
                raw_free(groups.cast());
            }
        }
        return vec![];
    }
    if groups.is_null() {
        return vec![];
    }

    let result = unsafe { std::slice::from_raw_parts(groups, count as usize).to_vec() };
    unsafe {
        raw_free(groups.cast());
    }
    result
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

    #[link_name = "getgroups"]
    fn raw_getgroups(size: i32, list: *mut u32) -> i32;

    #[cfg(not(target_os = "macos"))]
    #[link_name = "getgrouplist"]
    fn raw_getgrouplist(
        user: *const c_char,
        group: u32,
        groups: *mut i32,
        ngroups: *mut i32,
    ) -> i32;

    #[cfg(target_os = "macos")]
    #[link_name = "getgrouplist_2"]
    fn raw_getgrouplist_2(user: *const c_char, group: u32, groups: *mut *mut u32) -> i32;

    #[cfg(target_os = "macos")]
    #[link_name = "free"]
    fn raw_free(ptr: *mut std::ffi::c_void);
}
