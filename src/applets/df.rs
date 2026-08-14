use crate::core::Applet;
use std::fs;
use std::io::{self, BufRead, Write};

pub struct DfApplet;

impl Applet for DfApplet {
    fn name(&self) -> &'static str {
        "df"
    }

    fn description(&self) -> &'static str {
        "Report file system disk space usage"
    }

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
                if let Ok(stat) = statvfs(&mount.mount_point) {
                    print_statvfs_line(&mut out, &mount.device, &mount.mount_point, &stat, human_readable)?;
                }
            }
        }

        Ok(0)
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

struct MountEntry {
    device: String,
    mount_point: String,
    fs_type: String,
}

fn parse_proc_mounts() -> Result<Vec<MountEntry>, io::Error> {
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

fn unescape_mount_path(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            let mut hex = String::new();
            for _ in 0..3 {
                if let Some(h) = chars.next() {
                    hex.push(h);
                }
            }
            if let Ok(code) = u8::from_str_radix(&hex, 16) {
                result.push(code as char);
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn find_mount_for_path(path: &str) -> Result<MountEntry, io::Error> {
    let canonical = fs::canonicalize(path)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("{}: {}", path, e)))?;
    let path_str = canonical.to_string_lossy().to_string();

    let mounts = parse_proc_mounts()?;
    let mut best_match: Option<&MountEntry> = None;
    let mut best_len = 0;

    for mount in &mounts {
        if path_str == mount.mount_point || path_str.starts_with(&format!("{}/", mount.mount_point)) {
            if mount.mount_point.len() > best_len {
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
        None => Err(io::Error::new(io::ErrorKind::NotFound, "no mount found for path")),
    }
}

struct Statvfs {
    block_size: u64,
    blocks: u64,
    blocks_free: u64,
    blocks_avail: u64,
}

fn statvfs(path: &str) -> Result<Statvfs, io::Error> {
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

extern "C" {
    #[link_name = "statvfs"]
    fn raw_statvfs(path: *const std::ffi::c_char, buf: *mut libc_statvfs) -> std::ffi::c_int;
}

fn print_header(out: &mut impl Write) -> Result<(), io::Error> {
    writeln!(out, "{:<20} {:>10} {:>10} {:>10} {:>5}  {}", "Filesystem", "Size", "Used", "Avail", "Use%", "Mounted on")?;
    Ok(())
}

fn print_mount_entry(out: &mut impl Write, mount: &MountEntry, human_readable: bool) -> Result<(), io::Error> {
    let stat = statvfs(&mount.mount_point)?;
    print_statvfs_line(out, &mount.device, &mount.mount_point, &stat, human_readable)
}

fn print_statvfs_line(out: &mut impl Write, device: &str, mount_point: &str, stat: &Statvfs, human_readable: bool) -> Result<(), io::Error> {
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
        (format!("{}K", block_total), format!("{}K", block_used), format!("{}K", block_avail))
    };

    writeln!(out, "{:<20} {:>10} {:>10} {:>10} {:>4.0}%  {}", device, size_s, used_s, avail_s, use_pct, mount_point)?;
    Ok(())
}

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
