use crate::core::{banner, Applet};
use std::io::{self, Write};

pub struct BasenameApplet;

impl Applet for BasenameApplet {
    fn name(&self) -> &'static str {
        "basename"
    }

    fn description(&self) -> &'static str {
        "Strip directory and suffix from file names"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut multiple = false;
        let mut suffix: Option<&str> = None;
        let mut zero = false;
        let mut operands = Vec::new();
        let mut options_ended = false;
        let mut index = 0;

        while index < args.len() {
            let arg = args[index].as_str();
            if !options_ended {
                match arg {
                    "--" => {
                        options_ended = true;
                        index += 1;
                        continue;
                    }
                    "-a" | "--multiple" => {
                        multiple = true;
                        index += 1;
                        continue;
                    }
                    "-z" | "--zero" => {
                        zero = true;
                        index += 1;
                        continue;
                    }
                    "-s" | "--suffix" => {
                        index += 1;
                        if index == args.len() {
                            eprintln!("basename: option '{}' requires an argument", arg);
                            return Ok(1);
                        }
                        suffix = Some(&args[index]);
                        multiple = true;
                        index += 1;
                        continue;
                    }
                    option if option.starts_with("--suffix=") => {
                        suffix = Some(&option[9..]);
                        multiple = true;
                        index += 1;
                        continue;
                    }
                    option if option.starts_with('-') && option != "-" => {
                        eprintln!("basename: invalid option -- '{}'", option);
                        return Ok(1);
                    }
                    _ => {}
                }
            }

            operands.push(arg);
            index += 1;
        }

        if operands.is_empty() {
            self.print_usage();
            return Ok(1);
        }

        if !multiple {
            if operands.len() > 2 {
                eprintln!("basename: extra operand '{}'", operands[2]);
                return Ok(1);
            }
            if operands.len() == 2 {
                suffix = Some(operands[1]);
                operands.truncate(1);
            }
        }

        let separator = if zero { b'\0' } else { b'\n' };
        let stdout = io::stdout();
        let mut out = stdout.lock();
        for operand in operands {
            let mut result = basename(operand);
            if let Some(suffix) = suffix {
                if result.len() > suffix.len() && result.ends_with(suffix) {
                    result.truncate(result.len() - suffix.len());
                }
            }
            out.write_all(result.as_bytes())?;
            out.write_all(&[separator])?;
        }
        Ok(0)
    }

    fn help(&self) {
        println!("{}", banner());
        println!();
        println!("Usage: basename NAME [SUFFIX]");
        println!("       basename OPTION... NAME...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -a, --multiple       Process every NAME");
        println!("  -s, --suffix=SUFFIX  Remove a trailing SUFFIX; implies -a");
        println!("  -z, --zero           End each output with NUL, not newline");
    }
}

impl BasenameApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: basename NAME [SUFFIX]");
        eprintln!("       basename OPTION... NAME...");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -a, --multiple       Process every NAME");
        eprintln!("  -s, --suffix=SUFFIX  Remove a trailing SUFFIX; implies -a");
        eprintln!("  -z, --zero           End each output with NUL, not newline");
    }
}

fn basename(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    if is_windows_drive_root(path) {
        return preferred_root(path).to_string();
    }

    let trimmed = path.trim_end_matches(is_separator);
    if trimmed.is_empty() {
        return preferred_root(path).to_string();
    }

    trimmed
        .rsplit_once(is_separator)
        .map_or(trimmed, |(_, name)| name)
        .to_string()
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
    use super::basename;

    #[test]
    fn strips_directories_and_trailing_slashes() {
        assert_eq!(basename("/usr/bin/"), "bin");
        assert_eq!(basename("file"), "file");
        assert_eq!(basename("/"), "/");
        assert_eq!(basename(""), "");
    }

    #[test]
    #[cfg(windows)]
    fn handles_windows_drive_root() {
        assert_eq!(basename(r"C:\"), r"\");
    }
}
