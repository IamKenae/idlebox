use crate::core::Applet;
use std::ffi::CString;
use std::process::Command;

pub struct SuApplet;

impl Applet for SuApplet {
    fn name(&self) -> &'static str {
        "su"
    }

    fn description(&self) -> &'static str {
        "Run a shell with substitute user and group IDs"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut login_shell = false;
        let mut command: Option<&str> = None;
        let mut shell: Option<&str> = None;
        let mut user: Option<&str> = None;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-" | "-l" | "--login" => login_shell = true,
                "-c" | "--command" => {
                    i += 1;
                    if i < args.len() {
                        command = Some(&args[i]);
                    } else {
                        eprintln!("su: option '-c' requires an argument");
                        return Ok(1);
                    }
                }
                "-s" | "--shell" => {
                    i += 1;
                    if i < args.len() {
                        shell = Some(&args[i]);
                    } else {
                        eprintln!("su: option '-s' requires an argument");
                        return Ok(1);
                    }
                }
                _ if args[i].starts_with('-') && args[i] != "-" && args[i].len() > 1 => {
                    let mut combined = true;
                    let mut j = 1;
                    let chars: Vec<char> = args[i][1..].chars().collect();
                    while j <= chars.len() {
                        match chars[j - 1] {
                            'l' => {}
                            'c' => {
                                if j < chars.len() {
                                    combined = false;
                                    break;
                                }
                                i += 1;
                                if i < args.len() {
                                    command = Some(&args[i]);
                                } else {
                                    eprintln!("su: option '-c' requires an argument");
                                    return Ok(1);
                                }
                                j = chars.len() + 1;
                                continue;
                            }
                            's' => {
                                if j < chars.len() {
                                    combined = false;
                                    break;
                                }
                                i += 1;
                                if i < args.len() {
                                    shell = Some(&args[i]);
                                } else {
                                    eprintln!("su: option '-s' requires an argument");
                                    return Ok(1);
                                }
                                j = chars.len() + 1;
                                continue;
                            }
                            _ => {
                                combined = false;
                                break;
                            }
                        }
                        j += 1;
                    }
                    if combined {
                        for &ch in &chars {
                            if ch == 'l' {
                                login_shell = true;
                            }
                        }
                    } else {
                        eprintln!("su: invalid option -- '{}'", &args[i]);
                        return Ok(1);
                    }
                }
                _ => {
                    user = Some(&args[i]);
                }
            }
            i += 1;
        }

        let target_user = user.unwrap_or("root");

        let pw = match get_passwd_by_name(target_user) {
            Some(p) => p,
            None => {
                eprintln!("su: user '{}' does not exist", target_user);
                return Ok(1);
            }
        };

        let shell_path = match shell {
            Some(s) => s.to_string(),
            None => {
                if !pw.shell.is_empty() {
                    pw.shell.clone()
                } else {
                    "/bin/sh".to_string()
                }
            }
        };

        let current_uid = unsafe { raw_getuid() };

        if current_uid != 0 {
            eprintln!("su: permission denied (only root can switch user)");
            return Ok(1);
        }

        match command {
            Some(cmd) => {
                let mut child = Command::new(&shell_path);
                child.arg("-c").arg(cmd);

                if login_shell {
                    child.env("HOME", &pw.dir);
                    child.env("USER", target_user);
                    child.env("SHELL", &shell_path);
                    child.env("LOGNAME", target_user);
                    if pw.uid == 0 {
                        child.env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin");
                    } else {
                        child.env("PATH", "/usr/local/bin:/usr/bin:/bin");
                    }
                }

                child.env("HOME", &pw.dir);
                child.env("USER", target_user);
                child.env("SHELL", &shell_path);
                child.env("LOGNAME", target_user);

                match child.status() {
                    Ok(status) => Ok(status.code().unwrap_or(1)),
                    Err(e) => {
                        eprintln!("su: failed to execute '{}': {}", shell_path, e);
                        Ok(1)
                    }
                }
            }
            None => {
                let mut child = Command::new(&shell_path);

                if login_shell {
                    child.arg("-l");
                }

                child.env("HOME", &pw.dir);
                child.env("USER", target_user);
                child.env("SHELL", &shell_path);
                child.env("LOGNAME", target_user);

                match child.status() {
                    Ok(status) => Ok(status.code().unwrap_or(1)),
                    Err(e) => {
                        eprintln!("su: failed to execute '{}': {}", shell_path, e);
                        Ok(1)
                    }
                }
            }
        }
    }

    fn help(&self) {
        println!("Usage: su [options] [USER]");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -, -l, --login    Make the shell a login shell");
        println!("  -c, --command     Pass COMMAND to the shell");
        println!("  -s, --shell       Use SHELL instead of the default in /etc/passwd");
        println!();
        println!("If no USER is given, default is root.");
        println!("Note: only root can switch to another user.");
    }
}

struct PasswdInfo {
    uid: u32,
    dir: String,
    shell: String,
}

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

fn get_passwd_by_name(name: &str) -> Option<PasswdInfo> {
    let c_name = CString::new(name).ok()?;
    let ptr = unsafe { raw_getpwnam(c_name.as_ptr()) };
    if ptr.is_null() {
        return None;
    }
    unsafe {
        Some(PasswdInfo {
            uid: (*ptr).pw_uid,
            dir: c_char_to_string((*ptr).pw_dir),
            shell: c_char_to_string((*ptr).pw_shell),
        })
    }
}

extern "C" {
    #[link_name = "getuid"]
    fn raw_getuid() -> u32;

    #[link_name = "getpwnam"]
    fn raw_getpwnam(name: *const i8) -> *const Passwd;
}
