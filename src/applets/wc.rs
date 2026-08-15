use crate::core::Applet;
use std::fs::File;
use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct WcApplet;

#[derive(Default, Clone)]
struct Counts {
    lines: usize,
    words: usize,
    bytes: usize,
    chars: usize,
}

#[derive(Clone, Copy)]
struct CountMode {
    lines: bool,
    words: bool,
    bytes: bool,
    chars: bool,
}

type WcParallelResult = (String, Result<Counts, io::Error>);

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
        let mut num_threads: Option<usize> = None;

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
                "-j" | "--threads" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("wc: missing argument for -j".into());
                    }
                    num_threads = Some(match args[i].parse::<usize>() {
                        Ok(n) if n > 0 => n,
                        _ => return Err(format!("wc: invalid thread count: {}", args[i]).into()),
                    });
                }
                "--" => {
                    i += 1;
                    files.extend(args[i..].iter().map(|s| s.as_str()));
                    break;
                }
                _ if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") => {
                    let mut chars = arg[1..].chars().peekable();
                    while let Some(ch) = chars.next() {
                        match ch {
                            'l' => show_lines = true,
                            'w' => show_words = true,
                            'c' => show_bytes = true,
                            'm' => show_chars = true,
                            'h' => {
                                self.help();
                                return Ok(0);
                            }
                            'j' => {
                                let rest: String = chars.collect();
                                if rest.is_empty() {
                                    i += 1;
                                    if i >= args.len() {
                                        return Err("wc: missing argument for -j".into());
                                    }
                                    num_threads = Some(match args[i].parse::<usize>() {
                                        Ok(n) if n > 0 => n,
                                        _ => {
                                            return Err(format!(
                                                "wc: invalid thread count: {}",
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
                                                "wc: invalid thread count: {}",
                                                rest
                                            )
                                            .into())
                                        }
                                    });
                                }
                                break;
                            }
                            _ => return Err(format!("wc: invalid option -- '{}'", ch).into()),
                        }
                    }
                }
                _ => files.push(arg),
            }
            i += 1;
        }

        if !show_lines && !show_words && !show_bytes && !show_chars {
            show_lines = true;
            show_words = true;
            show_bytes = true;
        }
        let mode = CountMode {
            lines: show_lines,
            words: show_words,
            bytes: show_bytes,
            chars: show_chars,
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
        let mut total = Counts::default();
        let mut had_error = false;

        let has_stdin = files.contains(&"-");
        let file_count = files.iter().filter(|f| **f != "-").count();

        // Use single-threaded path when:
        // - Only 0 or 1 regular files (parallelism overhead not worth it)
        // - stdin is present (must be read sequentially)
        // - User explicitly requested single thread
        if file_count <= 1 || has_stdin || num_threads <= 1 {
            for file in &files {
                let result = if *file == "-" {
                    Self::count_stdin(mode)
                } else {
                    Self::count_file(file, mode)
                };

                match result {
                    Ok(counts) => {
                        let label = if *file == "-" { None } else { Some(*file) };
                        Self::print_counts(&mut out, &counts, mode, label)?;
                        total.lines += counts.lines;
                        total.words += counts.words;
                        total.bytes += counts.bytes;
                        total.chars += counts.chars;
                    }
                    Err(error) => {
                        eprintln!("wc: {}: {}", file, error);
                        had_error = true;
                    }
                }
            }
        } else {
            let results = Self::count_parallel(&files, mode, num_threads);
            for (file, result) in results {
                match result {
                    Ok(counts) => {
                        Self::print_counts(&mut out, &counts, mode, Some(&file))?;
                        total.lines += counts.lines;
                        total.words += counts.words;
                        total.bytes += counts.bytes;
                        total.chars += counts.chars;
                    }
                    Err(error) => {
                        eprintln!("wc: {}: {}", file, error);
                        had_error = true;
                    }
                }
            }
        }

        if file_count > 1 {
            Self::print_counts(&mut out, &total, mode, Some("total"))?;
        }

        Ok(i32::from(had_error))
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
        println!("  -j, --threads N use N threads for parallel counting (default: auto)");
        println!();
        println!("With no FILE, or when FILE is -, read standard input.");
    }
}

impl WcApplet {
    const BUFFER_SIZE: usize = 8 * 1024;

    fn count_file(path: &str, mode: CountMode) -> io::Result<Counts> {
        let mut file = File::open(path)?;
        Self::count_reader(&mut file, mode)
    }

    fn count_stdin(mode: CountMode) -> io::Result<Counts> {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        Self::count_reader(&mut input, mode)
    }

    fn count_reader<R: Read>(reader: &mut R, mode: CountMode) -> io::Result<Counts> {
        let mut counts = Counts::default();
        let mut buffer = [0_u8; Self::BUFFER_SIZE];
        let mut pending_utf8 = Vec::with_capacity(Self::BUFFER_SIZE + 3);
        let mut in_word = false;

        loop {
            let read = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            let chunk = &buffer[..read];

            if mode.bytes {
                counts.bytes += read;
            }
            if mode.lines {
                counts.lines += chunk.iter().filter(|byte| **byte == b'\n').count();
            }
            if mode.words || mode.chars {
                pending_utf8.extend_from_slice(chunk);
                let processed =
                    Self::count_complete_utf8(&pending_utf8, &mut counts, &mut in_word, mode);
                let remaining = pending_utf8.len() - processed;
                pending_utf8.copy_within(processed.., 0);
                pending_utf8.truncate(remaining);
            }
        }

        if !pending_utf8.is_empty() {
            Self::count_char('\u{fffd}', &mut counts, &mut in_word, mode);
        }

        Ok(counts)
    }

    fn count_complete_utf8(
        bytes: &[u8],
        counts: &mut Counts,
        in_word: &mut bool,
        mode: CountMode,
    ) -> usize {
        let mut offset = 0;

        while offset < bytes.len() {
            match std::str::from_utf8(&bytes[offset..]) {
                Ok(text) => {
                    Self::count_text(text, counts, in_word, mode);
                    return bytes.len();
                }
                Err(error) => {
                    let valid_end = offset + error.valid_up_to();
                    if valid_end > offset {
                        let text = std::str::from_utf8(&bytes[offset..valid_end])
                            .expect("UTF-8 error reported a valid prefix");
                        Self::count_text(text, counts, in_word, mode);
                    }
                    offset = valid_end;

                    if let Some(error_len) = error.error_len() {
                        Self::count_char('\u{fffd}', counts, in_word, mode);
                        offset += error_len;
                    } else {
                        return offset;
                    }
                }
            }
        }

        offset
    }

    fn count_text(text: &str, counts: &mut Counts, in_word: &mut bool, mode: CountMode) {
        for ch in text.chars() {
            Self::count_char(ch, counts, in_word, mode);
        }
    }

    fn count_char(ch: char, counts: &mut Counts, in_word: &mut bool, mode: CountMode) {
        if mode.chars {
            counts.chars += 1;
        }
        if mode.words {
            if ch.is_whitespace() {
                *in_word = false;
            } else if !*in_word {
                counts.words += 1;
                *in_word = true;
            }
        }
    }

    fn print_counts(
        out: &mut impl Write,
        counts: &Counts,
        mode: CountMode,
        name: Option<&str>,
    ) -> io::Result<()> {
        if mode.lines {
            write!(out, "{:>7}", counts.lines)?;
        }
        if mode.words {
            write!(out, "{:>7}", counts.words)?;
        }
        if mode.bytes {
            write!(out, "{:>7}", counts.bytes)?;
        }
        if mode.chars {
            write!(out, "{:>7}", counts.chars)?;
        }
        if let Some(name) = name {
            write!(out, " {}", name)?;
        }
        writeln!(out)
    }

    #[allow(clippy::type_complexity)]
    fn count_parallel(
        files: &[&str],
        mode: CountMode,
        num_threads: usize,
    ) -> Vec<WcParallelResult> {
        let files_arc: Arc<Vec<String>> = Arc::new(files.iter().map(|s| s.to_string()).collect());
        let files_len = files.len();
        let file_indices: Vec<usize> = (0..files_len).collect();
        let file_indices = Arc::new(Mutex::new(file_indices.into_iter()));

        let mut handles = Vec::new();
        let (tx, rx) = mpsc::channel();

        for _ in 0..num_threads.min(files_len) {
            let file_indices = Arc::clone(&file_indices);
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
                let result = Self::count_file(file, mode);
                tx.send((idx, file.clone(), result)).ok();
            });
            handles.push(handle);
        }

        drop(tx);

        let mut results: Vec<(usize, String, Result<Counts, io::Error>)> = rx.iter().collect();

        let mut had_panic = false;
        for handle in handles {
            if let Err(e) = handle.join() {
                eprintln!("wc: worker thread panicked: {:?}", e);
                had_panic = true;
            }
        }

        if had_panic && results.len() < files_len {
            eprintln!(
                "wc: warning: only processed {} of {} files due to thread panic",
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
}

#[cfg(test)]
mod tests {
    use super::{CountMode, WcApplet};
    use std::io::{self, Read};

    struct ChunkedReader<'a> {
        input: &'a [u8],
        offset: usize,
        chunk_size: usize,
    }

    impl Read for ChunkedReader<'_> {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if self.offset == self.input.len() {
                return Ok(0);
            }
            let len = self
                .chunk_size
                .min(output.len())
                .min(self.input.len() - self.offset);
            output[..len].copy_from_slice(&self.input[self.offset..self.offset + len]);
            self.offset += len;
            Ok(len)
        }
    }

    #[test]
    fn streaming_counts_match_lossy_reference_for_all_chunk_boundaries() {
        let cases: &[&[u8]] = &[
            b"",
            b"one two\nthree\n",
            "你好 😊\nnext\tword".as_bytes(),
            &[0xff, b'a', b' ', 0xfe, b'\n'],
            &[0xe2, 0x82],
            &[0xf0, 0x9f, 0x98, 0x8a, 0xf0, 0x9f],
        ];
        let mode = CountMode {
            lines: true,
            words: true,
            bytes: true,
            chars: true,
        };

        for input in cases {
            let text = String::from_utf8_lossy(input);
            let expected_lines = text.chars().filter(|ch| *ch == '\n').count();
            let expected_words = text.split_whitespace().count();
            let expected_chars = text.chars().count();

            for chunk_size in 1..=7 {
                let mut reader = ChunkedReader {
                    input,
                    offset: 0,
                    chunk_size,
                };
                let counts = WcApplet::count_reader(&mut reader, mode).unwrap();
                assert_eq!(counts.lines, expected_lines, "chunk size {chunk_size}");
                assert_eq!(counts.words, expected_words, "chunk size {chunk_size}");
                assert_eq!(counts.bytes, input.len(), "chunk size {chunk_size}");
                assert_eq!(counts.chars, expected_chars, "chunk size {chunk_size}");
            }
        }
    }
}
