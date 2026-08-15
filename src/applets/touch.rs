use crate::core::{banner, Applet};
use std::fs::{File, FileTimes, OpenOptions};
use std::path::Path;
use std::time::SystemTime;

pub struct TouchApplet;

impl Applet for TouchApplet {
    fn name(&self) -> &'static str {
        "touch"
    }

    fn description(&self) -> &'static str {
        "Update file timestamps or create empty files"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut files: Vec<&str> = Vec::new();
        let mut parse_options = true;

        for arg in args {
            match arg.as_str() {
                "--" if parse_options => parse_options = false,
                _ if parse_options && arg.starts_with('-') && arg.len() > 1 => {
                    return Err(format!("touch: invalid option -- '{}'", &arg[1..]).into());
                }
                _ => files.push(arg),
            }
        }

        if files.is_empty() {
            self.print_usage();
            return Ok(1);
        }

        let now = SystemTime::now();
        let mut had_error = false;

        for file in &files {
            let path = Path::new(file);

            if path.exists() {
                if let Err(e) = Self::update_timestamps(path, now) {
                    eprintln!("touch: failed to update timestamps for '{}': {}", file, e);
                    had_error = true;
                }
            } else if let Err(e) = File::create(path) {
                eprintln!("touch: cannot touch '{}': {}", file, e);
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
        println!("Usage: touch [OPTION]... FILE...");
        println!();
        println!("{}", self.description());
        println!();
        println!("If a FILE does not exist, it is created as an empty file.");
        println!("If a FILE exists, its access and modification times are updated to now.");
    }
}

impl TouchApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: touch [OPTION]... FILE...");
        eprintln!();
        eprintln!("If a FILE does not exist, it is created as an empty file.");
        eprintln!("If a FILE exists, its access and modification times are updated to now.");
    }

    fn update_timestamps(path: &Path, time: SystemTime) -> std::io::Result<()> {
        let file = OpenOptions::new().write(true).open(path)?;
        let times = FileTimes::new().set_accessed(time).set_modified(time);
        file.set_times(times)
    }
}
