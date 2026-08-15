use crate::core::Applet;
use std::io::{self, Write};

pub struct UptimeApplet;

impl Applet for UptimeApplet {
    fn name(&self) -> &'static str {
        "uptime"
    }

    fn description(&self) -> &'static str {
        "Tell how long the system has been running"
    }

    #[cfg(target_os = "linux")]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let stdout = io::stdout();
        let mut out = stdout.lock();

        let uptime_secs = parse_uptime_linux()?;
        let (load1, load5, load15, _running, _total) = parse_loadavg_linux()?;

        let current_time = current_time_string();

        let days = uptime_secs / 86400;
        let remaining = uptime_secs % 86400;
        let hours = remaining / 3600;
        let minutes = (remaining % 3600) / 60;

        let uptime_str = if days > 0 {
            format!(
                "up {} day{}, {:02}:{:02}",
                days,
                if days == 1 { "" } else { "s" },
                hours,
                minutes
            )
        } else {
            format!("up {:02}:{:02}", hours, minutes)
        };

        writeln!(
            out,
            " {}  {},  1 user,  load average: {:.2}, {:.2}, {:.2}",
            current_time, uptime_str, load1, load5, load15
        )?;

        Ok(0)
    }

    #[cfg(target_os = "macos")]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let stdout = io::stdout();
        let mut out = stdout.lock();

        let uptime_secs = get_uptime_macos()?;
        let (load1, load5, load15) = get_loadavg_macos()?;

        let current_time = current_time_string();

        let days = uptime_secs / 86400;
        let remaining = uptime_secs % 86400;
        let hours = remaining / 3600;
        let minutes = (remaining % 3600) / 60;

        let uptime_str = if days > 0 {
            format!(
                "up {} day{}, {:02}:{:02}",
                days,
                if days == 1 { "" } else { "s" },
                hours,
                minutes
            )
        } else {
            format!("up {:02}:{:02}", hours, minutes)
        };

        writeln!(
            out,
            " {}  {},  1 user,  load average: {:.2}, {:.2}, {:.2}",
            current_time, uptime_str, load1, load5, load15
        )?;

        Ok(0)
    }

    #[cfg(windows)]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let stdout = io::stdout();
        let mut out = stdout.lock();

        let uptime_secs = get_windows_uptime()?;
        let current_time = current_time_string();

        let days = uptime_secs / 86400;
        let remaining = uptime_secs % 86400;
        let hours = remaining / 3600;
        let minutes = (remaining % 3600) / 60;

        let uptime_str = if days > 0 {
            format!(
                "up {} day{}, {:02}:{:02}",
                days,
                if days == 1 { "" } else { "s" },
                hours,
                minutes
            )
        } else {
            format!("up {:02}:{:02}", hours, minutes)
        };

        writeln!(out, " {}  {},  1 user", current_time, uptime_str)?;

        Ok(0)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        eprintln!("uptime: not supported on this platform");
        Ok(1)
    }

    fn help(&self) {
        println!("Usage: uptime");
        println!();
        println!("{}", self.description());
    }
}

#[cfg(target_os = "linux")]
fn parse_uptime_linux() -> Result<u64, io::Error> {
    use std::fs;
    let content = fs::read_to_string("/proc/uptime")?;
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "empty /proc/uptime",
        ));
    }
    let secs: f64 = parts[0]
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid uptime value"))?;
    Ok(secs as u64)
}

#[cfg(target_os = "linux")]
fn parse_loadavg_linux() -> Result<(f64, f64, f64, u32, u32), io::Error> {
    use std::fs;
    let content = fs::read_to_string("/proc/loadavg")?;
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() < 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unexpected /proc/loadavg format",
        ));
    }
    let load1: f64 = parts[0]
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid load1"))?;
    let load5: f64 = parts[1]
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid load5"))?;
    let load15: f64 = parts[2]
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid load15"))?;

    let task_parts: Vec<&str> = parts[3].split('/').collect();
    let running: u32 = task_parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let total: u32 = task_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    Ok((load1, load5, load15, running, total))
}

#[cfg(target_os = "macos")]
fn get_uptime_macos() -> Result<u64, io::Error> {
    let output = std::process::Command::new("sysctl")
        .arg("-n")
        .arg("kern.boottime")
        .output()?;
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stdout_str = stdout_str.trim();
    if let Some(start) = stdout_str.find("sec = ") {
        let rest = &stdout_str[start + 6..];
        if let Some(end) = rest.find(',') {
            let secs_str = rest[..end].trim();
            let boot_secs: u64 = secs_str.parse().unwrap_or(0);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            return Ok(now.saturating_sub(boot_secs));
        }
    }
    Ok(0)
}

#[cfg(target_os = "macos")]
fn get_loadavg_macos() -> Result<(f64, f64, f64), io::Error> {
    let output = std::process::Command::new("sysctl")
        .arg("-n")
        .arg("vm.loadavg")
        .output()?;
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stdout_str = stdout_str.trim();
    let cleaned = stdout_str.replace(['{', '}'], "");
    let parts: Vec<&str> = cleaned.split_whitespace().collect();
    if parts.len() >= 3 {
        let load1: f64 = parts[0].parse().unwrap_or(0.0);
        let load5: f64 = parts[1].parse().unwrap_or(0.0);
        let load15: f64 = parts[2].parse().unwrap_or(0.0);
        Ok((load1, load5, load15))
    } else {
        Ok((0.0, 0.0, 0.0))
    }
}

#[cfg(unix)]
fn current_time_string() -> String {
    let mut tv: libc_timeval = unsafe { std::mem::zeroed() };
    unsafe { raw_gettimeofday(&mut tv, std::ptr::null_mut()) };

    let total_secs = tv.tv_sec as u64;
    let secs_of_day = total_secs % 86400;
    let hours = secs_of_day / 3600;
    let minutes = (secs_of_day % 3600) / 60;
    let seconds = secs_of_day % 60;

    let days_since_epoch = total_secs / 86400;
    let (year, month, day) = unix_days_to_ymd(days_since_epoch);

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hours, minutes, seconds
    )
}

#[cfg(windows)]
fn current_time_string() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = now.as_secs();
    let secs_of_day = total_secs % 86400;
    let hours = secs_of_day / 3600;
    let minutes = (secs_of_day % 3600) / 60;
    let seconds = secs_of_day % 60;

    let days_since_epoch = total_secs / 86400;
    let (year, month, day) = unix_days_to_ymd(days_since_epoch);

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hours, minutes, seconds
    )
}

#[cfg(windows)]
fn get_windows_uptime() -> Result<u64, io::Error> {
    let output = std::process::Command::new("wmic")
        .args(["os", "get", "LastBootUpTime", "/Value"])
        .output()?;
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    for line in stdout_str.lines() {
        if let Some(val) = line.strip_prefix("LastBootUpTime=") {
            let val = val.trim();
            if val.len() >= 14 {
                let year: u64 = val[0..4].parse().unwrap_or(2024);
                let month: u64 = val[4..6].parse().unwrap_or(1);
                let day: u64 = val[6..8].parse().unwrap_or(1);
                let hour: u64 = val[8..10].parse().unwrap_or(0);
                let min: u64 = val[10..12].parse().unwrap_or(0);
                let sec: u64 = val[12..14].parse().unwrap_or(0);

                let boot_days = ymd_to_days(year, month, day);
                let boot_secs = boot_days * 86400 + hour * 3600 + min * 60 + sec;

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                return Ok(now.saturating_sub(boot_secs));
            }
        }
    }
    Ok(0)
}

fn unix_days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let days_in_months: [u64; 12] = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &dim in &days_in_months {
        if days < dim {
            break;
        }
        days -= dim;
        month += 1;
    }
    (year, month, days + 1)
}

#[cfg(windows)]
fn ymd_to_days(year: u64, month: u64, day: u64) -> u64 {
    let mut days: u64 = 0;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }
    let days_in_months: [u64; 12] = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    for days_in_month in days_in_months.iter().take((month as usize - 1).min(12)) {
        days += days_in_month;
    }
    days += day - 1;
    days
}

fn is_leap(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

#[cfg(unix)]
#[repr(C)]
struct libc_timeval {
    tv_sec: i64,
    tv_usec: i64,
}

#[cfg(unix)]
extern "C" {
    #[link_name = "gettimeofday"]
    fn raw_gettimeofday(tv: *mut libc_timeval, tz: *mut u8) -> i32;
}
