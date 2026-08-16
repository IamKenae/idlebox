use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};

pub(crate) fn hex_encode(data: &[u8]) -> String {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(data.len() * 2);
    for &byte in data {
        out.push(HEX_CHARS[(byte >> 4) as usize] as char);
        out.push(HEX_CHARS[(byte & 0xf) as usize] as char);
    }
    out
}

pub trait HashImpl {
    fn new() -> Self;
    fn update(&mut self, data: &[u8]);
    fn finalize(self) -> String; // Returns hex string

    fn hash_file(file: &str) -> io::Result<String>
    where
        Self: Sized,
    {
        let mut reader: Box<dyn Read> = if file == "-" {
            Box::new(io::stdin())
        } else {
            Box::new(File::open(file)?)
        };

        let mut hasher = Self::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        Ok(hasher.finalize())
    }
}

pub fn run_hash_applet<H: HashImpl>(
    applet_name: &str,
    args: &[String],
) -> Result<i32, Box<dyn std::error::Error>> {
    let mut check = false;
    let mut status = false;
    let mut binary = false;
    let mut files = Vec::new();

    let it = args.iter();
    let mut end_of_options = false;

    for arg in it {
        if end_of_options {
            files.push(arg.clone());
            continue;
        }

        match arg.as_str() {
            "--" => end_of_options = true,
            "-c" | "--check" => check = true,
            "--status" => status = true,
            "-b" | "--binary" => binary = true,
            "-t" | "--text" => (), // text is default on unix, binary/text distinction is minimal on linux
            _ if arg.starts_with('-') && arg != "-" => {
                for ch in arg.chars().skip(1) {
                    match ch {
                        'c' => check = true,
                        'b' => binary = true,
                        't' => (),
                        _ => {
                            eprintln!("{}: invalid option -- '{}'", applet_name, ch);
                            return Ok(1);
                        }
                    }
                }
            }
            _ => files.push(arg.clone()),
        }
    }

    if files.is_empty() {
        files.push("-".to_string());
    }

    let mut exit_code = 0;

    if check {
        for file in files {
            if !check_file::<H>(applet_name, &file, status)? {
                exit_code = 1;
            }
        }
    } else {
        for file in files {
            match H::hash_file(&file) {
                Ok(hash) => {
                    let mode = if binary { "*" } else { " " };
                    println!("{} {}{}", hash, mode, file);
                }
                Err(e) => {
                    eprintln!("{}: {}: {}", applet_name, file, e);
                    exit_code = 1;
                }
            }
        }
    }

    Ok(exit_code)
}

fn check_file<H: HashImpl>(applet_name: &str, file: &str, status: bool) -> io::Result<bool> {
    let reader: Box<dyn Read> = if file == "-" {
        Box::new(io::stdin())
    } else {
        Box::new(File::open(file)?)
    };

    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();
    let mut all_ok = true;
    let mut bad_format = 0;

    loop {
        line.clear();
        let n = buf_reader.read_line(&mut line)?;
        if n == 0 {
            break;
        }

        let line = line.trim_end_matches(&['\r', '\n'][..]);
        if line.is_empty() {
            continue;
        }

        let space_idx = line.find(' ');
        if space_idx.is_none() {
            bad_format += 1;
            continue;
        }
        let space_idx = space_idx.unwrap();
        let expected_hash = &line[..space_idx];

        let rem = &line[space_idx..];
        if !rem.starts_with("  ") && !rem.starts_with(" *") {
            bad_format += 1;
            continue;
        }
        let target_file = &rem[2..];

        match H::hash_file(target_file) {
            Ok(actual_hash) => {
                if expected_hash.eq_ignore_ascii_case(&actual_hash) {
                    if !status {
                        println!("{}: OK", target_file);
                    }
                } else {
                    if !status {
                        println!("{}: FAILED", target_file);
                    }
                    all_ok = false;
                }
            }
            Err(_) => {
                if !status {
                    println!("{}: FAILED open or read", target_file);
                }
                all_ok = false;
            }
        }
    }

    // GNU coreutils behavior: Malformed lines produce a warning but do not affect the exit code.
    // Only wrong hashes or missing files cause a non-zero exit code (failures > 0).
    if bad_format > 0 && !status {
        eprintln!(
            "{}: WARNING: {} lines are improperly formatted",
            applet_name, bad_format
        );
    }

    Ok(all_ok)
}
