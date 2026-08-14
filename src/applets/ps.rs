use crate::core::Applet;
use std::fs;
use std::io::{self, Write};

pub struct PsApplet;

impl Applet for PsApplet {
    fn name(&self) -> &'static str {
        "ps"
    }

    fn description(&self) -> &'static str {
        "Report a snapshot of the current processes"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut show_all = false;
        let mut custom_cols: Option<Vec<String>> = None;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-e" | "-A" => show_all = true,
                "-o" => {
                    i += 1;
                    if i >= args.len() {
                        eprintln!("ps: option '-o' requires an argument");
                        return Ok(1);
                    }
                    custom_cols = Some(args[i].split(',').map(|s| s.trim().to_string()).collect());
                }
                _ if args[i].starts_with("-o=") => {
                    let val = &args[i][3..];
                    custom_cols = Some(val.split(',').map(|s| s.trim().to_string()).collect());
                }
                _ => {}
            }
            i += 1;
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();

        let default_cols = vec!["pid".to_string(), "tty".to_string(), "stat".to_string(), "time".to_string(), "cmd".to_string()];
        let cols_ref: &[String] = match &custom_cols {
            Some(ref c) => c.as_slice(),
            None => default_cols.as_slice(),
        };
        let col_vec: Vec<&str> = cols_ref.iter().map(|s| s.as_str()).collect();

        print_header(&mut out, &col_vec)?;

        let entries = match read_all_proc_entries(show_all) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("ps: failed to read /proc: {}", e);
                return Ok(1);
            }
        };

        for entry in &entries {
            print_entry(&mut out, entry, &col_vec)?;
        }

        Ok(0)
    }

    fn help(&self) {
        println!("Usage: ps [OPTIONS]");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -e, -A              Show all processes");
        println!("  -o COL1,COL2,...    Custom output columns (pid, tty, stat, time, cmd)");
    }
}

struct ProcEntry {
    pid: u32,
    tty: String,
    stat: String,
    time: String,
    cmd: String,
}

fn read_all_proc_entries(show_all: bool) -> Result<Vec<ProcEntry>, io::Error> {
    let mut entries = Vec::new();
    let my_pid = std::process::id();
    let my_ppid = get_ppid(my_pid).unwrap_or(0);

    for dir_entry in fs::read_dir("/proc")? {
        let dir_entry = match dir_entry {
            Ok(d) => d,
            Err(_) => continue,
        };
        let name = dir_entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let pid: u32 = match name_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        if !show_all && pid != my_pid && pid != my_ppid {
            let ppid = get_ppid(pid).unwrap_or(0);
            if ppid != my_pid && ppid != my_ppid {
                continue;
            }
        }

        let stat = match read_proc_stat(pid) {
            Some(s) => s,
            None => continue,
        };
        let cmd = read_proc_cmdline(pid).unwrap_or_else(|| stat.comm.clone());

        entries.push(ProcEntry {
            pid,
            tty: stat.tty,
            stat: stat.state,
            time: stat.cpu_time,
            cmd,
        });
    }

    entries.sort_by_key(|e| e.pid);
    Ok(entries)
}

struct ProcStat {
    comm: String,
    state: String,
    tty: String,
    cpu_time: String,
}

fn read_proc_stat(pid: u32) -> Option<ProcStat> {
    let path = format!("/proc/{}/stat", pid);
    let content = fs::read_to_string(&path).ok()?;

    let comm_start = content.find('(')?;
    let comm_end = content.rfind(')')?;
    let comm = content[comm_start + 1..comm_end].to_string();

    let after_comm = &content[comm_end + 2..];
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    if fields.len() < 12 {
        return None;
    }

    let state = fields[0].to_string();

    let tty_nr: u64 = fields[4].parse().unwrap_or(0);
    let tty = format_tty(tty_nr);

    let utime: u64 = fields[11].parse().unwrap_or(0);
    let stime: u64 = fields[12].parse().unwrap_or(0);
    let total_ticks = utime + stime;
    let total_secs = total_ticks / 100;
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    let cpu_time = format!("{:02}:{:02}", minutes, seconds);

    Some(ProcStat {
        comm,
        state,
        tty,
        cpu_time,
    })
}

fn format_tty(tty_nr: u64) -> String {
    if tty_nr == 0 {
        return "?".to_string();
    }
    let major = (tty_nr >> 8) & 0xff;
    let minor = tty_nr & 0xff;
    match major {
        4 => format!("tty{}", minor),
        136..=143 => format!("pts/{}", minor + (major - 136) * 256),
        _ => format!("{}/{}", major, minor),
    }
}

fn read_proc_cmdline(pid: u32) -> Option<String> {
    let path = format!("/proc/{}/cmdline", pid);
    let content = fs::read_to_string(&path).ok()?;
    if content.is_empty() {
        return None;
    }
    let cmd = content.replace('\0', " ").trim().to_string();
    if cmd.is_empty() {
        None
    } else {
        Some(cmd)
    }
}

fn get_ppid(pid: u32) -> Option<u32> {
    let path = format!("/proc/{}/stat", pid);
    let content = fs::read_to_string(&path).ok()?;
    let comm_end = content.rfind(')')?;
    let after_comm = &content[comm_end + 2..];
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    if fields.len() < 2 {
        return None;
    }
    fields[1].parse().ok()
}

fn print_header(out: &mut impl Write, cols: &[&str]) -> Result<(), io::Error> {
    let mut parts = Vec::new();
    for col in cols {
        match *col {
            "pid" => parts.push(format!("{:>8}", "PID")),
            "tty" => parts.push(format!("{:<8}", "TTY")),
            "stat" => parts.push(format!("{:<6}", "STAT")),
            "time" => parts.push(format!("{:>8}", "TIME")),
            "cmd" | "command" => parts.push("COMMAND".to_string()),
            _ => parts.push(col.to_uppercase()),
        }
    }
    writeln!(out, "{}", parts.join(" "))?;
    Ok(())
}

fn print_entry(out: &mut impl Write, entry: &ProcEntry, cols: &[&str]) -> Result<(), io::Error> {
    let mut parts = Vec::new();
    for col in cols {
        match *col {
            "pid" => parts.push(format!("{:>8}", entry.pid)),
            "tty" => parts.push(format!("{:<8}", entry.tty)),
            "stat" => parts.push(format!("{:<6}", entry.stat)),
            "time" => parts.push(format!("{:>8}", entry.time)),
            "cmd" | "command" => parts.push(entry.cmd.clone()),
            _ => parts.push(String::new()),
        }
    }
    writeln!(out, "{}", parts.join(" "))?;
    Ok(())
}
