use crate::core::{banner, Applet};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

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
        let mut num_threads: Option<usize> = None;

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "-i" | "--ignore-case" => ignore_case = true,
                "-v" | "--invert-match" => invert_match = true,
                "-n" | "--line-number" => show_line_number = true,
                "-c" | "--count" => count_only = true,
                "-j" | "--threads" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("grep: missing argument for -j".into());
                    }
                    num_threads = Some(match args[i].parse::<usize>() {
                        Ok(n) if n > 0 => n,
                        _ => return Err(format!("grep: invalid thread count: {}", args[i]).into()),
                    });
                }
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
                            'j' => {
                                let rest: String = chars.collect();
                                if rest.is_empty() {
                                    i += 1;
                                    if i >= args.len() {
                                        return Err("grep: missing argument for -j".into());
                                    }
                                    num_threads = Some(match args[i].parse::<usize>() {
                                        Ok(n) if n > 0 => n,
                                        _ => {
                                            return Err(format!(
                                                "grep: invalid thread count: {}",
                                                args[i]
                                            )
                                            .into())
                                        }
                                    });
                                } else {
                                    num_threads = Some(match rest.parse::<usize>() {
                                        Ok(n) if n > 0 => n,
                                        _ => {
                                            return Err(format!(
                                                "grep: invalid thread count: {}",
                                                rest
                                            )
                                            .into())
                                        }
                                    });
                                }
                                break;
                            }
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
                self.print_usage();
                return Ok(2);
            }
        };

        if files.is_empty() {
            files.push("-");
        }

        let num_threads = num_threads.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        });

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

        if files.len() == 1 || files.contains(&"-") || num_threads <= 1 {
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
        } else {
            let results = Self::grep_parallel(&files, &options, num_threads);
            for (file, result) in results {
                match result {
                    Ok((count, output_lines)) => {
                        total_matches += count;
                        for line in output_lines {
                            writeln!(out, "{}", line).ok();
                        }
                    }
                    Err(e) => {
                        eprintln!("grep: {}: {}", file, e);
                        had_error = true;
                    }
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
        println!("{}", banner());
        println!();
        println!("Usage: idlebox grep [OPTION]... PATTERN [FILE]...");
        println!();
        println!("Options:");
        println!("  -i, --ignore-case    Ignore case distinctions");
        println!("  -v, --invert-match   Select non-matching lines");
        println!("  -n, --line-number    Prefix each line with 1-based line number");
        println!("  -c, --count          Only print a count of matching lines");
        println!("  -j, --threads N      Use N threads for parallel search (default: auto)");
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
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: idlebox grep [OPTION]... PATTERN [FILE]...");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -i, --ignore-case    Ignore case distinctions");
        eprintln!("  -v, --invert-match   Select non-matching lines");
        eprintln!("  -n, --line-number    Prefix each line with 1-based line number");
        eprintln!("  -c, --count          Only print a count of matching lines");
        eprintln!("  -j, --threads N      Use N threads for parallel search (default: auto)");
        eprintln!();
        eprintln!("With no FILE, or when FILE is -, read standard input.");
    }

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

    fn matches_pattern(line: &str, pattern: &str, ignore_case: bool) -> bool {
        if ignore_case {
            line.to_lowercase().contains(&pattern.to_lowercase())
        } else {
            line.contains(pattern)
        }
    }

    fn grep_reader<R: BufRead>(
        out: &mut impl Write,
        reader: R,
        options: &GrepOptions<'_>,
        file_label: &str,
    ) -> io::Result<usize> {
        let mut match_count = 0usize;

        for (idx, line_result) in reader.lines().enumerate() {
            let line = line_result?;
            let matches = Self::matches_pattern(&line, options.pattern, options.ignore_case);
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

    #[allow(clippy::type_complexity)]
    fn grep_parallel(
        files: &[&str],
        options: &GrepOptions<'_>,
        num_threads: usize,
    ) -> Vec<GrepParallelResult> {
        let options = Arc::new(ParallelGrepOptions {
            pattern: options.pattern.to_string(),
            ignore_case: options.ignore_case,
            invert_match: options.invert_match,
            show_line_number: options.show_line_number,
            count_only: options.count_only,
            multiple: options.multiple,
        });

        let files_arc: Arc<Vec<String>> = Arc::new(files.iter().map(|s| s.to_string()).collect());
        let files_len = files.len();
        let file_indices: Vec<usize> = (0..files_len).collect();
        let file_indices = Arc::new(Mutex::new(file_indices.into_iter()));

        let mut handles = Vec::new();
        let (tx, rx) = mpsc::channel();

        for _ in 0..num_threads.min(files_len) {
            let file_indices = Arc::clone(&file_indices);
            let options = Arc::clone(&options);
            let files_arc = Arc::clone(&files_arc);
            let tx = tx.clone();

            let handle = thread::spawn(move || loop {
                let idx = {
                    let mut guard = file_indices.lock().unwrap_or_else(|e| e.into_inner());
                    guard.next()
                };

                let idx = match idx {
                    Some(i) => i,
                    None => break,
                };

                let file = &files_arc[idx];
                let result = Self::grep_file_to_strings(file, &options);
                tx.send((idx, file.clone(), result)).ok();
            });
            handles.push(handle);
        }

        drop(tx);

        let mut results: Vec<(usize, String, Result<(usize, Vec<String>), io::Error>)> =
            rx.iter().collect();

        let mut had_panic = false;
        for handle in handles {
            if let Err(e) = handle.join() {
                eprintln!("grep: worker thread panicked: {:?}", e);
                had_panic = true;
            }
        }

        if had_panic && results.len() < files_len {
            eprintln!(
                "grep: warning: only processed {} of {} files due to thread panic",
                results.len(),
                files_len
            );
        }

        results.sort_by_key(|(idx, _, _)| *idx);
        results
            .into_iter()
            .map(|(_, file, result)| (file, result))
            .collect()
    }

    fn grep_file_to_strings(
        path: &str,
        options: &ParallelGrepOptions,
    ) -> Result<(usize, Vec<String>), io::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut match_count = 0usize;
        let mut output_lines = Vec::new();

        for (idx, line_result) in reader.lines().enumerate() {
            let line = line_result?;
            let matches = Self::matches_pattern(&line, &options.pattern, options.ignore_case);
            let should_print = if options.invert_match {
                !matches
            } else {
                matches
            };

            if should_print {
                match_count += 1;
                if !options.count_only {
                    let mut output_line = String::new();
                    if options.multiple {
                        output_line.push_str(path);
                        output_line.push(':');
                    }
                    if options.show_line_number {
                        output_line.push_str(&(idx + 1).to_string());
                        output_line.push(':');
                    }
                    output_line.push_str(&line);
                    output_lines.push(output_line);
                }
            }
        }

        if options.count_only {
            let mut output_line = String::new();
            if options.multiple {
                output_line.push_str(path);
                output_line.push(':');
            }
            output_line.push_str(&match_count.to_string());
            output_lines.push(output_line);
        }

        Ok((match_count, output_lines))
    }
}

struct ParallelGrepOptions {
    pattern: String,
    ignore_case: bool,
    invert_match: bool,
    show_line_number: bool,
    count_only: bool,
    multiple: bool,
}

type GrepParallelResult = (String, Result<(usize, Vec<String>), io::Error>);
