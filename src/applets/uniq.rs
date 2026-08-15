use crate::core::Applet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

pub struct UniqApplet;

impl Applet for UniqApplet {
    fn name(&self) -> &'static str {
        "uniq"
    }

    fn description(&self) -> &'static str {
        "Report or omit repeated lines"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut show_count = false;
        let mut repeated_only = false;
        let mut unique_only = false;
        let mut ignore_case = false;
        let mut file: Option<&str> = None;

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "-h" | "--help" => {
                    self.help();
                    return Ok(0);
                }
                "-c" | "--count" => show_count = true,
                "-d" | "--repeated" => repeated_only = true,
                "-u" | "--unique" => unique_only = true,
                "-i" | "--ignore-case" => ignore_case = true,
                "--" => {
                    i += 1;
                    if i < args.len() {
                        file = Some(&args[i]);
                    }
                    break;
                }
                _ if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") => {
                    for ch in arg[1..].chars() {
                        match ch {
                            'c' => show_count = true,
                            'd' => repeated_only = true,
                            'u' => unique_only = true,
                            'i' => ignore_case = true,
                            _ => return Err(format!("uniq: invalid option -- '{}'", ch).into()),
                        }
                    }
                }
                _ => {
                    if file.is_none() {
                        file = Some(arg);
                    }
                }
            }
            i += 1;
        }

        let lines = match file {
            Some("-") | None => Self::read_stdin()?,
            Some(path) => Self::read_file(path).map_err(|e| format!("uniq: {}: {}", path, e))?,
        };

        let stdout = io::stdout();
        let mut out = stdout.lock();

        if lines.is_empty() {
            return Ok(0);
        }

        let compare = |a: &str, b: &str| -> bool {
            if ignore_case {
                a.to_lowercase() == b.to_lowercase()
            } else {
                a == b
            }
        };

        let mut groups: Vec<(String, usize)> = Vec::new();
        let mut current = lines[0].clone();
        let mut count = 1usize;

        for line in &lines[1..] {
            if compare(&current, line) {
                count += 1;
            } else {
                groups.push((current, count));
                current = line.clone();
                count = 1;
            }
        }
        groups.push((current, count));

        for (line, count) in &groups {
            if repeated_only && *count < 2 {
                continue;
            }
            if unique_only && *count > 1 {
                continue;
            }
            if show_count {
                writeln!(out, "{:>7} {}", count, line)?;
            } else {
                writeln!(out, "{}", line)?;
            }
        }

        Ok(0)
    }

    fn help(&self) {
        println!("Usage: uniq [OPTION]... [INPUT [OUTPUT]]");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -c, --count        prefix lines by the number of occurrences");
        println!("  -d, --repeated     only print duplicate lines");
        println!("  -u, --unique       only print unique lines");
        println!("  -i, --ignore-case  ignore case when comparing");
        println!();
        println!("With no INPUT, or when INPUT is -, read standard input.");
    }
}

impl UniqApplet {
    fn read_file(path: &str) -> io::Result<Vec<String>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        reader.lines().collect::<io::Result<Vec<_>>>()
    }

    fn read_stdin() -> io::Result<Vec<String>> {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());
        reader.lines().collect::<io::Result<Vec<_>>>()
    }
}
