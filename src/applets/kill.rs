use crate::core::Applet;
use std::io::{self, Write};

pub struct KillApplet;

impl Applet for KillApplet {
    fn name(&self) -> &'static str {
        "kill"
    }

    fn description(&self) -> &'static str {
        "Send signals to processes"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        if args.is_empty() {
            eprintln!("kill: usage: kill [-SIGNAL] PID... or kill -s SIGNAL PID...");
            return Ok(1);
        }

        let mut signal: Option<i32> = None;
        let mut list_signals = false;
        let mut pids: Vec<i32> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];
            if arg == "-l" || arg == "--list" {
                list_signals = true;
            } else if arg == "-s" {
                i += 1;
                if i >= args.len() {
                    eprintln!("kill: option '-s' requires an argument");
                    return Ok(1);
                }
                signal = Some(parse_signal(&args[i])?);
            } else if arg.starts_with("-s=") {
                let val = &arg[3..];
                signal = Some(parse_signal(val)?);
            } else if arg.starts_with('-') && arg.len() > 1 {
                let sig_part = &arg[1..];
                if let Ok(num) = sig_part.parse::<i32>() {
                    signal = Some(num);
                } else {
                    let upper = sig_part.to_uppercase();
                    let normalized = if upper.starts_with("SIG") {
                        upper.clone()
                    } else {
                        format!("SIG{}", upper)
                    };
                    signal = Some(signal_number_from_name(&normalized)?);
                }
            } else {
                match arg.parse::<i32>() {
                    Ok(pid) => pids.push(pid),
                    Err(_) => {
                        eprintln!("kill: invalid pid: {}", arg);
                        return Ok(1);
                    }
                }
            }
            i += 1;
        }

        if list_signals {
            let stdout = io::stdout();
            let mut out = stdout.lock();
            print_signal_list(&mut out)?;
            return Ok(0);
        }

        if pids.is_empty() {
            eprintln!("kill: no process ID specified");
            return Ok(1);
        }

        let sig = signal.unwrap_or(15);
        let mut failed = false;

        for pid in &pids {
            if let Err(e) = send_signal(*pid, sig) {
                eprintln!("kill: ({}) - {}", pid, e);
                failed = true;
            }
        }

        if failed {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    fn help(&self) {
        println!("Usage: kill [-SIGNAL] PID... or kill -s SIGNAL PID...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -SIGNAL       Signal number or name (e.g. -9, -TERM, -KILL)");
        println!("  -s SIGNAL     Specify signal by number or name");
        println!("  -l, --list    List available signals");
    }
}

fn parse_signal(s: &str) -> Result<i32, Box<dyn std::error::Error>> {
    if let Ok(num) = s.parse::<i32>() {
        return Ok(num);
    }
    let upper = s.to_uppercase();
    let name = if upper.starts_with("SIG") {
        upper
    } else {
        format!("SIG{}", upper)
    };
    signal_number_from_name(&name)
}

#[cfg(unix)]
fn signal_number_from_name(name: &str) -> Result<i32, Box<dyn std::error::Error>> {
    match name {
        "SIGHUP" => Ok(1),
        "SIGINT" => Ok(2),
        "SIGQUIT" => Ok(3),
        "SIGILL" => Ok(4),
        "SIGTRAP" => Ok(5),
        "SIGABRT" | "SIGIOT" => Ok(6),
        "SIGBUS" => Ok(7),
        "SIGFPE" => Ok(8),
        "SIGKILL" => Ok(9),
        "SIGUSR1" => Ok(10),
        "SIGSEGV" => Ok(11),
        "SIGUSR2" => Ok(12),
        "SIGPIPE" => Ok(13),
        "SIGALRM" => Ok(14),
        "SIGTERM" => Ok(15),
        "SIGSTKFLT" => Ok(16),
        "SIGCHLD" => Ok(17),
        "SIGCONT" => Ok(18),
        "SIGSTOP" => Ok(19),
        "SIGTSTP" => Ok(20),
        "SIGTTIN" => Ok(21),
        "SIGTTOU" => Ok(22),
        "SIGURG" => Ok(23),
        "SIGXCPU" => Ok(24),
        "SIGXFSZ" => Ok(25),
        "SIGVTALRM" => Ok(26),
        "SIGPROF" => Ok(27),
        "SIGWINCH" => Ok(28),
        "SIGIO" | "SIGPOLL" => Ok(29),
        "SIGPWR" => Ok(30),
        "SIGSYS" => Ok(31),
        _ => Err(format!("kill: unknown signal: {}", name).into()),
    }
}

#[cfg(windows)]
fn signal_number_from_name(name: &str) -> Result<i32, Box<dyn std::error::Error>> {
    match name {
        "SIGINT" => Ok(2),
        "SIGILL" => Ok(4),
        "SIGFPE" => Ok(8),
        "SIGSEGV" => Ok(11),
        "SIGTERM" => Ok(15),
        "SIGBREAK" => Ok(21),
        "SIGABRT" => Ok(22),
        _ => Err(format!("kill: unknown signal: {}", name).into()),
    }
}

#[cfg(not(any(unix, windows)))]
fn signal_number_from_name(name: &str) -> Result<i32, Box<dyn std::error::Error>> {
    Err(format!("kill: unknown signal: {}", name).into())
}

#[cfg(unix)]
fn signal_name(number: i32) -> &'static str {
    match number {
        1 => "HUP",
        2 => "INT",
        3 => "QUIT",
        4 => "ILL",
        5 => "TRAP",
        6 => "ABRT",
        7 => "BUS",
        8 => "FPE",
        9 => "KILL",
        10 => "USR1",
        11 => "SEGV",
        12 => "USR2",
        13 => "PIPE",
        14 => "ALRM",
        15 => "TERM",
        16 => "STKFLT",
        17 => "CHLD",
        18 => "CONT",
        19 => "STOP",
        20 => "TSTP",
        21 => "TTIN",
        22 => "TTOU",
        23 => "URG",
        24 => "XCPU",
        25 => "XFSZ",
        26 => "VTALRM",
        27 => "PROF",
        28 => "WINCH",
        29 => "IO",
        30 => "PWR",
        31 => "SYS",
        _ => "UNKNOWN",
    }
}

#[cfg(windows)]
fn signal_name(number: i32) -> &'static str {
    match number {
        2 => "INT",
        4 => "ILL",
        8 => "FPE",
        11 => "SEGV",
        15 => "TERM",
        21 => "BREAK",
        22 => "ABRT",
        _ => "UNKNOWN",
    }
}

#[cfg(not(any(unix, windows)))]
fn signal_name(number: i32) -> &'static str {
    let _ = number;
    "UNKNOWN"
}

#[cfg(unix)]
fn print_signal_list(out: &mut impl Write) -> Result<(), io::Error> {
    for i in 1..=31 {
        writeln!(out, "{:>2}) SIG{}", i, signal_name(i))?;
    }
    Ok(())
}

#[cfg(windows)]
fn print_signal_list(out: &mut impl Write) -> Result<(), io::Error> {
    for &i in &[2, 4, 8, 11, 15, 21, 22] {
        writeln!(out, "{:>2}) SIG{}", i, signal_name(i))?;
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn print_signal_list(_out: &mut impl Write) -> Result<(), io::Error> {
    Ok(())
}

#[cfg(unix)]
fn send_signal(pid: i32, sig: i32) -> Result<(), io::Error> {
    let ret = unsafe { raw_kill(pid, sig) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
fn send_signal(pid: i32, _sig: i32) -> Result<(), io::Error> {
    use std::process::Command;
    let output = Command::new("taskkill")
        .args(&["/PID", &pid.to_string(), "/F"])
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "taskkill failed"))
    }
}

#[cfg(not(any(unix, windows)))]
fn send_signal(_pid: i32, _sig: i32) -> Result<(), io::Error> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "signals not supported"))
}

#[cfg(unix)]
extern "C" {
    #[link_name = "kill"]
    fn raw_kill(pid: i32, sig: i32) -> i32;
}
