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
                    let mut chars = arg[1..].chars().peekable();
                    while let Some(ch) = chars.next() {
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
        let mut had_error = false;
        let mut total_matches = 0usize;

        for file in &files {
            let result = if *file == "-" {
                Self::grep_stdin(
                    &mut out,
                    pattern,
                    ignore_case,
                    invert_match,
                    show_line_number,
                    count_only,
                    multiple,
                    *file,
                )
            } else {
                Self::grep_file(
                    &mut out,
                    file,
                    pattern,
                    ignore_case,
                    invert_match,
                    show_line_number,
                    count_only,
                    multiple,
                    file,
                )
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

impl GrepApplet {
    fn grep_file(
        out: &mut impl Write,
        path: &str,
        pattern: &str,
        ignore_case: bool,
        invert_match: bool,
        show_line_number: bool,
        count_only: bool,
        multiple: bool,
        file_label: &str,
    ) -> io::Result<usize> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::grep_reader(
            out,
            reader,
            pattern,
            ignore_case,
            invert_match,
            show_line_number,
            count_only,
            multiple,
            file_label,
        )
    }

    fn grep_stdin(
        out: &mut impl Write,
        pattern: &str,
        ignore_case: bool,
        invert_match: bool,
        show_line_number: bool,
        count_only: bool,
        multiple: bool,
        file_label: &str,
    ) -> io::Result<usize> {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());
        Self::grep_reader(
            out,
            reader,
            pattern,
            ignore_case,
            invert_match,
            show_line_number,
            count_only,
            multiple,
            file_label,
        )
    }

    fn grep_reader<R: BufRead>(
        out: &mut impl Write,
        reader: R,
        pattern: &str,
        ignore_case: bool,
        invert_match: bool,
        show_line_number: bool,
        count_only: bool,
        multiple: bool,
        file_label: &str,
    ) -> io::Result<usize> {
        let pattern_compare = if ignore_case {
            pattern.to_lowercase()
        } else {
            pattern.to_string()
        };

        let mut match_count = 0usize;

        for (idx, line_result) in reader.lines().enumerate() {
            let line = line_result?;
            let line_compare = if ignore_case {
                line.to_lowercase()
            } else {
                line.clone()
            };

            let matches = line_compare.contains(&pattern_compare);
            let should_print = if invert_match { !matches } else { matches };

            if should_print {
                match_count += 1;
                if !count_only {
                    if multiple {
                        write!(out, "{}:", file_label)?;
                    }
                    if show_line_number {
                        write!(out, "{}:", idx + 1)?;
                    }
                    writeln!(out, "{}", line)?;
                }
            }
        }

        if count_only {
            if multiple {
                write!(out, "{}:", file_label)?;
            }
            writeln!(out, "{}", match_count)?;
        }

        Ok(match_count)
    }
}
