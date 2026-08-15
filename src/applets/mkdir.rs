use crate::core::{banner, Applet};
use std::fs;
use std::path::Path;

pub struct MkdirApplet;

impl Applet for MkdirApplet {
    fn name(&self) -> &'static str {
        "mkdir"
    }

    fn description(&self) -> &'static str {
        "Create directories"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut parents = false;
        let mut dirs: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "-p" | "--parents" => parents = true,
                "--" => {
                    dirs.extend(args[i + 1..].iter().map(|s| s.as_str()));
                    break;
                }
                _ if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") => {
                    for ch in arg[1..].chars() {
                        if ch == 'p' {
                            parents = true;
                        } else {
                            return Err(format!("mkdir: invalid option -- '{}'", ch).into());
                        }
                    }
                }
                _ => dirs.push(arg),
            }
            i += 1;
        }

        if dirs.is_empty() {
            self.print_usage();
            return Ok(1);
        }

        let mut had_error = false;

        for dir in &dirs {
            let path = Path::new(dir);
            let result = if parents {
                fs::create_dir_all(path)
            } else {
                fs::create_dir(path)
            };

            if let Err(e) = result {
                eprintln!("mkdir: cannot create directory '{}': {}", dir, e);
                had_error = true;
            }
        }

        if had_error {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    fn help(&self) {
        println!("{}", banner());
        println!();
        println!("Usage: mkdir [OPTION]... DIRECTORY...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -p, --parents    Create parent directories as needed; no error if existing");
    }
}

impl MkdirApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: mkdir [OPTION]... DIRECTORY...");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -p, --parents    Create parent directories as needed; no error if existing");
    }
}
