use crate::core::Applet;
use std::env;
use std::ffi::OsStr;
use std::io::{self, Write};

pub struct PrintenvApplet;

impl Applet for PrintenvApplet {
    fn name(&self) -> &'static str {
        "printenv"
    }

    fn description(&self) -> &'static str {
        "Print all or part of the environment"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut null = false;
        let mut names = Vec::new();
        let mut options_ended = false;

        for arg in args {
            if !options_ended {
                match arg.as_str() {
                    "--" => {
                        options_ended = true;
                        continue;
                    }
                    "-0" | "--null" => {
                        null = true;
                        continue;
                    }
                    option if option.starts_with('-') && option != "-" => {
                        eprintln!("printenv: invalid option -- '{}'", option);
                        return Ok(1);
                    }
                    _ => {}
                }
            }
            names.push(arg.as_str());
        }

        let terminator = if null { b'\0' } else { b'\n' };
        let stdout = io::stdout();
        let mut out = stdout.lock();

        if names.is_empty() {
            for (name, value) in env::vars_os() {
                write_os(&mut out, &name)?;
                out.write_all(b"=")?;
                write_os(&mut out, &value)?;
                out.write_all(&[terminator])?;
            }
            return Ok(0);
        }

        let mut missing = false;
        for name in names {
            match env::var_os(name) {
                Some(value) => {
                    write_os(&mut out, &value)?;
                    out.write_all(&[terminator])?;
                }
                None => missing = true,
            }
        }
        Ok(i32::from(missing))
    }

    fn help(&self) {
        println!("Usage: printenv [OPTION]... [VARIABLE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -0, --null  End each output with NUL, not newline");
    }
}

fn write_os(output: &mut impl Write, value: &OsStr) -> io::Result<()> {
    output.write_all(value.to_string_lossy().as_bytes())
}
