use crate::core::Applet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

pub struct CatApplet;

impl Applet for CatApplet {
    fn name(&self) -> &'static str {
        "cat"
    }

    fn description(&self) -> &'static str {
        "Concatenate files and print to standard output"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut number_all = false;
        let mut number_nonblank = false;
        let mut show_ends = false;
        let mut files: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];
            if arg == "-n" {
                number_all = true;
            } else if arg == "-b" {
                number_nonblank = true;
            } else if arg == "-A" || arg == "-e" {
                show_ends = true;
            } else if arg == "--" {
                files.extend(args[i + 1..].iter().map(|s| s.as_str()));
                break;
            } else if arg.starts_with('-') && arg != "-" {
                return Err(format!("cat: invalid option -- '{}'", &arg[1..]).into());
            } else {
                files.push(arg);
            }
            i += 1;
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();
        let mut line_number = 0usize;

        if files.is_empty() {
            files.push("-");
        }

        for file in files {
            let result = if file == "-" {
                Self::cat_stdin(&mut out, &mut line_number, number_all, number_nonblank, show_ends)
            } else {
                Self::cat_file(file, &mut out, &mut line_number, number_all, number_nonblank, show_ends)
            };

            if let Err(e) = result {
                eprintln!("cat: {}: {}", file, e);
                return Ok(1);
            }
        }

        Ok(0)
    }

    fn help(&self) {
        println!("Usage: cat [OPTION] [FILE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -n    Number all output lines");
        println!("  -b    Number nonempty output lines");
        println!("  -A    Equivalent to -vET");
        println!("  -e    Equivalent to -vE");
        println!();
        println!("With no FILE, or when FILE is -, read standard input.");
    }
}

impl CatApplet {
    fn cat_stdin<W: Write>(
        out: &mut W,
        line_number: &mut usize,
        number_all: bool,
        number_nonblank: bool,
        show_ends: bool,
    ) -> io::Result<()> {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());
        Self::process_lines(reader, out, line_number, number_all, number_nonblank, show_ends)
    }

    fn cat_file<W: Write>(
        path: &str,
        out: &mut W,
        line_number: &mut usize,
        number_all: bool,
        number_nonblank: bool,
        show_ends: bool,
    ) -> io::Result<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::process_lines(reader, out, line_number, number_all, number_nonblank, show_ends)
    }

    fn process_lines<R: BufRead, W: Write>(
        reader: R,
        out: &mut W,
        line_number: &mut usize,
        number_all: bool,
        number_nonblank: bool,
        show_ends: bool,
    ) -> io::Result<()> {
        for line_result in reader.lines() {
            let line = line_result?;
            let is_empty = line.is_empty();

            let should_number = if number_nonblank {
                !is_empty
            } else if number_all {
                true
            } else {
                false
            };

            if should_number {
                *line_number += 1;
                write!(out, "{:>6}\t", line_number)?;
            }

            if show_ends {
                let processed = Self::process_visible(&line);
                writeln!(out, "{}$", processed)?;
            } else {
                writeln!(out, "{}", line)?;
            }
        }
        Ok(())
    }

    fn process_visible(line: &str) -> String {
        let mut result = String::with_capacity(line.len());
        for ch in line.chars() {
            match ch {
                '\t' => result.push_str("^I"),
                c if c.is_control() => {
                    let code = c as u32;
                    if code < 32 {
                        result.push('^');
                        result.push((code + 64) as u8 as char);
                    } else if code == 127 {
                        result.push_str("^?");
                    } else {
                        result.push(c);
                    }
                }
                c => result.push(c),
            }
        }
        result
    }
}
