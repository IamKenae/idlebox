use crate::core::{banner, Applet};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

pub struct CutApplet;

enum Mode {
    Fields(Vec<Range>),
    Characters(Vec<Range>),
}

struct Range {
    start: usize,
    end: usize,
}

impl Applet for CutApplet {
    fn name(&self) -> &'static str {
        "cut"
    }

    fn description(&self) -> &'static str {
        "Remove sections from each line of files"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut delimiter = '\t';
        let mut mode: Option<Mode> = None;
        let mut files: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "-h" | "--help" => {
                    self.help();
                    return Ok(0);
                }
                "-d" | "--delimiter" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("cut: option requires an argument -- 'd'".into());
                    }
                    let d = &args[i];
                    delimiter = d.chars().next().ok_or("cut: invalid delimiter")?;
                }
                "-f" | "--fields" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("cut: option requires an argument -- 'f'".into());
                    }
                    let ranges = Self::parse_ranges(&args[i])?;
                    mode = Some(Mode::Fields(ranges));
                }
                "-c" | "--characters" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("cut: option requires an argument -- 'c'".into());
                    }
                    let ranges = Self::parse_ranges(&args[i])?;
                    mode = Some(Mode::Characters(ranges));
                }
                "--" => {
                    i += 1;
                    files.extend(args[i..].iter().map(|s| s.as_str()));
                    break;
                }
                _ if arg.starts_with("-d") && arg.len() > 2 => {
                    let d = &arg[2..];
                    delimiter = d.chars().next().ok_or("cut: invalid delimiter")?;
                }
                _ if arg.starts_with("-f") && arg.len() > 2 => {
                    let spec = &arg[2..];
                    let ranges = Self::parse_ranges(spec)?;
                    mode = Some(Mode::Fields(ranges));
                }
                _ if arg.starts_with("-c") && arg.len() > 2 => {
                    let spec = &arg[2..];
                    let ranges = Self::parse_ranges(spec)?;
                    mode = Some(Mode::Characters(ranges));
                }
                _ if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") => {
                    return Err(format!("cut: invalid option -- '{}'", &arg[1..]).into());
                }
                _ => files.push(arg),
            }
            i += 1;
        }

        let mode = match mode {
            Some(m) => m,
            None => {
                self.print_usage();
                return Ok(1);
            }
        };

        if files.is_empty() {
            files.push("-");
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();

        for file in &files {
            let result = if *file == "-" {
                Self::process_stdin(&mut out, &mode, delimiter)
            } else {
                Self::process_file(&mut out, file, &mode, delimiter)
            };

            if let Err(e) = result {
                eprintln!("cut: {}: {}", file, e);
                return Ok(1);
            }
        }

        Ok(0)
    }

    fn help(&self) {
        println!("{}", banner());
        println!();
        println!("Usage: cut [OPTION]... [FILE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -d, --delimiter=DELIM   use DELIM instead of TAB for field delimiter");
        println!("  -f, --fields=LIST       select only these fields");
        println!("  -c, --characters=LIST   select only these characters");
        println!();
        println!("With no FILE, or when FILE is -, read standard input.");
    }
}

impl CutApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: cut [OPTION]... [FILE]...");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -d, --delimiter=DELIM   use DELIM instead of TAB for field delimiter");
        eprintln!("  -f, --fields=LIST       select only these fields");
        eprintln!("  -c, --characters=LIST   select only these characters");
        eprintln!();
        eprintln!("With no FILE, or when FILE is -, read standard input.");
    }

    fn parse_ranges(spec: &str) -> Result<Vec<Range>, Box<dyn std::error::Error>> {
        let mut ranges = Vec::new();
        for part in spec.split(',') {
            let part = part.trim();
            if part.contains('-') {
                let mut parts = part.splitn(2, '-');
                let start_str = parts.next().unwrap_or("");
                let end_str = parts.next().unwrap_or("");
                let start: usize = if start_str.is_empty() {
                    1
                } else {
                    start_str
                        .parse::<usize>()
                        .map_err(|_| format!("cut: invalid field value -- '{}'", part))?
                };
                let end: usize = if end_str.is_empty() {
                    usize::MAX
                } else {
                    end_str
                        .parse::<usize>()
                        .map_err(|_| format!("cut: invalid field value -- '{}'", part))?
                };
                if start == 0 || end == 0 {
                    return Err("cut: fields and positions are numbered from 1".into());
                }
                ranges.push(Range { start, end });
            } else {
                let n: usize = part
                    .parse::<usize>()
                    .map_err(|_| format!("cut: invalid field value -- '{}'", part))?;
                if n == 0 {
                    return Err("cut: fields and positions are numbered from 1".into());
                }
                ranges.push(Range { start: n, end: n });
            }
        }
        Ok(ranges)
    }

    fn process_file(
        out: &mut impl Write,
        path: &str,
        mode: &Mode,
        delimiter: char,
    ) -> io::Result<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::process_reader(out, reader, mode, delimiter)
    }

    fn process_stdin(out: &mut impl Write, mode: &Mode, delimiter: char) -> io::Result<()> {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());
        Self::process_reader(out, reader, mode, delimiter)
    }

    fn process_reader<R: BufRead>(
        out: &mut impl Write,
        reader: R,
        mode: &Mode,
        delimiter: char,
    ) -> io::Result<()> {
        for line_result in reader.lines() {
            let line = line_result?;
            match mode {
                Mode::Fields(ranges) => {
                    if !line.contains(delimiter) {
                        writeln!(out, "{}", line)?;
                        continue;
                    }
                    let fields: Vec<&str> = line.split(delimiter).collect();
                    let mut selected: Vec<(usize, &str)> = Vec::new();
                    for range in ranges {
                        let start = range.start.saturating_sub(1);
                        let end = if range.end == usize::MAX {
                            fields.len()
                        } else {
                            range.end
                        };
                        for (idx, field) in fields
                            .iter()
                            .enumerate()
                            .take(end.min(fields.len()))
                            .skip(start)
                        {
                            if !selected.iter().any(|&(i, _)| i == idx) {
                                selected.push((idx, *field));
                            }
                        }
                    }
                    selected.sort_unstable_by_key(|&(i, _)| i);
                    let values: Vec<&str> = selected.into_iter().map(|(_, v)| v).collect();
                    writeln!(out, "{}", values.join(&delimiter.to_string()))?;
                }
                Mode::Characters(ranges) => {
                    let chars: Vec<char> = line.chars().collect();
                    let mut selected: Vec<(usize, char)> = Vec::new();
                    for range in ranges {
                        let start = range.start.saturating_sub(1);
                        let end = if range.end == usize::MAX {
                            chars.len()
                        } else {
                            range.end
                        };
                        for (idx, ch) in chars
                            .iter()
                            .enumerate()
                            .take(end.min(chars.len()))
                            .skip(start)
                        {
                            if !selected.iter().any(|&(i, _)| i == idx) {
                                selected.push((idx, *ch));
                            }
                        }
                    }
                    selected.sort_unstable_by_key(|&(i, _)| i);
                    writeln!(
                        out,
                        "{}",
                        selected.into_iter().map(|(_, c)| c).collect::<String>()
                    )?;
                }
            }
        }
        Ok(())
    }
}
