use crate::core::Applet;
use std::collections::BTreeMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::process::{Command, ExitStatus};

pub struct EnvApplet;

impl Applet for EnvApplet {
    fn name(&self) -> &'static str {
        "env"
    }

    fn description(&self) -> &'static str {
        "Run a command in a modified environment"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut clear = false;
        let mut null = false;
        let mut unset = Vec::new();
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--" => {
                    index += 1;
                    break;
                }
                "-" | "-i" | "--ignore-environment" => clear = true,
                "-0" | "--null" => null = true,
                "-u" | "--unset" => {
                    index += 1;
                    if index == args.len() {
                        eprintln!("env: option '-u' requires an argument");
                        return Ok(1);
                    }
                    if !valid_name(&args[index]) {
                        eprintln!("env: invalid variable name '{}'", args[index]);
                        return Ok(1);
                    }
                    unset.push(args[index].as_str());
                }
                option if option.starts_with("--unset=") => {
                    let name = &option[8..];
                    if !valid_name(name) {
                        eprintln!("env: invalid variable name '{}'", name);
                        return Ok(1);
                    }
                    unset.push(name);
                }
                option if option.starts_with('-') && option != "-" => {
                    eprintln!("env: invalid option -- '{}'", option);
                    return Ok(1);
                }
                _ => break,
            }
            index += 1;
        }

        let mut assignments = Vec::new();
        while index < args.len() {
            let Some((name, value)) = args[index].split_once('=') else {
                break;
            };
            if !valid_name(name) {
                eprintln!("env: invalid variable name '{}'", name);
                return Ok(1);
            }
            assignments.push((name, value));
            index += 1;
        }

        let mut environment: BTreeMap<OsString, OsString> = if clear {
            BTreeMap::new()
        } else {
            env::vars_os().collect()
        };
        for name in unset {
            environment.remove(OsStr::new(name));
        }
        for (name, value) in assignments {
            environment.insert(OsString::from(name), OsString::from(value));
        }

        if index == args.len() {
            return print_environment(&environment, null).map_err(Into::into);
        }

        let command_name = &args[index];
        let mut command = Command::new(command_name);
        command
            .args(&args[index + 1..])
            .env_clear()
            .envs(&environment);

        match command.status() {
            Ok(status) => Ok(exit_status_code(status)),
            Err(error) => {
                eprintln!("env: '{}': {}", command_name, error);
                Ok(match error.kind() {
                    io::ErrorKind::NotFound => 127,
                    _ => 126,
                })
            }
        }
    }

    fn help(&self) {
        println!("Usage: env [OPTION]... [-] [NAME=VALUE]... [COMMAND [ARG]...]");
        println!();
        println!("{}", self.description());
        println!("With no COMMAND, print the resulting environment.");
        println!();
        println!("Options:");
        println!("  -i, --ignore-environment  Start with an empty environment");
        println!("  -u, --unset=NAME          Remove NAME from the environment");
        println!("  -0, --null                End printed entries with NUL");
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('=') && !name.contains('\0')
}

fn print_environment(
    environment: &BTreeMap<OsString, OsString>,
    null: bool,
) -> Result<i32, io::Error> {
    let terminator = if null { b'\0' } else { b'\n' };
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for (name, value) in environment {
        write_os(&mut out, name)?;
        out.write_all(b"=")?;
        write_os(&mut out, value)?;
        out.write_all(&[terminator])?;
    }
    Ok(0)
}

fn write_os(output: &mut impl Write, value: &OsStr) -> io::Result<()> {
    output.write_all(value.to_string_lossy().as_bytes())
}

#[cfg(unix)]
fn exit_status_code(status: ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    status
        .code()
        .or_else(|| status.signal().map(|signal| 128 + signal))
        .unwrap_or(1)
}

#[cfg(not(unix))]
fn exit_status_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::valid_name;

    #[test]
    fn validates_environment_names() {
        assert!(valid_name("PATH"));
        assert!(valid_name("name.with.dots"));
        assert!(!valid_name(""));
        assert!(!valid_name("A=B"));
    }
}
