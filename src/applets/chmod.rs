use crate::core::{banner, Applet};

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::Path;

pub struct ChmodApplet;

impl Applet for ChmodApplet {
    fn name(&self) -> &'static str {
        "chmod"
    }

    fn description(&self) -> &'static str {
        "Change file mode bits"
    }

    #[cfg(unix)]
    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut recursive = false;
        let mut mode_str: Option<&str> = None;
        let mut paths: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-R" | "--recursive" => recursive = true,
                _ if args[i].starts_with('-') && args[i].len() > 1 && mode_str.is_none() => {
                    let mut combined = true;
                    for ch in args[i][1..].chars() {
                        match ch {
                            'R' => {}
                            _ => {
                                combined = false;
                                break;
                            }
                        }
                    }
                    if combined {
                        recursive = true;
                    } else {
                        mode_str = Some(&args[i]);
                    }
                }
                _ if mode_str.is_none() => {
                    mode_str = Some(&args[i]);
                }
                _ => {
                    paths.push(&args[i]);
                }
            }
            i += 1;
        }

        let mode_str = match mode_str {
            Some(m) => m,
            None => {
                self.print_usage();
                return Ok(1);
            }
        };

        if paths.is_empty() {
            self.print_usage();
            return Ok(1);
        }

        let mode = match parse_mode(mode_str) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("chmod: {}", e);
                return Ok(1);
            }
        };

        let mut exit_code = 0;
        for path in &paths {
            if let Err(e) = apply_chmod(path, mode, recursive) {
                eprintln!("chmod: cannot access '{}': {}", path, e);
                exit_code = 1;
            }
        }

        Ok(exit_code)
    }

    #[cfg(not(unix))]
    fn run(&self, _args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        eprintln!("chmod: not supported on this platform");
        Ok(1)
    }

    fn help(&self) {
        println!("{}", banner());
        println!();
        println!("Usage: chmod [OPTION]... MODE[,MODE]... FILE...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -R, --recursive   Change files and directories recursively");
        println!();
        println!("MODE is an octal number (e.g. 755, 0644).");
        #[cfg(not(unix))]
        println!();
        #[cfg(not(unix))]
        println!("Note: this applet is not supported on this platform.");
    }
}

#[cfg(unix)]
impl ChmodApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: chmod [OPTION]... MODE[,MODE]... FILE...");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -R, --recursive   Change files and directories recursively");
        eprintln!();
        eprintln!("MODE is an octal number (e.g. 755, 0644).");
    }
}

#[cfg(unix)]
fn parse_mode(s: &str) -> Result<u32, String> {
    u32::from_str_radix(s, 8).map_err(|_| format!("invalid mode: '{}'", s))
}

#[cfg(unix)]
fn apply_chmod(path: &str, mode: u32, recursive: bool) -> Result<(), std::io::Error> {
    let p = Path::new(path);
    let metadata = fs::symlink_metadata(p)?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }

    let perms = fs::Permissions::from_mode(mode);
    fs::set_permissions(p, perms)?;

    if recursive && metadata.is_dir() {
        for entry in fs::read_dir(p)? {
            let entry = entry?;
            let entry_path = entry.path();
            let entry_str = entry_path.to_string_lossy().to_string();
            apply_chmod(&entry_str, mode, true)?;
        }
    }

    Ok(())
}
