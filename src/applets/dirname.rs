use crate::core::{banner, Applet};
use std::io::{self, Write};

pub struct DirnameApplet;

impl Applet for DirnameApplet {
    fn name(&self) -> &'static str {
        "dirname"
    }

    fn description(&self) -> &'static str {
        "Strip the last component from file names"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut zero = false;
        let mut operands = Vec::new();
        let mut options_ended = false;

        for arg in args {
            if !options_ended {
                match arg.as_str() {
                    "--" => {
                        options_ended = true;
                        continue;
                    }
                    "-z" | "--zero" => {
                        zero = true;
                        continue;
                    }
                    option if option.starts_with('-') && option != "-" => {
                        eprintln!("dirname: invalid option -- '{}'", option);
                        return Ok(1);
                    }
                    _ => {}
                }
            }
            operands.push(arg.as_str());
        }

        if operands.is_empty() {
            self.print_usage();
            return Ok(1);
        }

        let separator = if zero { b'\0' } else { b'\n' };
        let stdout = io::stdout();
        let mut out = stdout.lock();
        for operand in operands {
            out.write_all(dirname(operand).as_bytes())?;
            out.write_all(&[separator])?;
        }
        Ok(0)
    }

    fn help(&self) {
        println!("{}", banner());
        println!();
        println!("Usage: dirname [OPTION] NAME...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -z, --zero  End each output with NUL, not newline");
    }
}

impl DirnameApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: dirname [OPTION] NAME...");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -z, --zero  End each output with NUL, not newline");
    }
}

fn dirname(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }
    if is_windows_drive_root(path) {
        return path[..3].to_string();
    }

    let trimmed = path.trim_end_matches(is_separator);
    if trimmed.is_empty() {
        return preferred_root(path).to_string();
    }

    let Some((directory, _)) = trimmed.rsplit_once(is_separator) else {
        if is_windows_drive_relative(trimmed) {
            return trimmed[..2].to_string();
        }
        return ".".to_string();
    };
    let directory = directory.trim_end_matches(is_separator);
    if directory.is_empty() {
        if trimmed.starts_with(is_separator) {
            preferred_root(path).to_string()
        } else {
            ".".to_string()
        }
    } else if is_windows_drive_prefix(directory) {
        format!(
            "{}{}",
            directory,
            &trimmed[directory.len()..directory.len() + 1]
        )
    } else {
        directory.to_string()
    }
}

fn is_windows_drive_root(path: &str) -> bool {
    if !cfg!(windows) {
        return false;
    }
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
        && bytes[3..].iter().all(|byte| matches!(byte, b'/' | b'\\'))
}

fn is_windows_drive_prefix(path: &str) -> bool {
    cfg!(windows) && path.len() == 2 && path.as_bytes()[1] == b':'
}

fn is_windows_drive_relative(path: &str) -> bool {
    cfg!(windows) && path.len() > 2 && path.as_bytes()[1] == b':'
}

fn is_separator(character: char) -> bool {
    character == '/' || (cfg!(windows) && character == '\\')
}

fn preferred_root(path: &str) -> &'static str {
    if cfg!(windows) && path.contains('\\') && !path.contains('/') {
        "\\"
    } else {
        "/"
    }
}

#[cfg(test)]
mod tests {
    use super::dirname;

    #[test]
    fn strips_last_path_component() {
        assert_eq!(dirname("/usr/bin/"), "/usr");
        assert_eq!(dirname("file"), ".");
        assert_eq!(dirname("/file"), "/");
        assert_eq!(dirname("/"), "/");
        assert_eq!(dirname(""), ".");
    }

    #[test]
    #[cfg(windows)]
    fn handles_windows_drive_paths() {
        assert_eq!(dirname(r"C:\"), r"C:\");
        assert_eq!(dirname(r"C:\file"), r"C:\");
        assert_eq!(dirname(r"C:file"), "C:");
    }
}
