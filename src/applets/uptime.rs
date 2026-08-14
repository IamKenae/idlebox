use crate::core::Applet;
use std::fs;
use std::io::{self, Write};

pub struct UptimeApplet;

impl Applet for UptimeApplet {
    fn name(&self) -> &'static str {
        "uptime"
    }

    fn description(&self) -> &'static str {
        "Tell how long the system has been running"
    }

    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let stdout = io::stdout();
        let mut out = stdout.lock();

        let uptime_secs = parse_uptime()?;
        let (load1, load5, load15, _running, _total) = parse_loadavg()?;

        let current_time = current_time_string();

        let days = uptime_secs / 86400;
        let remaining = uptime_secs % 86400;
        let hours = remaining / 3600;
        let minutes = (remaining % 3600) / 60;

        let uptime_str = if days > 0 {
            format!("up {} day{}, {:02}:{:02}", days, if days == 1 { "" } else { "s" }, hours, minutes)
        } else {
            format!("up {:02}:{:02}", hours, minutes)
        };

        writeln!(out, " {}  {},  1 user,  load average: {:.2}, {:.2}, {:.2}",
            current_time, uptime_str, load1, load5, load15)?;

        Ok(0)
    }

    fn help(&self) {
        println!("Usage: uptime");
        println!();
        println!("{}", self.description());
    }
}

fn parse_uptime() -> Result<u64, io::Error> {
    let content = fs::read_to_string("/proc/uptime")?;
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "empty /proc/uptime"));
    }
    let secs: f64 = parts[0].parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid uptime value"))?;
    Ok(secs as u64)
}

fn parse_loadavg() -> Result<(f64, f64, f64, u32, u32), io::Error> {
    let content = fs::read_to_string("/proc/loadavg")?;
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() < 4 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected /proc/loadavg format"));
    }
    let load1: f64 = parts[0].parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid load1"))?;
    let load5: f64 = parts[1].parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid load5"))?;
    let load15: f64 = parts[2].parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid load15"))?;

    let task_parts: Vec<&str> = parts[3].split('/').collect();
    let running: u32 = task_parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let total: u32 = task_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    Ok((load1, load5, load15, running, total))
}

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

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hours, minutes, seconds)
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

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[repr(C)]
struct libc_timeval {
    tv_sec: i64,
    tv_usec: i64,
}

extern "C" {
    #[link_name = "gettimeofday"]
    fn raw_gettimeofday(tv: *mut libc_timeval, tz: *mut u8) -> i32;
}
