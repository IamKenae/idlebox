use crate::core::Applet;
use std::io::{self, Write};

pub struct FreeApplet;

impl Applet for FreeApplet {
    fn name(&self) -> &'static str {
        "free"
    }

    fn description(&self) -> &'static str {
        "Display amount of free and available memory"
    }

    #[cfg(unix)]
    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut human_readable = false;

        for arg in args {
            match arg.as_str() {
                "-h" | "--human-readable" => human_readable = true,
                _ if arg.starts_with('-') => {
                    let mut combined = true;
                    for ch in arg[1..].chars() {
                        match ch {
                            'h' => human_readable = true,
                            _ => {
                                combined = false;
                                break;
                            }
                        }
                    }
                    if !combined {
                        eprintln!("free: invalid option -- '{}'", &arg[1..]);
                        return Ok(1);
                    }
                }
                _ => {}
            }
        }

        let meminfo = parse_meminfo()?;

        let stdout = io::stdout();
        let mut out = stdout.lock();

        if human_readable {
            writeln!(out, "{:<8} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12}",
                "", "total", "used", "free", "shared", "buff/cache", "available")?;
            writeln!(out, "{:<8} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12}",
                "Mem:",
                human_size_kb(meminfo.mem_total),
                human_size_kb(meminfo.mem_total - meminfo.mem_available),
                human_size_kb(meminfo.mem_free),
                human_size_kb(meminfo.mem_shared),
                human_size_kb(meminfo.buffers + meminfo.cached),
                human_size_kb(meminfo.mem_available))?;
            writeln!(out, "{:<8} {:>12} {:>12} {:>12}",
                "Swap:",
                human_size_kb(meminfo.swap_total),
                human_size_kb(meminfo.swap_total - meminfo.swap_free),
                human_size_kb(meminfo.swap_free))?;
        } else {
            writeln!(out, "{:<8} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12}",
                "", "total", "used", "free", "shared", "buff/cache", "available")?;
            writeln!(out, "{:<8} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12}",
                "Mem:",
                meminfo.mem_total,
                meminfo.mem_total - meminfo.mem_available,
                meminfo.mem_free,
                meminfo.mem_shared,
                meminfo.buffers + meminfo.cached,
                meminfo.mem_available)?;
            writeln!(out, "{:<8} {:>12} {:>12} {:>12}",
                "Swap:",
                meminfo.swap_total,
                meminfo.swap_total - meminfo.swap_free,
                meminfo.swap_free)?;
        }

        Ok(0)
    }

    #[cfg(windows)]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let stdout = io::stdout();
        let mut out = stdout.lock();
        writeln!(out, "{:<8} {:>12} {:>12} {:>12}", "", "total", "used", "free")?;

        let (total, avail) = get_windows_memory()?;
        let used = total.saturating_sub(avail);
        writeln!(out, "{:<8} {:>12} {:>12} {:>12}",
            "Mem:",
            human_size_kb(total / 1024),
            human_size_kb(used / 1024),
            human_size_kb(avail / 1024))?;

        Ok(0)
    }

    #[cfg(not(any(unix, windows)))]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        eprintln!("free: not supported on this platform");
        Ok(1)
    }

    fn help(&self) {
        println!("Usage: free [OPTIONS]");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -h, --human-readable  Print sizes in human readable format");
    }
}

#[cfg(unix)]
struct MemInfo {
    mem_total: u64,
    mem_free: u64,
    mem_available: u64,
    mem_shared: u64,
    buffers: u64,
    cached: u64,
    swap_total: u64,
    swap_free: u64,
}

#[cfg(unix)]
fn parse_meminfo() -> Result<MemInfo, io::Error> {
    use std::fs;
    use std::io::BufRead;
    let file = fs::File::open("/proc/meminfo")?;
    let reader = io::BufReader::new(file);

    let mut mem_total = 0u64;
    let mut mem_free = 0u64;
    let mut mem_available = 0u64;
    let mut mem_shared = 0u64;
    let mut buffers = 0u64;
    let mut cached = 0u64;
    let mut swap_total = 0u64;
    let mut swap_free = 0u64;

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let key = parts[0];
        let value: u64 = parts[1].parse().unwrap_or(0);

        match key {
            "MemTotal:" => mem_total = value,
            "MemFree:" => mem_free = value,
            "MemAvailable:" => mem_available = value,
            "Shmem:" => mem_shared = value,
            "Buffers:" => buffers = value,
            "Cached:" => cached = value,
            "SwapTotal:" => swap_total = value,
            "SwapFree:" => swap_free = value,
            _ => {}
        }
    }

    Ok(MemInfo {
        mem_total,
        mem_free,
        mem_available,
        mem_shared,
        buffers,
        cached,
        swap_total,
        swap_free,
    })
}

#[cfg(windows)]
fn get_windows_memory() -> Result<(u64, u64), io::Error> {
    let output = std::process::Command::new("wmic")
        .args(&["OS", "get", "TotalVisibleMemorySize,FreePhysicalMemory", "/Value"])
        .output()?;
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut total: u64 = 0;
    let mut free: u64 = 0;
    for line in stdout_str.lines() {
        if let Some(val) = line.strip_prefix("TotalVisibleMemorySize=") {
            total = val.trim().parse().unwrap_or(0) * 1024;
        }
        if let Some(val) = line.strip_prefix("FreePhysicalMemory=") {
            free = val.trim().parse().unwrap_or(0) * 1024;
        }
    }
    Ok((total, free))
}

fn human_size_kb(kb: u64) -> String {
    let bytes = kb * 1024;
    const UNITS: &[&str] = &["B", "K", "M", "G", "T"];
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
