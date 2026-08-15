use crate::core::{
    file_ops::{same_file, FollowSymlinks},
    Applet,
};
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;

pub struct UniqApplet;

#[derive(Clone, Copy)]
struct UniqOptions {
    show_count: bool,
    repeated_only: bool,
    unique_only: bool,
    ignore_case: bool,
}

impl Applet for UniqApplet {
    fn name(&self) -> &'static str {
        "uniq"
    }

    fn description(&self) -> &'static str {
        "Report or omit repeated lines"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut options = UniqOptions {
            show_count: false,
            repeated_only: false,
            unique_only: false,
            ignore_case: false,
        };
        let mut operands: [Option<&str>; 2] = [None, None];
        let mut operand_count = 0;

        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "-h" | "--help" => {
                    self.help();
                    return Ok(0);
                }
                "-c" | "--count" => options.show_count = true,
                "-d" | "--repeated" => options.repeated_only = true,
                "-u" | "--unique" => options.unique_only = true,
                "-i" | "--ignore-case" => options.ignore_case = true,
                "--" => {
                    for operand in &args[i + 1..] {
                        if operand_count == operands.len() {
                            return Err(format!("uniq: extra operand '{}'", operand).into());
                        }
                        operands[operand_count] = Some(operand);
                        operand_count += 1;
                    }
                    break;
                }
                _ if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") => {
                    for ch in arg[1..].chars() {
                        match ch {
                            'c' => options.show_count = true,
                            'd' => options.repeated_only = true,
                            'u' => options.unique_only = true,
                            'i' => options.ignore_case = true,
                            _ => return Err(format!("uniq: invalid option -- '{}'", ch).into()),
                        }
                    }
                }
                _ => {
                    if operand_count == operands.len() {
                        return Err(format!("uniq: extra operand '{}'", arg).into());
                    }
                    operands[operand_count] = Some(arg);
                    operand_count += 1;
                }
            }
            i += 1;
        }

        let input = operands[0];
        let output = operands[1];
        if let (Some(input_path), Some(output_path)) = (input, output) {
            if input_path != "-"
                && output_path != "-"
                && (input_path == output_path
                    || same_file(
                        Path::new(input_path),
                        Path::new(output_path),
                        FollowSymlinks::Yes,
                    )
                    .map_err(|error| format!("uniq: cannot compare input and output: {error}"))?)
            {
                return Err("uniq: input and output must be different files".into());
            }
        }

        if let Some(path) = output.filter(|path| *path != "-") {
            let file = File::create(path).map_err(|error| format!("uniq: {}: {}", path, error))?;
            let mut out = BufWriter::new(file);
            Self::process_input(input, &mut out, options)
                .map_err(|error| Self::input_error(input, error))?;
            out.flush()?;
        } else {
            let stdout = io::stdout();
            let mut out = stdout.lock();
            Self::process_input(input, &mut out, options)
                .map_err(|error| Self::input_error(input, error))?;
        }

        Ok(0)
    }

    fn help(&self) {
        println!("Usage: uniq [OPTION]... [INPUT [OUTPUT]]");
        println!();
        println!("{}", self.description());
        println!();
        println!("Options:");
        println!("  -c, --count        prefix lines by the number of occurrences");
        println!("  -d, --repeated     only print duplicate lines");
        println!("  -u, --unique       only print unique lines");
        println!("  -i, --ignore-case  ignore case when comparing");
        println!();
        println!("With no INPUT, or when INPUT is -, read standard input.");
    }
}

impl UniqApplet {
    fn process_input(
        input: Option<&str>,
        out: &mut impl Write,
        options: UniqOptions,
    ) -> io::Result<()> {
        match input {
            Some(path) if path != "-" => {
                let file = File::open(path)?;
                Self::process_reader(BufReader::new(file), out, options)
            }
            _ => {
                let stdin = io::stdin();
                Self::process_reader(BufReader::new(stdin.lock()), out, options)
            }
        }
    }

    fn process_reader(
        mut reader: impl BufRead,
        out: &mut impl Write,
        options: UniqOptions,
    ) -> io::Result<()> {
        let mut current = String::new();
        if reader.read_line(&mut current)? == 0 {
            return Ok(());
        }
        Self::trim_line_ending(&mut current);

        let mut current_key = options.ignore_case.then(|| current.to_lowercase());
        let mut count = 1_usize;
        let mut next = String::new();

        loop {
            next.clear();
            if reader.read_line(&mut next)? == 0 {
                break;
            }
            Self::trim_line_ending(&mut next);

            let next_key = options.ignore_case.then(|| next.to_lowercase());
            let is_same = match (&current_key, &next_key) {
                (Some(current), Some(next)) => current == next,
                _ => current == next,
            };

            if is_same {
                count += 1;
            } else {
                Self::write_group(out, &current, count, options)?;
                std::mem::swap(&mut current, &mut next);
                current_key = next_key;
                count = 1;
            }
        }

        Self::write_group(out, &current, count, options)
    }

    fn trim_line_ending(line: &mut String) {
        if line.ends_with('\n') {
            line.pop();
            if line.ends_with('\r') {
                line.pop();
            }
        }
    }

    fn write_group(
        out: &mut impl Write,
        line: &str,
        count: usize,
        options: UniqOptions,
    ) -> io::Result<()> {
        if options.repeated_only && count < 2 {
            return Ok(());
        }
        if options.unique_only && count > 1 {
            return Ok(());
        }
        if options.show_count {
            writeln!(out, "{:>7} {}", count, line)
        } else {
            writeln!(out, "{}", line)
        }
    }

    fn input_error(input: Option<&str>, error: io::Error) -> String {
        format!("uniq: {}: {}", input.unwrap_or("-"), error)
    }
}
