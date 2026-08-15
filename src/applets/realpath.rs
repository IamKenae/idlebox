use crate::core::{banner, Applet};
use std::fs;
use std::io::{self, Write};

pub struct RealpathApplet;

impl Applet for RealpathApplet {
    fn name(&self) -> &'static str {
        "realpath"
    }

    fn description(&self) -> &'static str {
        "Print canonical absolute path names"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut quiet = false;
        let mut zero = false;
        let mut paths = Vec::new();
        let mut options_ended = false;

        for arg in args {
            if !options_ended {
                match arg.as_str() {
                    "--" => {
                        options_ended = true;
                        continue;
                    }
                    "-e" | "--canonicalize-existing" => continue,
                    "-q" | "--quiet" => {
                        quiet = true;
                        continue;
                    }
                    "-z" | "--zero" => {
                        zero = true;
                        continue;
                    }
                    option if option.starts_with('-') && option != "-" => {
                        eprintln!("realpath: invalid option -- '{}'", option);
                        return Ok(1);
                    }
                    _ => {}
                }
            }
            paths.push(arg.as_str());
        }

        if paths.is_empty() {
            self.print_usage();
            return Ok(1);
        }

        let separator = if zero { b'\0' } else { b'\n' };
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let mut failed = false;

        for path in paths {
            match fs::canonicalize(path) {
                Ok(canonical) => {
                    write!(out, "{}", canonical.display())?;
                    out.write_all(&[separator])?;
                }
                Err(error) => {
                    if !quiet {
                        eprintln!("realpath: {}: {}", path, error);
                    }
                    failed = true;
                }
            }
        }

        Ok(i32::from(failed))
    }

    fn help(&self) {
        println!("{}", banner());
        println!();
        println!("Usage: realpath [OPTION]... FILE...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -e, --canonicalize-existing  Require every path component to exist");
        println!("  -q, --quiet                  Suppress diagnostics for invalid paths");
        println!("  -z, --zero                   End each output with NUL, not newline");
    }
}

impl RealpathApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: realpath [OPTION]... FILE...");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -e, --canonicalize-existing  Require every path component to exist");
        eprintln!("  -q, --quiet                  Suppress diagnostics for invalid paths");
        eprintln!("  -z, --zero                   End each output with NUL, not newline");
    }
}
