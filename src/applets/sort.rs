use crate::core::Applet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

pub struct SortApplet;

impl Applet for SortApplet {
    fn name(&self) -> &'static str {
        "sort"
    }

    fn description(&self) -> &'static str {
        "Sort lines of text files"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut reverse = false;
        let mut numeric = false;
        let mut unique = false;
        let mut files: Vec<&str> = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "-h" | "--help" => {
                    self.help();
                    return Ok(0);
                }
                "-r" | "--reverse" => reverse = true,
                "-n" | "--numeric-sort" => numeric = true,
                "-u" | "--unique" => unique = true,
                "--" => {
                    i += 1;
                    files.extend(args[i..].iter().map(|s| s.as_str()));
                    break;
                }
                _ if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") => {
                    for ch in arg[1..].chars() {
                        match ch {
                            'r' => reverse = true,
                            'n' => numeric = true,
                            'u' => unique = true,
                            _ => return Err(format!("sort: invalid option -- '{}'", ch).into()),
                        }
                    }
                }
                _ => files.push(arg),
            }
            i += 1;
        }

        if files.is_empty() {
            files.push("-");
        }

        let mut all_lines: Vec<String> = Vec::new();

        for file in &files {
            let result = if *file == "-" {
                Self::read_stdin()
            } else {
                Self::read_file(file)
            };

            match result {
                Ok(lines) => all_lines.extend(lines),
                Err(e) => {
                    eprintln!("sort: {}: {}", file, e);
                    return Ok(1);
                }
            }
        }

        if numeric {
            all_lines.sort_by(|a, b| numeric_cmp(a, b));
        } else {
            all_lines.sort();
        }

        if reverse {
            all_lines.reverse();
        }

        if unique {
            all_lines.dedup();
        }

        let stdout = io::stdout();
        let mut out = stdout.lock();
        for line in &all_lines {
            writeln!(out, "{}", line)?;
        }

        Ok(0)
    }

    fn help(&self) {
        println!("Usage: sort [OPTION]... [FILE]...");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -r, --reverse       reverse the result of comparisons");
        println!("  -n, --numeric-sort  compare according to string numerical value");
        println!("  -u, --unique        output only unique lines");
        println!();
        println!("With no FILE, or when FILE is -, read standard input.");
    }
}

#[derive(Clone, Copy)]
struct Decimal<'a> {
    negative: bool,
    integer: &'a [u8],
    fraction: &'a [u8],
}

impl<'a> Decimal<'a> {
    const ZERO: Self = Self {
        negative: false,
        integer: b"",
        fraction: b"",
    };

    fn parse(value: &'a str) -> Option<Self> {
        let mut bytes = value.trim().as_bytes();
        let mut negative = false;
        if let Some((&sign, rest)) = bytes.split_first() {
            if sign == b'-' || sign == b'+' {
                negative = sign == b'-';
                bytes = rest;
            }
        }

        let decimal = bytes.iter().position(|&byte| byte == b'.');
        let (integer, fraction) = match decimal {
            Some(index) => {
                let fraction = &bytes[index + 1..];
                if fraction.contains(&b'.') {
                    return None;
                }
                (&bytes[..index], fraction)
            }
            None => (bytes, &b""[..]),
        };

        if (integer.is_empty() && fraction.is_empty())
            || !integer.iter().chain(fraction).all(u8::is_ascii_digit)
        {
            return None;
        }

        let integer = integer
            .iter()
            .position(|&digit| digit != b'0')
            .map_or(&integer[integer.len()..], |index| &integer[index..]);
        let fraction_end = fraction
            .iter()
            .rposition(|&digit| digit != b'0')
            .map_or(0, |index| index + 1);
        let fraction = &fraction[..fraction_end];
        if integer.is_empty() && fraction.is_empty() {
            negative = false;
        }

        Some(Self {
            negative,
            integer,
            fraction,
        })
    }

    fn magnitude_cmp(self, other: Self) -> std::cmp::Ordering {
        self.integer
            .len()
            .cmp(&other.integer.len())
            .then_with(|| self.integer.cmp(other.integer))
            .then_with(|| {
                let digits = self.fraction.len().max(other.fraction.len());
                (0..digits)
                    .map(|index| {
                        (
                            self.fraction.get(index).copied().unwrap_or(b'0'),
                            other.fraction.get(index).copied().unwrap_or(b'0'),
                        )
                    })
                    .find_map(|(left, right)| (left != right).then(|| left.cmp(&right)))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}

fn numeric_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    let left = Decimal::parse(left).unwrap_or(Decimal::ZERO);
    let right = Decimal::parse(right).unwrap_or(Decimal::ZERO);

    match (left.negative, right.negative) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (true, true) => right.magnitude_cmp(left),
        (false, false) => left.magnitude_cmp(right),
    }
}

impl SortApplet {
    fn read_file(path: &str) -> io::Result<Vec<String>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        reader.lines().collect::<io::Result<Vec<_>>>()
    }

    fn read_stdin() -> io::Result<Vec<String>> {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());
        reader.lines().collect::<io::Result<Vec<_>>>()
    }
}

#[cfg(test)]
mod tests {
    use super::numeric_cmp;
    use std::cmp::Ordering;

    #[test]
    fn compares_decimal_numbers_without_float_conversion() {
        assert_eq!(numeric_cmp("-10", "-2"), Ordering::Less);
        assert_eq!(numeric_cmp("1.02", "1.2"), Ordering::Less);
        assert_eq!(numeric_cmp("001.200", "+1.2"), Ordering::Equal);
        assert_eq!(numeric_cmp(".5", "0.50"), Ordering::Equal);
        assert_eq!(numeric_cmp("999999999999999999999", "2"), Ordering::Greater);
        assert_eq!(numeric_cmp("not-a-number", "0"), Ordering::Equal);
    }
}
