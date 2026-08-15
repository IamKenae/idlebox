use crate::core::Applet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

pub struct SortApplet;

impl Applet for SortApplet {
    fn name(&self) -> &'static str {
        "sort"
    }

    fn description(&self) -> &'static str {
        "Sort lines of text files"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut reverse = false;
        let mut numeric = false;
        let mut unique = false;
        let mut files: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "-h" | "--help" => {
                    self.help();
                    return Ok(0);
                }
                "-r" | "--reverse" => reverse = true,
                "-n" | "--numeric-sort" => numeric = true,
                "-u" | "--unique" => unique = true,
                "--" => {
                    i += 1;
                    files.extend(args[i..].iter().map(|s| s.as_str()));
                    break;
                }
                _ if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") => {
                    for ch in arg[1..].chars() {
                        match ch {
                            'r' => reverse = true,
                            'n' => numeric = true,
                            'u' => unique = true,
                            _ => return Err(format!("sort: invalid option -- '{}'", ch).into()),
                        }
                    }
                }
                _ => files.push(arg),
            }
            i += 1;
        }

        if files.is_empty() {
            files.push("-");
        }

        let mut all_lines: Vec<String> = Vec::new();

        for file in &files {
            let result = if *file == "-" {
                Self::read_stdin()
            } else {
                Self::read_file(file)
            };

            match result {
                Ok(lines) => all_lines.extend(lines),
                Err(e) => {
                    eprintln!("sort: {}: {}", file, e);
                    return Ok(1);
                }
            }
        }

        if numeric {
            all_lines.sort_by(|a, b| {
                let na: f64 = a.trim().parse().unwrap_or(0.0);
                let nb: f64 = b.trim().parse().unwrap_or(0.0);
                na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            all_lines.sort();
        }

        if reverse {
            all_lines.reverse();
        }

        if unique {
            all_lines.dedup();
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();
        for line in &all_lines {
            writeln!(out, "{}", line)?;
        }

        Ok(0)
    }

    fn help(&self) {
        println!("Usage: sort [OPTION]... [FILE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -r, --reverse       reverse the result of comparisons");
        println!("  -n, --numeric-sort  compare according to string numerical value");
        println!("  -u, --unique        output only unique lines");
        println!();
        println!("With no FILE, or when FILE is -, read standard input.");
    }
}

impl SortApplet {
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
