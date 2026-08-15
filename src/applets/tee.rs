use crate::core::Applet;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};

pub struct TeeApplet;

struct TeeFile<'a> {
    name: &'a str,
    file: File,
    active: bool,
}

impl Applet for TeeApplet {
    fn name(&self) -> &'static str {
        "tee"
    }

    fn description(&self) -> &'static str {
        "Copy standard input to files and standard output"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut append = false;
        let mut ignore_interrupts = false;
        let mut paths = Vec::new();
        let mut options_ended = false;

        for arg in args {
            if !options_ended {
                match arg.as_str() {
                    "--" => {
                        options_ended = true;
                        continue;
                    }
                    "-a" | "--append" => {
                        append = true;
                        continue;
                    }
                    "-i" | "--ignore-interrupts" => {
                        ignore_interrupts = true;
                        continue;
                    }
                    option if option.starts_with('-') && option != "-" => {
                        let mut valid = true;
                        for flag in option[1..].chars() {
                            match flag {
                                'a' => append = true,
                                'i' => ignore_interrupts = true,
                                _ => valid = false,
                            }
                        }
                        if !valid {
                            eprintln!("tee: invalid option -- '{}'", option);
                            return Ok(1);
                        }
                        continue;
                    }
                    _ => {}
                }
            }
            paths.push(arg.as_str());
        }

        if ignore_interrupts {
            ignore_interrupt();
        }

        let mut failed = false;
        let mut files = Vec::new();
        for path in paths {
            match open_output(path, append) {
                Ok(file) => files.push(TeeFile {
                    name: path,
                    file,
                    active: true,
                }),
                Err(error) => {
                    eprintln!("tee: {}: {}", path, error);
                    failed = true;
                }
            }
        }

        let stdin = io::stdin();
        let mut input = stdin.lock();
        let stdout = io::stdout();
        let mut output = stdout.lock();
        let mut stdout_active = true;
        let mut buffer = [0_u8; 16 * 1024];

        loop {
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            let chunk = &buffer[..count];

            for destination in &mut files {
                if destination.active {
                    if let Err(error) = destination.file.write_all(chunk) {
                        eprintln!("tee: {}: {}", destination.name, error);
                        destination.active = false;
                        failed = true;
                    }
                }
            }

            if stdout_active {
                if let Err(error) = output.write_all(chunk) {
                    if error.kind() == io::ErrorKind::BrokenPipe {
                        stdout_active = false;
                    } else {
                        eprintln!("tee: standard output: {}", error);
                        stdout_active = false;
                        failed = true;
                    }
                }
            }
        }

        for destination in &mut files {
            if destination.active {
                if let Err(error) = destination.file.flush() {
                    eprintln!("tee: {}: {}", destination.name, error);
                    failed = true;
                }
            }
        }
        if stdout_active {
            output.flush()?;
        }

        Ok(i32::from(failed))
    }

    fn help(&self) {
        println!("Usage: tee [OPTION]... [FILE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -a, --append             Append instead of overwriting files");
        println!("  -i, --ignore-interrupts  Ignore interrupt signals");
    }
}

fn open_output(path: &str, append: bool) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).write(true);
    if append {
        options.append(true);
    } else {
        options.truncate(true);
    }
    options.open(path)
}

#[cfg(unix)]
fn ignore_interrupt() {
    const SIGINT: i32 = 2;
    const SIG_IGN: usize = 1;
    unsafe {
        raw_signal(SIGINT, SIG_IGN);
    }
}

#[cfg(unix)]
extern "C" {
    #[link_name = "signal"]
    fn raw_signal(signal: i32, handler: usize) -> usize;
}

#[cfg(not(unix))]
fn ignore_interrupt() {}
