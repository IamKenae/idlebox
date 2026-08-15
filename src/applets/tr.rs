use crate::core::{banner, Applet};
use std::io::{self, Read, Write};

pub struct TrApplet;

impl Applet for TrApplet {
    fn name(&self) -> &'static str {
        "tr"
    }

    fn description(&self) -> &'static str {
        "Translate or delete characters"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut delete = false;
        let mut squeeze = false;
        let mut positional: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "-h" | "--help" => {
                    self.help();
                    return Ok(0);
                }
                "-d" | "--delete" => delete = true,
                "-s" | "--squeeze-repeats" => squeeze = true,
                "--" => {
                    i += 1;
                    positional.extend(args[i..].iter().map(|s| s.as_str()));
                    break;
                }
                _ if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") => {
                    for ch in arg[1..].chars() {
                        match ch {
                            'd' => delete = true,
                            's' => squeeze = true,
                            _ => return Err(format!("tr: invalid option -- '{}'", ch).into()),
                        }
                    }
                }
                _ => positional.push(arg),
            }
            i += 1;
        }

        if positional.is_empty() {
            self.print_usage();
            return Ok(1);
        }

        let set1 = Self::expand_set(positional[0]);

        let set2 = if positional.len() > 1 {
            Some(Self::expand_set(positional[1]))
        } else {
            None
        };

        if !delete && set2.is_none() && !squeeze {
            eprintln!("tr: missing operand after '{}'", positional[0]);
            return Ok(1);
        }

        if !delete && set2.as_ref().is_some_and(Vec::is_empty) {
            eprintln!("tr: SET2 must not be empty when translating");
            return Ok(1);
        }

        if delete && squeeze && set2.is_none() {
            eprintln!("tr: missing operand after '{}'", positional[0]);
            return Ok(1);
        }

        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut out = stdout.lock();

        let mut input = String::new();
        stdin.lock().read_to_string(&mut input)?;

        let mut result = String::new();
        let mut last_char: Option<char> = None;

        for ch in input.chars() {
            if delete && set1.contains(&ch) {
                continue;
            }

            let translated = if !delete {
                if let Some(ref s2) = set2 {
                    if let Some(pos) = set1.iter().position(|&candidate| candidate == ch) {
                        if pos < s2.len() {
                            s2[pos]
                        } else {
                            *s2.last().expect("non-empty SET2 was validated")
                        }
                    } else {
                        ch
                    }
                } else {
                    ch
                }
            } else {
                ch
            };

            let squeeze_set = set2.as_ref().unwrap_or(&set1);
            if squeeze && squeeze_set.contains(&translated) && last_char == Some(translated) {
                continue;
            }

            result.push(translated);
            last_char = Some(translated);
        }

        write!(out, "{}", result)?;
        Ok(0)
    }

    fn help(&self) {
        println!("{}", banner());
        println!();
        println!("Usage: tr [OPTION]... SET1 [SET2]");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -d, --delete          delete characters in SET1");
        println!("  -s, --squeeze-repeats  replace each sequence of a repeated character");
        println!();
        println!("SETs are specified as strings of characters. Ranges like 'a-z' are expanded.");
    }
}

impl TrApplet {
    fn print_usage(&self) {
        eprintln!("{}", banner());
        eprintln!();
        eprintln!("Usage: tr [OPTION]... SET1 [SET2]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -d, --delete          delete characters in SET1");
        eprintln!("  -s, --squeeze-repeats  replace each sequence of a repeated character");
        eprintln!();
        eprintln!("SETs are specified as strings of characters. Ranges like 'a-z' are expanded.");
    }

    fn expand_set(spec: &str) -> Vec<char> {
        let mut result = Vec::new();
        let chars: Vec<char> = spec.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if i + 2 < chars.len() && chars[i + 1] == '-' {
                let start = chars[i];
                let end = chars[i + 2];
                if start <= end {
                    for c in start..=end {
                        result.push(c);
                    }
                } else {
                    for c in (end..=start).rev() {
                        result.push(c);
                    }
                }
                i += 3;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }
        result
    }
}
