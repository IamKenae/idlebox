use crate::core::Applet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

pub struct GrepApplet;

impl Applet for GrepApplet {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Search for patterns in files or standard input"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut ignore_case = false;
        let mut invert_match = false;
        let mut show_line_number = false;
        let mut count_only = false;
        let mut pattern: Option<&str> = None;
        let mut files: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "-i" | "--ignore-case" => ignore_case = true,
                "-v" | "--invert-match" => invert_match = true,
                "-n" | "--line-number" => show_line_number = true,
                "-c" | "--count" => count_only = true,
                "--" => {
                    i += 1;
                    if i < args.len() && pattern.is_none() {
                        pattern = Some(&args[i]);
                        i += 1;
                    }
                    files.extend(args[i..].iter().map(|s| s.as_str()));
                    break;
                }
                _ if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") => {
                    for ch in arg[1..].chars() {
                        match ch {
                            'i' => ignore_case = true,
                            'v' => invert_match = true,
                            'n' => show_line_number = true,
                            'c' => count_only = true,
                            _ => return Err(format!("grep: invalid option -- '{}'", ch).into()),
                        }
                    }
                }
                _ => {
                    if pattern.is_none() {
                        pattern = Some(arg);
                    } else {
                        files.push(arg);
                    }
                }
            }
            i += 1;
        }

        let pattern = match pattern {
            Some(p) => p,
            None => {
                eprintln!("grep: missing pattern");
                return Ok(2);
            }
        };

        if files.is_empty() {
            files.push("-");
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();
        let multiple = files.len() > 1;
        let options = GrepOptions {
            pattern,
            ignore_case,
            invert_match,
            show_line_number,
            count_only,
            multiple,
        };
        let mut had_error = false;
        let mut total_matches = 0usize;

        for file in &files {
            let result = if *file == "-" {
                Self::grep_stdin(&mut out, &options, file)
            } else {
                Self::grep_file(&mut out, file, &options, file)
            };

            match result {
                Ok(count) => total_matches += count,
                Err(e) => {
                    eprintln!("grep: {}: {}", file, e);
                    had_error = true;
                }
            }
        }

        if had_error {
            Ok(2)
        } else if total_matches > 0 {
            Ok(0)
        } else {
            Ok(1)
        }
    }

    fn help(&self) {
        println!("Usage: grep [OPTION]... PATTERN [FILE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -i, --ignore-case    Ignore case distinctions");
        println!("  -v, --invert-match   Select non-matching lines");
        println!("  -n, --line-number    Prefix each line with 1-based line number");
        println!("  -c, --count          Only print a count of matching lines");
        println!();
        println!("With no FILE, or when FILE is -, read standard input.");
    }
}

struct GrepOptions<'a> {
    pattern: &'a str,
    ignore_case: bool,
    invert_match: bool,
    show_line_number: bool,
    count_only: bool,
    multiple: bool,
}

impl GrepApplet {
    fn grep_file(
        out: &mut impl Write,
        path: &str,
        options: &GrepOptions<'_>,
        file_label: &str,
    ) -> io::Result<usize> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::grep_reader(out, reader, options, file_label)
    }

    fn grep_stdin(
        out: &mut impl Write,
        options: &GrepOptions<'_>,
        file_label: &str,
    ) -> io::Result<usize> {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());
        Self::grep_reader(out, reader, options, file_label)
    }

    fn grep_reader<R: BufRead>(
        out: &mut impl Write,
        reader: R,
        options: &GrepOptions<'_>,
        file_label: &str,
    ) -> io::Result<usize> {
        let pattern_compare = if options.ignore_case {
            options.pattern.to_lowercase()
        } else {
            options.pattern.to_string()
        };

        let mut match_count = 0usize;

        for (idx, line_result) in reader.lines().enumerate() {
            let line = line_result?;
            let line_compare = if options.ignore_case {
                line.to_lowercase()
            } else {
                line.clone()
            };

            let matches = line_compare.contains(&pattern_compare);
            let should_print = if options.invert_match {
                !matches
            } else {
                matches
            };

            if should_print {
                match_count += 1;
                if !options.count_only {
                    if options.multiple {
                        write!(out, "{}:", file_label)?;
                    }
                    if options.show_line_number {
                        write!(out, "{}:", idx + 1)?;
                    }
                    writeln!(out, "{}", line)?;
                }
            }
        }

        if options.count_only {
            if options.multiple {
                write!(out, "{}:", file_label)?;
            }
            writeln!(out, "{}", match_count)?;
        }

        Ok(match_count)
    }
}
