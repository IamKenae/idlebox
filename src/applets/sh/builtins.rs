use std::collections::HashMap;
use std::io::{self, BufRead};

pub struct ShellState {
    pub env: HashMap<String, String>,
    pub aliases: HashMap<String, String>,
    pub last_exit_code: i32,
    pub should_exit: bool,
    pub exit_code: i32,
}

impl ShellState {
    pub fn new() -> Self {
        let mut env = HashMap::new();
        for (key, value) in std::env::vars() {
            env.insert(key, value);
        }
        Self {
            env,
            aliases: HashMap::new(),
            last_exit_code: 0,
            should_exit: false,
            exit_code: 0,
        }
    }

    pub fn get_var(&self, name: &str) -> Option<&String> {
        self.env.get(name)
    }

    pub fn set_var(&mut self, name: String, value: String) {
        self.env.insert(name, value);
    }

    pub fn unset_var(&mut self, name: &str) {
        self.env.remove(name);
    }

    pub fn set_alias(&mut self, name: String, value: String) {
        self.aliases.insert(name, value);
    }

    pub fn unset_alias(&mut self, name: &str) {
        self.aliases.remove(name);
    }
}

pub fn execute_builtin(
    state: &mut ShellState,
    name: &str,
    args: &[String],
) -> Result<i32, Box<dyn std::error::Error>> {
    match name {
        "cd" => builtin_cd(state, args),
        "exit" => builtin_exit(state, args),
        "export" => builtin_export(state, args),
        "unset" => builtin_unset(state, args),
        "alias" => builtin_alias(state, args),
        "unalias" => builtin_unalias(state, args),
        "read" => builtin_read(state, args),
        "pwd" => builtin_pwd(state, args),
        _ => Err(format!("{}: not a shell builtin", name).into()),
    }
}

fn builtin_cd(
    state: &mut ShellState,
    args: &[String],
) -> Result<i32, Box<dyn std::error::Error>> {
    let target = if args.is_empty() {
        state
            .env
            .get("HOME")
            .cloned()
            .unwrap_or_else(|| "/".to_string())
    } else {
        args[0].clone()
    };

    match std::env::set_current_dir(&target) {
        Ok(_) => Ok(0),
        Err(e) => {
            eprintln!("cd: {}: {}", target, e);
            Ok(1)
        }
    }
}

fn builtin_exit(
    state: &mut ShellState,
    args: &[String],
) -> Result<i32, Box<dyn std::error::Error>> {
    let code = if args.is_empty() {
        state.last_exit_code
    } else {
        args[0].parse::<i32>().unwrap_or(1)
    };
    state.should_exit = true;
    state.exit_code = code;
    Ok(code)
}

fn builtin_export(
    state: &mut ShellState,
    args: &[String],
) -> Result<i32, Box<dyn std::error::Error>> {
    if args.is_empty() {
        for (key, value) in &state.env {
            println!("export {}=\"{}\"", key, value);
        }
        return Ok(0);
    }

    for arg in args {
        if let Some(eq_pos) = arg.find('=') {
            let key = &arg[..eq_pos];
            let value = &arg[eq_pos + 1..];
            state.set_var(key.to_string(), value.to_string());
        } else {
            if let Some(value) = state.get_var(arg) {
                state.set_var(arg.clone(), value.clone());
            }
        }
    }
    Ok(0)
}

fn builtin_unset(
    state: &mut ShellState,
    args: &[String],
) -> Result<i32, Box<dyn std::error::Error>> {
    for arg in args {
        state.unset_var(arg);
    }
    Ok(0)
}

fn builtin_alias(
    state: &mut ShellState,
    args: &[String],
) -> Result<i32, Box<dyn std::error::Error>> {
    if args.is_empty() {
        for (name, value) in &state.aliases {
            println!("alias {}='{}'", name, value);
        }
        return Ok(0);
    }

    for arg in args {
        if let Some(eq_pos) = arg.find('=') {
            let name = &arg[..eq_pos];
            let value = &arg[eq_pos + 1..];
            state.set_alias(name.to_string(), value.to_string());
        } else {
            if let Some(value) = state.aliases.get(arg) {
                println!("alias {}='{}'", arg, value);
            } else {
                eprintln!("alias: {}: not found", arg);
                return Ok(1);
            }
        }
    }
    Ok(0)
}

fn builtin_unalias(
    state: &mut ShellState,
    args: &[String],
) -> Result<i32, Box<dyn std::error::Error>> {
    for arg in args {
        state.unset_alias(arg);
    }
    Ok(0)
}

fn builtin_read(
    state: &mut ShellState,
    args: &[String],
) -> Result<i32, Box<dyn std::error::Error>> {
    let var_name = if args.is_empty() {
        "REPLY"
    } else {
        &args[0]
    };

    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(_) => {
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            state.set_var(var_name.to_string(), line.to_string());
            Ok(0)
        }
        Err(_) => Ok(1),
    }
}

fn builtin_pwd(
    _state: &mut ShellState,
    _args: &[String],
) -> Result<i32, Box<dyn std::error::Error>> {
    match std::env::current_dir() {
        Ok(path) => {
            println!("{}", path.display());
            Ok(0)
        }
        Err(e) => {
            eprintln!("pwd: {}", e);
            Ok(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_state_env() {
        let mut state = ShellState::new();
        state.set_var("TEST_VAR".to_string(), "test_value".to_string());
        assert_eq!(state.get_var("TEST_VAR"), Some(&"test_value".to_string()));
        state.unset_var("TEST_VAR");
        assert_eq!(state.get_var("TEST_VAR"), None);
    }

    #[test]
    fn test_shell_state_aliases() {
        let mut state = ShellState::new();
        state.set_alias("ll".to_string(), "ls -l".to_string());
        assert_eq!(state.aliases.get("ll"), Some(&"ls -l".to_string()));
        state.unset_alias("ll");
        assert_eq!(state.aliases.get("ll"), None);
    }

    #[test]
    fn test_builtin_export() {
        let mut state = ShellState::new();
        let args = vec!["FOO=bar".to_string()];
        let result = builtin_export(&mut state, &args).unwrap();
        assert_eq!(result, 0);
        assert_eq!(state.get_var("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_builtin_unset() {
        let mut state = ShellState::new();
        state.set_var("FOO".to_string(), "bar".to_string());
        let args = vec!["FOO".to_string()];
        let result = builtin_unset(&mut state, &args).unwrap();
        assert_eq!(result, 0);
        assert_eq!(state.get_var("FOO"), None);
    }

    #[test]
    fn test_builtin_alias() {
        let mut state = ShellState::new();
        let args = vec!["ll=ls -l".to_string()];
        let result = builtin_alias(&mut state, &args).unwrap();
        assert_eq!(result, 0);
        assert_eq!(state.aliases.get("ll"), Some(&"ls -l".to_string()));
    }

    #[test]
    fn test_builtin_unalias() {
        let mut state = ShellState::new();
        state.set_alias("ll".to_string(), "ls -l".to_string());
        let args = vec!["ll".to_string()];
        let result = builtin_unalias(&mut state, &args).unwrap();
        assert_eq!(result, 0);
        assert_eq!(state.aliases.get("ll"), None);
    }

    #[test]
    fn test_builtin_exit() {
        let mut state = ShellState::new();
        let args = vec!["42".to_string()];
        let result = builtin_exit(&mut state, &args).unwrap();
        assert_eq!(result, 42);
        assert!(state.should_exit);
        assert_eq!(state.exit_code, 42);
    }
}
