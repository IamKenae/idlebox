use crate::core::{banner, Applet};
use std::fs;
use std::path::Path;

pub struct RmApplet;

impl Applet for RmApplet {
    fn name(&self) -> &'static str {
        "rm"
    }

    fn description(&self) -> &'static str {
        "Remove files or directories"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut recursive = false;
        let mut force = false;
        let mut targets: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "-r" | "-R" | "--recursive" => recursive = true,
                "-f" | "--force" => force = true,
                "-rf" | "-fr" | "-Rf" | "-fR" => {
                    recursive = true;
                    force = true;
                }
                "--" => {
                    targets.extend(args[i + 1..].iter().map(|s| s.as_str()));
                    break;
                }
                _ if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") => {
                    for ch in arg[1..].chars() {
                        match ch {
                            'r' | 'R' => recursive = true,
                            'f' => force = true,
                            _ => {
                                eprintln!("rm: invalid option -- '{}'", ch);
                                return Ok(1);
                            }
                        }
                    }
                }
                _ => targets.push(arg),
            }
            i += 1;
        }

        if targets.is_empty() {
            if force {
                return Ok(0);
            }
            self.print_usage();
            return Ok(1);
        }

        let mut had_error = false;

        for target in &targets {
            let path = Path::new(target);

            let metadata = match path.symlink_metadata() {
                Ok(metadata) => metadata,
                Err(_) if force => continue,
                Err(_) => {
                    eprintln!("rm: cannot remove '{}': No such file or directory", target);
                    had_error = true;
                    continue;
                }
            };

            let result = if metadata.is_dir() {
                if !recursive {
                    eprintln!("rm: cannot remove '{}': Is a directory", target);
                    had_error = true;
                    continue;
                }
                fs::remove_dir_all(path)
            } else {
                fs::remove_file(path)
            };

            if let Err(e) = result {
                eprintln!("rm: cannot remove '{}': {}", target, e);
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
        println!("Usage: rm [OPTION]... FILE...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -r, -R, --recursive    Remove directories and their contents recursively");
        println!("  -f, --force            Ignore nonexistent files and arguments");
    }
}

impl RmApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: rm [OPTION]... FILE...");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -r, -R, --recursive    Remove directories and their contents recursively");
        eprintln!("  -f, --force            Ignore nonexistent files and arguments");
    }
}
