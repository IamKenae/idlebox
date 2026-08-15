use crate::core::Applet;
use std::io::{self, Write};

pub struct DfApplet;

impl Applet for DfApplet {
    fn name(&self) -> &'static str {
        "df"
    }

    fn description(&self) -> &'static str {
        "Report file system disk space usage"
    }

    #[cfg(target_os = "linux")]
    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut human_readable = false;
        let mut target_path: Option<&str> = None;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-h" | "--human-readable" => human_readable = true,
                _ if args[i].starts_with('-') => {
                    let mut combined = true;
                    for ch in args[i][1..].chars() {
                        match ch {
                            'h' => human_readable = true,
                            _ => {
                                combined = false;
                                break;
                            }
                        }
                    }
                    if !combined {
                        eprintln!("df: invalid option -- '{}'", &args[i][1..]);
                        return Ok(1);
                    }
                }
                _ => {
                    target_path = Some(&args[i]);
                }
            }
            i += 1;
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();

        if let Some(path) = target_path {
            let mount = find_mount_for_path(path)?;
            print_header(&mut out)?;
            print_mount_entry(&mut out, &mount, human_readable)?;
        } else {
            let mounts = parse_proc_mounts()?;
            print_header(&mut out)?;
            let mut seen = std::collections::HashSet::new();
            for mount in &mounts {
                if seen.contains(&mount.mount_point) {
                    continue;
                }
                seen.insert(mount.mount_point.clone());
                if mount.fs_type == "proc"
                    || mount.fs_type == "sysfs"
                    || mount.fs_type == "devtmpfs"
                    || mount.fs_type == "devpts"
                    || mount.fs_type == "securityfs"
                    || mount.fs_type == "cgroup"
                    || mount.fs_type == "cgroup2"
                    || mount.fs_type == "pstore"
                    || mount.fs_type == "debugfs"
                    || mount.fs_type == "hugetlbfs"
                    || mount.fs_type == "mqueue"
                    || mount.fs_type == "binfmt_misc"
                    || mount.fs_type == "configfs"
                    || mount.fs_type == "fusectl"
                    || mount.fs_type == "tracefs"
                    || mount.fs_type == "rpc_pipefs"
                    || mount.fs_type == "nsfs"
                    || mount.fs_type == "bpf"
                    || mount.fs_type == "autofs"
                    || mount.fs_type == "efivarfs"
                {
                    continue;
                }
                if let Ok(stat) = statvfs_linux(&mount.mount_point) {
                    print_statvfs_line(
                        &mut out,
                        &mount.device,
                        &mount.mount_point,
                        &stat,
                        human_readable,
                    )?;
                }
            }
        }

        Ok(0)
    }

    #[cfg(target_os = "macos")]
    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut human_readable = false;
        let mut target_path: Option<String> = None;

        for arg in args {
            match arg.as_str() {
                "-h" | "--human-readable" => human_readable = true,
                _ if arg.starts_with('-') => {}
                _ => {
                    target_path = Some(arg.clone());
                }
            }
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();
        print_header(&mut out)?;

        if let Some(path) = target_path {
            if let Ok(info) = statfs_macos(&path) {
                print_statfs_line(&mut out, &info.0, &path, &info.1, human_readable)?;
            }
        } else {
            let output = std::process::Command::new("mount").output()?;
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            for line in stdout_str.lines() {
                if let Some(info) = parse_mount_line(line) {
                    if let Ok(stat) = statfs_macos(&info.1) {
                        print_statfs_line(&mut out, &info.0, &info.1, &stat.1, human_readable)?;
                    }
                }
            }
        }

        Ok(0)
    }

    #[cfg(windows)]
    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut human_readable = false;
        let mut target_path: Option<&str> = None;

        for arg in args {
            match arg.as_str() {
                "-h" | "--human-readable" => human_readable = true,
                _ if arg.starts_with('-') => {}
                _ => {
                    target_path = Some(arg.as_str());
                }
            }
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();

        print_header(&mut out)?;

        if let Some(path) = target_path {
            let info = get_disk_space_windows(path)?;
            print_disk_line(&mut out, path, &info, human_readable)?;
        } else {
            for drive_letter in b'A'..=b'Z' {
                let drive = format!("{}:\\", drive_letter as char);
                if let Ok(info) = get_disk_space_windows(&drive) {
                    print_disk_line(&mut out, &drive, &info, human_readable)?;
                }
            }
        }

        Ok(0)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        eprintln!("df: not supported on this platform");
        Ok(1)
    }

    fn help(&self) {
        println!("Usage: df [OPTION]... [FILE]");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -h, --human-readable  Print sizes in human readable format");
    }
}

#[cfg(target_os = "linux")]
struct MountEntry {
    device: String,
    mount_point: String,
    fs_type: String,
}

#[cfg(target_os = "linux")]
fn parse_proc_mounts() -> Result<Vec<MountEntry>, io::Error> {
    use std::fs;
    use std::io::BufRead;
    let file = fs::File::open("/proc/mounts")?;
    let reader = io::BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            entries.push(MountEntry {
                device: unescape_mount_path(parts[0]),
                mount_point: unescape_mount_path(parts[1]),
                fs_type: parts[2].to_string(),
            });
        }
    }

    Ok(entries)
}

#[cfg(target_os = "linux")]
fn unescape_mount_path(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            let mut escaped = String::new();
            for _ in 0..3 {
                if let Some(h) = chars.next() {
                    escaped.push(h);
                }
            }
            if escaped.len() == 3 && escaped.chars().all(|digit| matches!(digit, '0'..='7')) {
                let code = u8::from_str_radix(&escaped, 8).expect("validated octal escape");
                result.push(code as char);
            } else {
                result.push('\\');
                result.push_str(&escaped);
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(target_os = "linux")]
fn find_mount_for_path(path: &str) -> Result<MountEntry, io::Error> {
    use std::fs;
    let canonical = fs::canonicalize(path)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("{}: {}", path, e)))?;
    let canonical_str = canonical.to_string_lossy().to_string();

    let mounts = parse_proc_mounts()?;

    let mut best_match: Option<&MountEntry> = None;
    let mut best_len = 0;

    for mount in &mounts {
        for candidate in [&canonical_str, path] {
            if std::path::Path::new(candidate).starts_with(&mount.mount_point)
                && mount.mount_point.len() > best_len
            {
                best_len = mount.mount_point.len();
                best_match = Some(mount);
            }
        }
    }

    match best_match {
        Some(m) => Ok(MountEntry {
            device: m.device.clone(),
            mount_point: m.mount_point.clone(),
            fs_type: m.fs_type.clone(),
        }),
        None => Ok(MountEntry {
            device: "none".to_string(),
            mount_point: path.to_string(),
            fs_type: "unknown".to_string(),
        }),
    }
}

#[cfg(target_os = "linux")]
struct Statvfs {
    block_size: u64,
    blocks: u64,
    blocks_free: u64,
    blocks_avail: u64,
}

#[cfg(target_os = "linux")]
fn statvfs_linux(path: &str) -> Result<Statvfs, io::Error> {
    let mut buf: libc_statvfs = unsafe { std::mem::zeroed() };
    let c_path = std::ffi::CString::new(path)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?;
    let ret = unsafe { raw_statvfs(c_path.as_ptr(), &mut buf) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Statvfs {
        block_size: buf.f_frsize,
        blocks: buf.f_blocks,
        blocks_free: buf.f_bfree,
        blocks_avail: buf.f_bavail,
    })
}

#[cfg(target_os = "linux")]
#[repr(C)]
struct libc_statvfs {
    f_bsize: u64,
    f_frsize: u64,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_favail: u64,
    f_fsid: u64,
    f_flag: u64,
    f_namemax: u64,
    __f_spare: [i32; 6],
}

#[cfg(target_os = "linux")]
extern "C" {
    #[link_name = "statvfs"]
    fn raw_statvfs(path: *const std::ffi::c_char, buf: *mut libc_statvfs) -> std::ffi::c_int;
}

#[cfg(target_os = "macos")]
fn parse_mount_line(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.splitn(2, " on ").collect();
    if parts.len() < 2 {
        return None;
    }
    let device = parts[0].to_string();
    let rest = parts[1];
    let mount_point = rest.split(" (").next()?.to_string();
    Some((device, mount_point))
}

#[cfg(target_os = "macos")]
fn statfs_macos(path: &str) -> Result<(String, String), io::Error> {
    let output = std::process::Command::new("df")
        .args(&["-P", "-k", path])
        .output()?;
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout_str.trim().lines().collect();
    if lines.len() < 2 {
        return Err(io::Error::new(io::ErrorKind::Other, "df output too short"));
    }
    let parts: Vec<&str> = lines[1].split_whitespace().collect();
    if parts.len() < 5 {
        return Err(io::Error::new(io::ErrorKind::Other, "unexpected df format"));
    }
    let device = parts[0].to_string();
    let info = format!("{} {} {} {}", parts[1], parts[2], parts[3], parts[4]);
    Ok((device, info))
}

fn print_header(out: &mut impl Write) -> Result<(), io::Error> {
    writeln!(
        out,
        "{:<20} {:>10} {:>10} {:>10} {:>5}  Mounted on",
        "Filesystem", "Size", "Used", "Avail", "Use%"
    )?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn print_mount_entry(
    out: &mut impl Write,
    mount: &MountEntry,
    human_readable: bool,
) -> Result<(), io::Error> {
    let stat = statvfs_linux(&mount.mount_point)?;
    print_statvfs_line(
        out,
        &mount.device,
        &mount.mount_point,
        &stat,
        human_readable,
    )
}

#[cfg(target_os = "linux")]
fn print_statvfs_line(
    out: &mut impl Write,
    device: &str,
    mount_point: &str,
    stat: &Statvfs,
    human_readable: bool,
) -> Result<(), io::Error> {
    let total = stat.block_size * stat.blocks;
    let free = stat.block_size * stat.blocks_free;
    let avail = stat.block_size * stat.blocks_avail;
    let used = total - free;
    let use_pct = if total == 0 {
        0.0
    } else {
        (used as f64 / (used as f64 + avail as f64)) * 100.0
    };

    let (size_s, used_s, avail_s) = if human_readable {
        (human_size(total), human_size(used), human_size(avail))
    } else {
        let block_total = total / 1024;
        let block_used = used / 1024;
        let block_avail = avail / 1024;
        (
            format!("{}K", block_total),
            format!("{}K", block_used),
            format!("{}K", block_avail),
        )
    };

    writeln!(
        out,
        "{:<20} {:>10} {:>10} {:>10} {:>4.0}%  {}",
        device, size_s, used_s, avail_s, use_pct, mount_point
    )?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn print_statfs_line(
    out: &mut impl Write,
    device: &str,
    mount_point: &str,
    info: &str,
    _human_readable: bool,
) -> Result<(), io::Error> {
    let parts: Vec<&str> = info.split_whitespace().collect();
    if parts.len() >= 4 {
        let total_kb: u64 = parts[0].parse().unwrap_or(0);
        let used_kb: u64 = parts[1].parse().unwrap_or(0);
        let avail_kb: u64 = parts[2].parse().unwrap_or(0);
        let use_pct_str = parts[3].trim_end_matches('%');
        let use_pct: f64 = use_pct_str.parse().unwrap_or(0.0);

        writeln!(
            out,
            "{:<20} {:>10} {:>10} {:>10} {:>4.0}%  {}",
            device,
            format!("{}K", total_kb),
            format!("{}K", used_kb),
            format!("{}K", avail_kb),
            use_pct,
            mount_point
        )?;
    }
    Ok(())
}

#[cfg(windows)]
struct WindowsDiskSpace {
    total: u64,
    available: u64,
    free: u64,
}

#[cfg(windows)]
fn get_disk_space_windows(path: &str) -> Result<WindowsDiskSpace, io::Error> {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    let p = Path::new(path);
    if !p.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "path not found"));
    }

    let query_path = if p.is_file() {
        p.parent().unwrap_or(p)
    } else {
        p
    };
    let wide_path: Vec<u16> = query_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut available = 0u64;
    let mut total = 0u64;
    let mut free = 0u64;
    let result = unsafe {
        raw_get_disk_free_space_ex_w(wide_path.as_ptr(), &mut available, &mut total, &mut free)
    };
    if result == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(WindowsDiskSpace {
        total,
        available,
        free,
    })
}

#[cfg(windows)]
fn print_disk_line(
    out: &mut impl Write,
    mount_point: &str,
    info: &WindowsDiskSpace,
    human_readable: bool,
) -> Result<(), io::Error> {
    let used = info.total.saturating_sub(info.free);
    let use_pct = if used + info.available == 0 {
        0.0
    } else {
        used as f64 / (used + info.available) as f64 * 100.0
    };
    let (total, used, available) = if human_readable {
        (
            human_size(info.total),
            human_size(used),
            human_size(info.available),
        )
    } else {
        (
            format!("{}K", info.total / 1024),
            format!("{}K", used / 1024),
            format!("{}K", info.available / 1024),
        )
    };
    writeln!(
        out,
        "{:<20} {:>10} {:>10} {:>10} {:>4.0}%  {}",
        mount_point, total, used, available, use_pct, mount_point
    )?;
    Ok(())
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    #[link_name = "GetDiskFreeSpaceExW"]
    fn raw_get_disk_free_space_ex_w(
        directory_name: *const u16,
        free_bytes_available: *mut u64,
        total_bytes: *mut u64,
        total_free_bytes: *mut u64,
    ) -> i32;
}

#[cfg(any(target_os = "linux", windows))]
fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T", "P"];
    let mut size = bytes as f64;
    let mut idx = 0;
    while size >= 1024.0 && idx < UNITS.len() - 1 {
        size /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{}{}", bytes, UNITS[0])
    } else {
        format!("{:.1}{}", size, UNITS[idx])
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::unescape_mount_path;

    #[test]
    fn proc_mount_escapes_are_octal() {
        assert_eq!(
            unescape_mount_path(r"/path\040with\011space"),
            "/path with\tspace"
        );
    }
}
