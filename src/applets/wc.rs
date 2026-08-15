use crate::core::Applet;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};

pub struct WcApplet;

struct Counts {
    lines: usize,
    words: usize,
    bytes: usize,
    chars: usize,
}

impl Applet for WcApplet {
    fn name(&self) -> &'static str {
        "wc"
    }

    fn description(&self) -> &'static str {
        "Print newline, word, and byte counts for each file"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut show_lines = false;
        let mut show_words = false;
        let mut show_bytes = false;
        let mut show_chars = false;
        let mut files: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "-h" | "--help" => {
                    self.help();
                    return Ok(0);
                }
                "-l" | "--lines" => show_lines = true,
                "-w" | "--words" => show_words = true,
                "-c" | "--bytes" => show_bytes = true,
                "-m" | "--chars" => show_chars = true,
                "--" => {
                    i += 1;
                    files.extend(args[i..].iter().map(|s| s.as_str()));
                    break;
                }
                _ if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") => {
                    for ch in arg[1..].chars() {
                        match ch {
                            'l' => show_lines = true,
                            'w' => show_words = true,
                            'c' => show_bytes = true,
                            'm' => show_chars = true,
                            _ => return Err(format!("wc: invalid option -- '{}'", ch).into()),
                        }
                    }
                }
                _ => files.push(arg),
            }
            i += 1;
        }

        let default_mode = !show_lines && !show_words && !show_bytes && !show_chars;
        if default_mode {
            show_lines = true;
            show_words = true;
            show_bytes = true;
        }

        if files.is_empty() {
            files.push("-");
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();
        let mut total = Counts {
            lines: 0,
            words: 0,
            bytes: 0,
            chars: 0,
        };
        let mut had_error = false;

        for file in &files {
            let result = if *file == "-" {
                Self::count_stdin()
            } else {
                Self::count_file(file)
            };

            match result {
                Ok(counts) => {
                    let label = if *file == "-" { None } else { Some(*file) };
                    Self::print_counts(
                        &mut out, &counts, show_lines, show_words, show_bytes, show_chars, label,
                    )?;
                    total.lines += counts.lines;
                    total.words += counts.words;
                    total.bytes += counts.bytes;
                    total.chars += counts.chars;
                }
                Err(e) => {
                    eprintln!("wc: {}: {}", file, e);
                    had_error = true;
                }
            }
        }

        if files.len() > 1 {
            let total_label: Option<&str> = Some("total");
            Self::print_counts(
                &mut out,
                &total,
                show_lines,
                show_words,
                show_bytes,
                show_chars,
                total_label,
            )?;
        }

        if had_error {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    fn help(&self) {
        println!("Usage: wc [OPTION]... [FILE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -l, --lines     print the newline counts");
        println!("  -w, --words     print the word counts");
        println!("  -c, --bytes     print the byte counts");
        println!("  -m, --chars     print the character counts");
        println!();
        println!("With no FILE, or when FILE is -, read standard input.");
    }
}

impl WcApplet {
    fn count_file(path: &str) -> io::Result<Counts> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        Self::count_reader(&mut reader)
    }

    fn count_stdin() -> io::Result<Counts> {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        Self::count_reader(&mut reader)
    }

    fn count_reader<R: Read>(reader: &mut R) -> io::Result<Counts> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;

        let bytes = buf.len();
        let text = String::from_utf8_lossy(&buf);
        let chars = text.chars().count();
        let lines = text.chars().filter(|&c| c == '\n').count();
        let words = text.split_whitespace().count();

        Ok(Counts {
            lines,
            words,
            bytes,
            chars,
        })
    }

    fn print_counts(
        out: &mut impl Write,
        counts: &Counts,
        show_lines: bool,
        show_words: bool,
        show_bytes: bool,
        show_chars: bool,
        name: Option<&str>,
    ) -> io::Result<()> {
        let mut parts: Vec<String> = Vec::new();
        if show_lines {
            parts.push(format!("{:>7}", counts.lines));
        }
        if show_words {
            parts.push(format!("{:>7}", counts.words));
        }
        if show_bytes {
            parts.push(format!("{:>7}", counts.bytes));
        }
        if show_chars {
            parts.push(format!("{:>7}", counts.chars));
        }
        if let Some(n) = name {
            writeln!(out, "{} {}", parts.join(""), n)?;
        } else {
            writeln!(out, "{}", parts.join(""))?;
        }
        Ok(())
    }
}
