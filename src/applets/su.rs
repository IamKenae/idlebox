use crate::core::Applet;
use std::ffi::CString;

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

        let shell_c = CString::new(shell_path.as_str()).map_err(|_| "invalid shell path")?;

        let mut exec_args: Vec<CString> = Vec::new();
        if login_shell {
            let login_name = format!("-{}", shell_path.rsplit('/').next().unwrap_or("sh"));
            exec_args.push(CString::new(login_name).unwrap());
        } else {
            exec_args.push(shell_c.clone());
        }

        if let Some(cmd) = command {
            exec_args.push(CString::new("-c").unwrap());
            exec_args.push(CString::new(cmd).unwrap());
        }

        let home_c = CString::new("HOME").unwrap();
        let user_c = CString::new("USER").unwrap();
        let shell_env_c = CString::new("SHELL").unwrap();
        let logname_c = CString::new("LOGNAME").unwrap();
        let path_c = CString::new("PATH").unwrap();

        let home_val = CString::new(pw.dir.as_str()).unwrap();
        let user_val = CString::new(target_user).unwrap();
        let shell_val = CString::new(shell_path.as_str()).unwrap();
        let path_val = if pw.uid == 0 {
            CString::new("/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin").unwrap()
        } else {
            CString::new("/usr/local/bin:/usr/bin:/bin").unwrap()
        };

        let pid = unsafe { raw_fork() };
        if pid < 0 {
            eprintln!("su: fork failed");
            return Ok(1);
        }

        if pid == 0 {
            if unsafe { raw_setgid(pw.gid) } != 0 {
                unsafe { raw__exit(1); }
            }
            if unsafe { raw_setuid(pw.uid) } != 0 {
                unsafe { raw__exit(1); }
            }

            unsafe {
                raw_setenv(home_c.as_ptr(), home_val.as_ptr(), 1);
                raw_setenv(user_c.as_ptr(), user_val.as_ptr(), 1);
                raw_setenv(shell_env_c.as_ptr(), shell_val.as_ptr(), 1);
                raw_setenv(logname_c.as_ptr(), user_val.as_ptr(), 1);
                if login_shell {
                    raw_setenv(path_c.as_ptr(), path_val.as_ptr(), 1);
                }
            }

            let mut c_argv: Vec<*const i8> = exec_args.iter().map(|a| a.as_ptr()).collect();
            c_argv.push(std::ptr::null());

            unsafe {
                raw_execvp(shell_c.as_ptr(), c_argv.as_ptr());
            }

            eprintln!("su: failed to execute '{}'", shell_path);
            unsafe { raw__exit(1); }
        }

        let mut status: i32 = 0;
        loop {
            let ret = unsafe { raw_waitpid(pid, &mut status, 0) };
            if ret < 0 {
                break;
            }
            if unsafe { raw_wifexited(status) } {
                return Ok(unsafe { raw_wexitstatus(status) });
            }
            return Ok(1);
        }

        Ok(1)
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
    gid: u32,
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
            gid: (*ptr).pw_gid,
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

    #[link_name = "fork"]
    fn raw_fork() -> i32;

    #[link_name = "setgid"]
    fn raw_setgid(gid: u32) -> i32;

    #[link_name = "setuid"]
    fn raw_setuid(uid: u32) -> i32;

    #[link_name = "execvp"]
    fn raw_execvp(file: *const i8, argv: *const *const i8) -> i32;

    #[link_name = "waitpid"]
    fn raw_waitpid(pid: i32, status: *mut i32, options: i32) -> i32;

    #[link_name = "setenv"]
    fn raw_setenv(name: *const i8, value: *const i8, overwrite: i32) -> i32;

    #[link_name = "_exit"]
    fn raw__exit(status: i32) -> !;
}

unsafe fn raw_wifexited(status: i32) -> bool {
    (status & 0x7f) == 0
}

unsafe fn raw_wexitstatus(status: i32) -> i32 {
    (status >> 8) & 0xff
}
