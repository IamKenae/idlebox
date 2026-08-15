use crate::core::{banner, Applet};
use std::fs;
use std::io::{self, Write};

pub struct ReadlinkApplet;

impl Applet for ReadlinkApplet {
    fn name(&self) -> &'static str {
        "readlink"
    }

    fn description(&self) -> &'static str {
        "Print resolved symbolic links or canonical file names"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut canonicalize = false;
        let mut no_newline = false;
        let mut paths: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-f" | "--canonicalize" | "-e" => canonicalize = true,
                "-n" | "--no-newline" => no_newline = true,
                "-fn" | "-nf" | "-en" | "-ne" => {
                    for ch in args[i][1..].chars() {
                        match ch {
                            'f' | 'e' => canonicalize = true,
                            'n' => no_newline = true,
                            _ => {}
                        }
                    }
                }
                _ if args[i].starts_with('-') && args[i].len() > 1 => {
                    let mut combined = true;
                    for ch in args[i][1..].chars() {
                        match ch {
                            'f' | 'e' => canonicalize = true,
                            'n' => no_newline = true,
                            _ => {
                                combined = false;
                                break;
                            }
                        }
                    }
                    if !combined {
                        eprintln!("readlink: invalid option -- '{}'", &args[i][1..]);
                        return Ok(1);
                    }
                }
                _ => paths.push(&args[i]),
            }
            i += 1;
        }

        if paths.is_empty() {
            self.print_usage();
            return Ok(1);
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();
        let mut failed = false;

        for (idx, path) in paths.iter().enumerate() {
            let result = if canonicalize {
                fs::canonicalize(path).map(|p| p.to_string_lossy().to_string())
            } else {
                fs::read_link(path).map(|p| p.to_string_lossy().to_string())
            };

            match result {
                Ok(resolved) => {
                    if no_newline && idx == paths.len() - 1 {
                        write!(out, "{}", resolved)?;
                    } else {
                        writeln!(out, "{}", resolved)?;
                    }
                }
                Err(e) => {
                    eprintln!("readlink: {}: {}", path, e);
                    failed = true;
                }
            }
        }

        if failed {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    fn help(&self) {
        println!("{}", banner());
        println!();
        println!("Usage: readlink [OPTION]... FILE...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -f, -e, --canonicalize  Canonicalize by following every symlink");
        println!("  -n, --no-newline        Do not output the trailing newline");
    }
}

impl ReadlinkApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: readlink [OPTION]... FILE...");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -f, -e, --canonicalize  Canonicalize by following every symlink");
        eprintln!("  -n, --no-newline        Do not output the trailing newline");
    }
}
