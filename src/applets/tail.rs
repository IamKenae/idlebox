use crate::core::Applet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};

pub struct TailApplet;

impl Applet for TailApplet {
    fn name(&self) -> &'static str {
        "tail"
    }

    fn description(&self) -> &'static str {
        "Output the last part of files"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut line_count: Option<usize> = None;
        let mut byte_count: Option<usize> = None;
        let mut files: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "-n" | "--lines" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("tail: option requires an argument -- 'n'".into());
                    }
                    line_count = Some(args[i].parse().map_err(|_| {
                        format!("tail: invalid number of lines: '{}'", args[i])
                    })?);
                }
                "-c" | "--bytes" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("tail: option requires an argument -- 'c'".into());
                    }
                    byte_count = Some(args[i].parse().map_err(|_| {
                        format!("tail: invalid number of bytes: '{}'", args[i])
                    })?);
                }
                "--" => {
                    files.extend(args[i + 1..].iter().map(|s| s.as_str()));
                    break;
                }
                _ if arg.starts_with("--lines=") => {
                    let val = &arg["--lines=".len()..];
                    line_count = Some(val.parse().map_err(|_| {
                        format!("tail: invalid number of lines: '{}'", val)
                    })?);
                }
                _ if arg.starts_with("--bytes=") => {
                    let val = &arg["--bytes=".len()..];
                    byte_count = Some(val.parse().map_err(|_| {
                        format!("tail: invalid number of bytes: '{}'", val)
                    })?);
                }
                _ if arg.starts_with('-') && arg.len() > 1 => {
                    let mut chars = arg[1..].chars().peekable();
                    while let Some(ch) = chars.next() {
                        match ch {
                            'n' => {
                                let val: String = chars.collect();
                                if val.is_empty() {
                                    i += 1;
                                    if i >= args.len() {
                                        return Err("tail: option requires an argument -- 'n'".into());
                                    }
                                    line_count = Some(args[i].parse().map_err(|_| {
                                        format!("tail: invalid number of lines: '{}'", args[i])
                                    })?);
                                } else {
                                    line_count = Some(val.parse().map_err(|_| {
                                        format!("tail: invalid number of lines: '{}'", val)
                                    })?);
                                }
                                break;
                            }
                            'c' => {
                                let val: String = chars.collect();
                                if val.is_empty() {
                                    i += 1;
                                    if i >= args.len() {
                                        return Err("tail: option requires an argument -- 'c'".into());
                                    }
                                    byte_count = Some(args[i].parse().map_err(|_| {
                                        format!("tail: invalid number of bytes: '{}'", args[i])
                                    })?);
                                } else {
                                    byte_count = Some(val.parse().map_err(|_| {
                                        format!("tail: invalid number of bytes: '{}'", val)
                                    })?);
                                }
                                break;
                            }
                            _ => return Err(format!("tail: invalid option -- '{}'", ch).into()),
                        }
                    }
                }
                _ => files.push(arg),
            }
            i += 1;
        }

        let lines = line_count.unwrap_or(10);

        if files.is_empty() {
            files.push("-");
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();
        let multiple = files.len() > 1;
        let mut had_error = false;

        for file in &files {
            if multiple {
                if *file != "-" {
                    writeln!(out, "==> {} <==", file).ok();
                } else {
                    writeln!(out, "==> standard input <==").ok();
                }
            }

            let result = if *file == "-" {
                if let Some(bytes) = byte_count {
                    Self::tail_bytes_stdin(&mut out, bytes)
                } else {
                    Self::tail_lines_stdin(&mut out, lines)
                }
            } else {
                if let Some(bytes) = byte_count {
                    Self::tail_bytes_file(file, &mut out, bytes)
                } else {
                    Self::tail_lines_file(file, &mut out, lines)
                }
            };

            if let Err(e) = result {
                eprintln!("tail: error reading '{}': {}", file, e);
                had_error = true;
            }
        }

        if had_error { Ok(1) } else { Ok(0) }
    }

    fn help(&self) {
        println!("Usage: tail [OPTION]... [FILE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -n, --lines=N    Output the last N lines (default 10)");
        println!("  -c, --bytes=N    Output the last N bytes");
        println!();
        println!("With no FILE, or when FILE is -, read standard input.");
    }
}

impl TailApplet {
    fn tail_lines_file(path: &str, out: &mut impl Write, n: usize) -> io::Result<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::tail_lines_reader(reader, out, n)
    }

    fn tail_lines_stdin(out: &mut impl Write, n: usize) -> io::Result<()> {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());
        Self::tail_lines_reader(reader, out, n)
    }

    fn tail_lines_reader<R: BufRead>(reader: R, out: &mut impl Write, n: usize) -> io::Result<()> {
        let mut ring: Vec<String> = Vec::with_capacity(n);

        for line_result in reader.lines() {
            let l = line_result?;
            if ring.len() >= n {
                ring.remove(0);
            }
            ring.push(l);
        }

        for l in &ring {
            writeln!(out, "{}", l)?;
        }

        Ok(())
    }

    fn tail_bytes_file(path: &str, out: &mut impl Write, n: usize) -> io::Result<()> {
        let mut file = File::open(path)?;
        let file_size = file.metadata()?.len() as usize;

        if file_size <= n {
            io::copy(&mut file, out)?;
        } else {
            use std::io::Seek;
            file.seek(io::SeekFrom::End(-(n as i64)))?;
            io::copy(&mut file, out)?;
        }

        Ok(())
    }

    fn tail_bytes_stdin(out: &mut impl Write, n: usize) -> io::Result<()> {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;

        let start = if buf.len() > n { buf.len() - n } else { 0 };
        out.write_all(&buf[start..])?;

        Ok(())
    }
}
