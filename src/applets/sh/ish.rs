use crate::applets::sh::evaluator::Evaluator;
use crate::applets::sh::parser::Parser;
use crate::core::Applet;
use std::io::{self, BufRead, Write};

pub struct ShellApplet {
    name: &'static str,
}

impl ShellApplet {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl Applet for ShellApplet {
    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        "Idle Shell - A POSIX-compatible shell interpreter"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        if args.is_empty() {
            return run_repl(self.name);
        }

        if args[0] == "-c" {
            if args.len() < 2 {
                return Err(format!("{}: -c requires an argument", self.name).into());
            }
            return run_command(&args[1]);
        }

        run_script(&args[0])
    }

    fn help(&self) {
        println!("Usage: {} [OPTIONS] [SCRIPT]", self.name);
        println!("       {} -c COMMAND", self.name);
        println!();
        println!(
            "Idle Shell ({}) is a POSIX-compatible shell interpreter.",
            self.name
        );
        println!();
        println!("Options:");
        println!("  -c COMMAND    Execute COMMAND and exit");
        println!("  SCRIPT        Execute script file");
        println!("  (no args)     Start interactive shell");
        println!();
        println!("Examples:");
        println!("  {}                    Start interactive shell", self.name);
        println!("  {} script.sh          Execute script.sh", self.name);
        println!("  {} -c \"echo hello\"    Execute command", self.name);
    }
}

fn run_repl(applet_name: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let mut evaluator = Evaluator::new();
    let stdin = io::stdin();
    let mut line = String::new();
    let prompt = format!("{}$ ", applet_name);

    loop {
        print!("{}", prompt);
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
                    eprintln!("{}: {}", applet_name, e);
                    evaluator.state.last_exit_code = 1;
                }
            },
            Err(e) => {
                eprintln!("{}: parse error: {}", applet_name, e);
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
    let content = if content.starts_with("#!") {
        content.lines().skip(1).collect::<Vec<_>>().join("\n")
    } else {
        content
    };
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
