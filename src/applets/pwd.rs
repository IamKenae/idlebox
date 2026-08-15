use crate::core::Applet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub struct PwdApplet;

impl Applet for PwdApplet {
    fn name(&self) -> &'static str {
        "pwd"
    }

    fn description(&self) -> &'static str {
        "Print the current working directory"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut logical = true;
        let mut options_ended = false;

        for arg in args {
            if options_ended {
                eprintln!("pwd: extra operand '{}'", arg);
                return Ok(1);
            }

            match arg.as_str() {
                "--" => options_ended = true,
                "-L" | "--logical" => logical = true,
                "-P" | "--physical" => logical = false,
                option if option.starts_with('-') && option.len() > 1 => {
                    for flag in option[1..].chars() {
                        match flag {
                            'L' => logical = true,
                            'P' => logical = false,
                            _ => {
                                eprintln!("pwd: invalid option -- '{}'", flag);
                                return Ok(1);
                            }
                        }
                    }
                }
                operand => {
                    eprintln!("pwd: extra operand '{}'", operand);
                    return Ok(1);
                }
            }
        }

        let path = if logical {
            logical_working_directory().unwrap_or(env::current_dir()?)
        } else {
            fs::canonicalize(env::current_dir()?)?
        };

        let stdout = io::stdout();
        let mut out = stdout.lock();
        writeln!(out, "{}", path.display())?;
        Ok(0)
    }

    fn help(&self) {
        println!("Usage: pwd [-LP]");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -L, --logical   Use PWD and preserve symbolic links (default)");
        println!("  -P, --physical  Resolve symbolic links in the printed path");
    }
}

fn logical_working_directory() -> Option<PathBuf> {
    let logical = PathBuf::from(env::var_os("PWD")?);
    if !logical.is_absolute() || contains_dot_component(&logical) {
        return None;
    }

    let physical = env::current_dir().ok()?;
    if fs::canonicalize(&logical).ok()? == fs::canonicalize(physical).ok()? {
        Some(logical)
    } else {
        None
    }
}

fn contains_dot_component(path: &Path) -> bool {
    path.to_string_lossy()
        .split(|character| character == '/' || (cfg!(windows) && character == '\\'))
        .any(|component| component == "." || component == "..")
}

#[cfg(test)]
mod tests {
    use super::contains_dot_component;
    use std::path::Path;

    #[test]
    fn rejects_logical_paths_with_dot_components() {
        assert!(contains_dot_component(Path::new("/tmp/../var")));
        assert!(contains_dot_component(Path::new("/tmp/./file")));
        assert!(!contains_dot_component(Path::new("/tmp/file")));
    }
}
