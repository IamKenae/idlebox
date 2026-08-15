use crate::core::Applet;
use std::fs;
use std::path::Path;

pub struct TestApplet;

impl Applet for TestApplet {
    fn name(&self) -> &'static str {
        "test"
    }

    fn description(&self) -> &'static str {
        "Evaluate conditional expressions (POSIX test/[)"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let exit_code = evaluate(args);
        Ok(exit_code)
    }

    fn help(&self) {
        println!("Usage: test EXPR");
        println!("       [ EXPR ]");
        println!();
        println!("Evaluate conditional expressions.");
        println!();
        println!("File tests:");
        println!("  -e FILE  FILE exists");
        println!("  -f FILE  FILE exists and is a regular file");
        println!("  -d FILE  FILE exists and is a directory");
        println!("  -s FILE  FILE exists and has size > 0");
        println!("  -L FILE  FILE exists and is a symbolic link");
        println!("  -h FILE  same as -L");
        println!();
        println!("String tests:");
        println!("  -z STR   STR has zero length");
        println!("  -n STR   STR has non-zero length");
        println!("  S1 = S2  strings are equal");
        println!("  S1 == S2 strings are equal");
        println!("  S1 != S2 strings are not equal");
        println!();
        println!("Numeric tests:");
        println!("  N1 -eq N2  N1 equals N2");
        println!("  N1 -ne N2  N1 not equals N2");
        println!("  N1 -gt N2  N1 greater than N2");
        println!("  N1 -ge N2  N1 greater than or equal to N2");
        println!("  N1 -lt N2  N1 less than N2");
        println!("  N1 -le N2  N1 less than or equal to N2");
        println!();
        println!("Logical operators:");
        println!("  ! EXPR   negation");
        println!("  E1 -a E2 both true");
        println!("  E1 -o E2 either true");
    }
}

pub struct BracketApplet;

impl Applet for BracketApplet {
    fn name(&self) -> &'static str {
        "["
    }

    fn description(&self) -> &'static str {
        "Evaluate conditional expressions (alias for test)"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        if args.is_empty() || args.last().map(|s| s.as_str()) != Some("]") {
            eprintln!("[: missing ']'");
            return Ok(2);
        }
        let inner = &args[..args.len() - 1];
        let exit_code = evaluate(inner);
        Ok(exit_code)
    }

    fn help(&self) {
        println!("Usage: [ EXPR ]");
        println!();
        println!("Evaluate conditional expressions.");
        println!("This is equivalent to 'test EXPR' but requires a closing ']'.");
    }
}

fn evaluate(args: &[String]) -> i32 {
    if args.is_empty() {
        return 1;
    }

    match parse_expr(args) {
        Ok(val) => {
            if val {
                0
            } else {
                1
            }
        }
        Err(_) => 2,
    }
}

fn parse_expr(args: &[String]) -> Result<bool, ()> {
    if args.is_empty() {
        return Err(());
    }

    let mut pos = 0;
    let result = parse_or(args, &mut pos)?;

    if pos < args.len() {
        return Err(());
    }

    Ok(result)
}

fn parse_or(args: &[String], pos: &mut usize) -> Result<bool, ()> {
    let mut left = parse_and(args, pos)?;

    while *pos < args.len() && args[*pos] == "-o" {
        *pos += 1;
        let right = parse_and(args, pos)?;
        left = left || right;
    }

    Ok(left)
}

fn parse_and(args: &[String], pos: &mut usize) -> Result<bool, ()> {
    let mut left = parse_unary(args, pos)?;

    while *pos < args.len() && args[*pos] == "-a" {
        *pos += 1;
        let right = parse_unary(args, pos)?;
        left = left && right;
    }

    Ok(left)
}

fn parse_unary(args: &[String], pos: &mut usize) -> Result<bool, ()> {
    if *pos >= args.len() {
        return Err(());
    }

    if args[*pos] == "!" {
        *pos += 1;
        let val = parse_unary(args, pos)?;
        return Ok(!val);
    }

    parse_primary(args, pos)
}

fn parse_primary(args: &[String], pos: &mut usize) -> Result<bool, ()> {
    if *pos >= args.len() {
        return Err(());
    }

    let arg = &args[*pos];

    match arg.as_str() {
        "-f" | "-d" | "-e" | "-s" | "-L" | "-h" => {
            *pos += 1;
            if *pos >= args.len() {
                return Err(());
            }
            let path = &args[*pos];
            *pos += 1;
            Ok(file_test(arg, path))
        }
        "-z" => {
            *pos += 1;
            if *pos >= args.len() {
                return Err(());
            }
            let s = &args[*pos];
            *pos += 1;
            Ok(s.is_empty())
        }
        "-n" => {
            *pos += 1;
            if *pos >= args.len() {
                return Err(());
            }
            let s = &args[*pos];
            *pos += 1;
            Ok(!s.is_empty())
        }
        _ => {
            let left = arg.clone();
            *pos += 1;

            if *pos >= args.len() {
                return Ok(!left.is_empty());
            }

            let op = &args[*pos];

            match op.as_str() {
                "=" | "==" => {
                    *pos += 1;
                    if *pos >= args.len() {
                        return Err(());
                    }
                    let right = &args[*pos];
                    *pos += 1;
                    Ok(left == *right)
                }
                "!=" => {
                    *pos += 1;
                    if *pos >= args.len() {
                        return Err(());
                    }
                    let right = &args[*pos];
                    *pos += 1;
                    Ok(left != *right)
                }
                "-eq" | "-ne" | "-gt" | "-ge" | "-lt" | "-le" => {
                    *pos += 1;
                    if *pos >= args.len() {
                        return Err(());
                    }
                    let right = &args[*pos];
                    *pos += 1;
                    let l: i64 = left.parse().map_err(|_| ())?;
                    let r: i64 = right.parse().map_err(|_| ())?;
                    Ok(match op.as_str() {
                        "-eq" => l == r,
                        "-ne" => l != r,
                        "-gt" => l > r,
                        "-ge" => l >= r,
                        "-lt" => l < r,
                        "-le" => l <= r,
                        _ => unreachable!(),
                    })
                }
                _ => {
                    *pos -= 1;
                    Ok(!left.is_empty())
                }
            }
        }
    }
}

fn file_test(flag: &str, path: &str) -> bool {
    let p = Path::new(path);
    match flag {
        "-e" => p.exists(),
        "-f" => p.is_file(),
        "-d" => p.is_dir(),
        "-s" => {
            if let Ok(meta) = fs::metadata(p) {
                meta.len() > 0
            } else {
                false
            }
        }
        "-L" | "-h" => {
            if let Ok(meta) = fs::symlink_metadata(p) {
                meta.file_type().is_symlink()
            } else {
                false
            }
        }
        _ => false,
    }
}
