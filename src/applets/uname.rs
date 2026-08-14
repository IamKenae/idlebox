use crate::core::Applet;
use std::io::{self, Write};

pub struct UnameApplet;

impl Applet for UnameApplet {
    fn name(&self) -> &'static str {
        "uname"
    }

    fn description(&self) -> &'static str {
        "Print system information"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut show_sysname = false;
        let mut show_nodename = false;
        let mut show_release = false;
        let mut show_version = false;
        let mut show_machine = false;
        let mut show_all = false;
        let mut has_flag = false;

        for arg in args {
            match arg.as_str() {
                "-a" | "--all" => {
                    show_all = true;
                    has_flag = true;
                }
                _ if arg.starts_with('-') && arg.len() > 1 => {
                    for ch in arg[1..].chars() {
                        has_flag = true;
                        match ch {
                            's' => show_sysname = true,
                            'n' => show_nodename = true,
                            'r' => show_release = true,
                            'v' => show_version = true,
                            'm' => show_machine = true,
                            'a' => show_all = true,
                            _ => {
                                eprintln!("uname: invalid option -- '{}'", ch);
                                return Ok(1);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if !has_flag {
            show_sysname = true;
        }

        let info = get_uname_info()?;

        let stdout = io::stdout();
        let mut out = stdout.lock();
        let mut parts: Vec<&str> = Vec::new();

        if show_all || show_sysname {
            parts.push(&info.sysname);
        }
        if show_all || show_nodename {
            parts.push(&info.nodename);
        }
        if show_all || show_release {
            parts.push(&info.release);
        }
        if show_all || show_version {
            parts.push(&info.version);
        }
        if show_all || show_machine {
            parts.push(&info.machine);
        }

        writeln!(out, "{}", parts.join(" "))?;

        Ok(0)
    }

    fn help(&self) {
        println!("Usage: uname [OPTION]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -a, --all       Print all information");
        println!("  -s, --sysname   Print the kernel name");
        println!("  -n, --nodename  Print the network node hostname");
        println!("  -r, --release   Print the kernel release");
        println!("  -v, --version   Print the kernel version");
        println!("  -m, --machine   Print the machine hardware name");
    }
}

struct UtsName {
    sysname: String,
    nodename: String,
    release: String,
    version: String,
    machine: String,
}

fn get_uname_info() -> Result<UtsName, Box<dyn std::error::Error>> {
    let mut buf: libc_utsname = unsafe { std::mem::zeroed() };
    let ret = unsafe { raw_uname(&mut buf) };
    if ret != 0 {
        return Err("uname failed".into());
    }
    Ok(UtsName {
        sysname: c_buf_to_string(&buf.sysname),
        nodename: c_buf_to_string(&buf.nodename),
        release: c_buf_to_string(&buf.release),
        version: c_buf_to_string(&buf.version),
        machine: c_buf_to_string(&buf.machine),
    })
}

fn c_buf_to_string(buf: &[i8; 65]) -> String {
    let bytes: Vec<u8> = buf.iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as u8)
        .collect();
    String::from_utf8_lossy(&bytes).to_string()
}

#[repr(C)]
struct libc_utsname {
    sysname: [i8; 65],
    nodename: [i8; 65],
    release: [i8; 65],
    version: [i8; 65],
    machine: [i8; 65],
    _domainname: [i8; 65],
}

extern "C" {
    #[link_name = "uname"]
    fn raw_uname(buf: *mut libc_utsname) -> i32;
}
