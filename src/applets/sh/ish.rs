use crate::applets::sh::evaluator::Evaluator;
use crate::applets::sh::parser::Parser;
use crate::core::Applet;
use std::io::{self, BufRead, Write};

pub struct IshApplet;

impl Applet for IshApplet {
    fn name(&self) -> &'static str {
        "ish"
    }

    fn description(&self) -> &'static str {
        "Idle Shell - A POSIX-compatible shell interpreter"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        if args.is_empty() {
            return run_repl();
        }

        if args[0] == "-c" {
            if args.len() < 2 {
                return Err("ish: -c requires an argument".into());
            }
            return run_command(&args[1]);
        }

        run_script(&args[0])
    }

    fn help(&self) {
        println!("Usage: ish [OPTIONS] [SCRIPT]");
        println!("       ish -c COMMAND");
        println!();
        println!("Idle Shell (ish) is a POSIX-compatible shell interpreter.");
        println!();
        println!("Options:");
        println!("  -c COMMAND    Execute COMMAND and exit");
        println!("  SCRIPT        Execute script file");
        println!("  (no args)     Start interactive shell");
        println!();
        println!("Examples:");
        println!("  ish                    Start interactive shell");
        println!("  ish script.sh          Execute script.sh");
        println!("  ish -c \"echo hello\"    Execute command");
    }
}

pub struct ShApplet;

impl Applet for ShApplet {
    fn name(&self) -> &'static str {
        "sh"
    }

    fn description(&self) -> &'static str {
        "Symbolic link to ish (Idle Shell)"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        IshApplet.run(args)
    }
}

pub struct AshApplet;

impl Applet for AshApplet {
    fn name(&self) -> &'static str {
        "ash"
    }

    fn description(&self) -> &'static str {
        "Symbolic link to ish (Idle Shell)"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        IshApplet.run(args)
    }
}

fn run_repl() -> Result<i32, Box<dyn std::error::Error>> {
    let mut evaluator = Evaluator::new();
    let stdin = io::stdin();
    let mut line = String::new();

    loop {
        print!("ish$ ");
        io::stdout().flush()?;

        line.clear();
        let bytes_read = stdin.lock().read_line(&mut line)?;

        if bytes_read == 0 {
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match Parser::parse(line) {
            Ok(ast) => match evaluator.execute(&ast) {
                Ok(code) => {
                    if evaluator.state.should_exit {
                        return Ok(evaluator.state.exit_code);
                    }
                    evaluator.state.last_exit_code = code;
                }
                Err(e) => {
                    eprintln!("ish: {}", e);
                    evaluator.state.last_exit_code = 1;
                }
            },
            Err(e) => {
                eprintln!("ish: parse error: {}", e);
                evaluator.state.last_exit_code = 1;
            }
        }
    }

    Ok(evaluator.state.last_exit_code)
}

fn run_command(command: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let mut evaluator = Evaluator::new();
    let ast = Parser::parse(command)?;
    let code = evaluator.execute(&ast)?;
    Ok(code)
}

fn run_script(path: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let mut evaluator = Evaluator::new();
    let ast = Parser::parse(&content)?;
    let code = evaluator.execute(&ast)?;
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_run_command_echo() {
        let result = run_command("echo hello").unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_run_command_true() {
        let result = run_command("true").unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_run_command_false() {
        let result = run_command("false").unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_run_command_export() {
        let result = run_command("export TEST=value").unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_run_script() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");
        fs::write(&script_path, "echo hello\nexit 0\n").unwrap();

        let result = run_script(script_path.to_str().unwrap()).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_run_script_with_exit_code() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");
        fs::write(&script_path, "exit 42\n").unwrap();

        let result = run_script(script_path.to_str().unwrap()).unwrap();
        assert_eq!(result, 42);
    }
}
