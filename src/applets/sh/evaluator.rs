use crate::applets::sh::builtins::{execute_builtin, ShellState};
use crate::applets::sh::parser::{Ast, Command, For, If, List, ListOp, Pipeline, While, Word};
use crate::core::Dispatcher;
use std::process::Command as ProcessCommand;

#[cfg(unix)]
mod libc {
    extern "C" {
        pub fn dup(oldfd: i32) -> i32;
        pub fn dup2(oldfd: i32, newfd: i32) -> i32;
        pub fn close(fd: i32) -> i32;
    }
}

pub struct Evaluator {
    pub state: ShellState,
    dispatcher: Dispatcher,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            state: ShellState::new(),
            dispatcher: Dispatcher::new(),
        }
    }

    pub fn execute(&mut self, ast: &Ast) -> Result<i32, Box<dyn std::error::Error>> {
        match ast {
            Ast::Command(cmd) => self.execute_command(cmd),
            Ast::Pipeline(pipeline) => self.execute_pipeline(pipeline),
            Ast::List(list) => self.execute_list(list),
            Ast::If(if_stmt) => self.execute_if(if_stmt),
            Ast::For(for_stmt) => self.execute_for(for_stmt),
            Ast::While(while_stmt) => self.execute_while(while_stmt),
            Ast::Sequence(stmts) => self.execute_sequence(stmts),
        }
    }

    fn expand_alias(&self, name: &str, args: Vec<String>) -> (String, Vec<String>) {
        if let Some(alias_value) = self.state.aliases.get(name).cloned() {
            let mut parts = alias_value.split_whitespace();
            if let Some(first) = parts.next() {
                let alias_args: Vec<String> = parts.map(|s| s.to_string()).collect();
                let mut new_args = alias_args;
                new_args.extend(args);
                return (first.to_string(), new_args);
            }
        }
        (name.to_string(), args)
    }

    fn execute_command(&mut self, cmd: &Command) -> Result<i32, Box<dyn std::error::Error>> {
        let raw_name = self.expand_word(&cmd.name);
        let raw_args: Vec<String> = cmd.args.iter().map(|arg| self.expand_word(arg)).collect();
        let (name, args) = self.expand_alias(&raw_name, raw_args);

        let mut stdin_redirect: Option<String> = None;
        let mut stdout_redirect: Option<(String, bool)> = None;
        let mut stderr_redirect: Option<(String, bool)> = None;

        for redir in &cmd.redirections {
            let target = self.expand_word(&redir.target);
            match redir.op {
                crate::applets::sh::lexer::RedirectOp::In => {
                    stdin_redirect = Some(target);
                }
                crate::applets::sh::lexer::RedirectOp::Out => {
                    stdout_redirect = Some((target, false));
                }
                crate::applets::sh::lexer::RedirectOp::Append => {
                    stdout_redirect = Some((target, true));
                }
                crate::applets::sh::lexer::RedirectOp::Err => {
                    stderr_redirect = Some((target, false));
                }
                crate::applets::sh::lexer::RedirectOp::ErrAppend => {
                    stderr_redirect = Some((target, true));
                }
            }
        }

        if stdin_redirect.is_some() || stdout_redirect.is_some() || stderr_redirect.is_some() {
            return self.execute_command_with_redirects(
                &name,
                &args,
                stdin_redirect,
                stdout_redirect,
                stderr_redirect,
            );
        }

        if let Ok(code) = execute_builtin(&mut self.state, &name, &args) {
            self.state.last_exit_code = code;
            return Ok(code);
        }

        if let Ok(code) = self.dispatch_applet(&name, &args) {
            self.state.last_exit_code = code;
            return Ok(code);
        }

        let exit_code = self.execute_external(&name, &args)?;
        self.state.last_exit_code = exit_code;
        Ok(exit_code)
    }

    #[cfg(unix)]
    fn execute_command_with_redirects(
        &mut self,
        name: &str,
        args: &[String],
        stdin_redirect: Option<String>,
        stdout_redirect: Option<(String, bool)>,
        stderr_redirect: Option<(String, bool)>,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        use std::fs::{File, OpenOptions};
        use std::os::unix::io::AsRawFd;

        let is_builtin_or_applet = self.is_builtin_or_applet(name);

        if is_builtin_or_applet {
            let saved_stdin = stdin_redirect.as_ref().map(|_| unsafe { libc::dup(0) });
            let saved_stdout = stdout_redirect.as_ref().map(|_| unsafe { libc::dup(1) });
            let saved_stderr = stderr_redirect.as_ref().map(|_| unsafe { libc::dup(2) });

            if let Some(path) = &stdin_redirect {
                let file = File::open(path)?;
                unsafe { libc::dup2(file.as_raw_fd(), 0) };
            }
            if let Some((path, append)) = &stdout_redirect {
                let file = if *append {
                    OpenOptions::new().create(true).append(true).open(path)?
                } else {
                    File::create(path)?
                };
                unsafe { libc::dup2(file.as_raw_fd(), 1) };
            }
            if let Some((path, append)) = &stderr_redirect {
                let file = if *append {
                    OpenOptions::new().create(true).append(true).open(path)?
                } else {
                    File::create(path)?
                };
                unsafe { libc::dup2(file.as_raw_fd(), 2) };
            }

            let code = if matches!(
                name,
                "cd" | "exit" | "export" | "unset" | "alias" | "unalias" | "read" | "pwd"
            ) {
                execute_builtin(&mut self.state, name, args).unwrap_or(1)
            } else {
                self.dispatch_applet(name, args).unwrap_or(1)
            };

            if let Some(fd) = saved_stdin {
                unsafe {
                    libc::dup2(fd, 0);
                    libc::close(fd);
                }
            }
            if let Some(fd) = saved_stdout {
                unsafe {
                    libc::dup2(fd, 1);
                    libc::close(fd);
                }
            }
            if let Some(fd) = saved_stderr {
                unsafe {
                    libc::dup2(fd, 2);
                    libc::close(fd);
                }
            }

            self.state.last_exit_code = code;
            return Ok(code);
        }

        let mut cmd = ProcessCommand::new(name);
        cmd.args(args);

        if let Some(path) = stdin_redirect {
            cmd.stdin(File::open(&path)?);
        }
        if let Some((path, append)) = stdout_redirect {
            let file = if append {
                OpenOptions::new().create(true).append(true).open(&path)?
            } else {
                File::create(&path)?
            };
            cmd.stdout(file);
        }
        if let Some((path, append)) = stderr_redirect {
            let file = if append {
                OpenOptions::new().create(true).append(true).open(&path)?
            } else {
                File::create(&path)?
            };
            cmd.stderr(file);
        }

        for (key, value) in &self.state.env {
            cmd.env(key, value);
        }

        let status = cmd.status()?;
        let code = status.code().unwrap_or(1);
        self.state.last_exit_code = code;
        Ok(code)
    }

    #[cfg(not(unix))]
    fn execute_command_with_redirects(
        &mut self,
        name: &str,
        args: &[String],
        stdin_redirect: Option<String>,
        stdout_redirect: Option<(String, bool)>,
        stderr_redirect: Option<(String, bool)>,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        use std::fs::{File, OpenOptions};

        let mut cmd = ProcessCommand::new(name);
        cmd.args(args);

        if let Some(path) = stdin_redirect {
            cmd.stdin(File::open(&path)?);
        }
        if let Some((path, append)) = stdout_redirect {
            let file = if append {
                OpenOptions::new().create(true).append(true).open(&path)?
            } else {
                File::create(&path)?
            };
            cmd.stdout(file);
        }
        if let Some((path, append)) = stderr_redirect {
            let file = if append {
                OpenOptions::new().create(true).append(true).open(&path)?
            } else {
                File::create(&path)?
            };
            cmd.stderr(file);
        }

        for (key, value) in &self.state.env {
            cmd.env(key, value);
        }

        let status = cmd.status()?;
        let code = status.code().unwrap_or(1);
        self.state.last_exit_code = code;
        Ok(code)
    }

    fn is_builtin_or_applet(&self, name: &str) -> bool {
        if matches!(
            name,
            "cd" | "exit" | "export" | "unset" | "alias" | "unalias" | "read" | "pwd"
        ) {
            return true;
        }
        // Check if it's a known applet by trying with empty args
        // If it fails with "applet not found", it's not an applet
        match self.dispatcher.dispatch(name, &[]) {
            Ok(_) => true,
            Err(e) => !e.to_string().contains("applet not found"),
        }
    }

    fn execute_pipeline(&mut self, pipeline: &Pipeline) -> Result<i32, Box<dyn std::error::Error>> {
        if pipeline.commands.len() == 1 {
            return self.execute(&pipeline.commands[0]);
        }

        use std::process::Stdio;
        let mut children: Vec<std::process::Child> = Vec::new();
        let mut pending_input: Option<std::fs::File> = None;

        for (idx, cmd_ast) in pipeline.commands.iter().enumerate() {
            let is_last = idx == pipeline.commands.len() - 1;

            if let Ast::Command(cmd) = cmd_ast {
                let raw_name = self.expand_word(&cmd.name);
                let raw_args: Vec<String> =
                    cmd.args.iter().map(|arg| self.expand_word(arg)).collect();
                let (name, args) = self.expand_alias(&raw_name, raw_args);

                if is_last {
                    
                    // Check if we have pending input from a previous command
                    let has_pending_input = pending_input.is_some();
                    
                    // Last command: try builtin/applet first (only if no pending input)
                    if !has_pending_input {
                        if let Ok(code) = execute_builtin(&mut self.state, &name, &args) {
                            for mut child in children {
                                let _ = child.wait();
                            }
                            self.state.last_exit_code = code;
                            return Ok(code);
                        }
                        if let Ok(code) = self.dispatch_applet(&name, &args) {
                            for mut child in children {
                                let _ = child.wait();
                            }
                            self.state.last_exit_code = code;
                            return Ok(code);
                        }
                    }

                    // Fall back to external command
                    let mut cmd_proc = ProcessCommand::new(&name);
                    cmd_proc.args(&args);

                    if let Some(file) = pending_input.take() {
                        cmd_proc.stdin(file);
                    } else if let Some(prev) = children.last_mut() {
                        if let Some(stdout) = prev.stdout.take() {
                            cmd_proc.stdin(stdout);
                        }
                    }

                    for (key, value) in &self.state.env {
                        cmd_proc.env(key, value);
                    }

                    let status = cmd_proc.status()?;
                    let code = status.code().unwrap_or(1);
                    for mut child in children {
                        let _ = child.wait();
                    }
                    self.state.last_exit_code = code;
                    return Ok(code);
                } else {
                    // Non-last command: try builtin/applet with output capture
                    let is_builtin_or_applet = self.is_builtin_or_applet(&name);

                    if is_builtin_or_applet {
                        #[cfg(unix)]
                        {
                            use std::os::unix::io::AsRawFd;

                            let temp_path = std::env::temp_dir().join(format!(
                                "ish_pipe_{}_{}",
                                std::process::id(),
                                idx
                            ));
                            let saved_stdout = unsafe { libc::dup(1) };
                            let temp_file = std::fs::File::create(&temp_path)?;
                            unsafe { libc::dup2(temp_file.as_raw_fd(), 1) };

                            let _code = if matches!(
                                name.as_str(),
                                "cd"
                                    | "exit"
                                    | "export"
                                    | "unset"
                                    | "alias"
                                    | "unalias"
                                    | "read"
                                    | "pwd"
                            ) {
                                execute_builtin(&mut self.state, &name, &args)
                            } else {
                                self.dispatch_applet(&name, &args)
                            };

                            unsafe {
                                libc::dup2(saved_stdout, 1);
                                libc::close(saved_stdout);
                            }
                            drop(temp_file);

                            // Open the temp file for reading and pass it as input to the next command
                            let input_file = std::fs::File::open(&temp_path)?;
                            pending_input = Some(input_file);
                            // Clean up the temp file after it's been opened
                            let _ = std::fs::remove_file(&temp_path);
                        }

                        #[cfg(not(unix))]
                        {
                            // On non-Unix, fall through to external command
                            let mut cmd_proc = ProcessCommand::new(&name);
                            cmd_proc.args(&args);
                            cmd_proc.stdout(Stdio::piped());

                            if let Some(file) = pending_input.take() {
                                cmd_proc.stdin(file);
                            } else if let Some(prev) = children.last_mut() {
                                if let Some(stdout) = prev.stdout.take() {
                                    cmd_proc.stdin(stdout);
                                }
                            }

                            for (key, value) in &self.state.env {
                                cmd_proc.env(key, value);
                            }

                            match cmd_proc.spawn() {
                                Ok(child) => children.push(child),
                                Err(e) => {
                                    for mut child in children {
                                        let _ = child.wait();
                                    }
                                    return Err(e.into());
                                }
                            }
                        }
                    } else {
                        // External command
                        let mut cmd_proc = ProcessCommand::new(&name);
                        cmd_proc.args(&args);
                        cmd_proc.stdout(Stdio::piped());

                        if let Some(file) = pending_input.take() {
                            cmd_proc.stdin(file);
                        } else if let Some(prev) = children.last_mut() {
                            if let Some(stdout) = prev.stdout.take() {
                                cmd_proc.stdin(stdout);
                            }
                        }

                        for (key, value) in &self.state.env {
                            cmd_proc.env(key, value);
                        }

                        match cmd_proc.spawn() {
                            Ok(child) => children.push(child),
                            Err(e) => {
                                for mut child in children {
                                    let _ = child.wait();
                                }
                                return Err(e.into());
                            }
                        }
                    }
                }
            } else {
                for mut child in children {
                    let _ = child.wait();
                }
                return Err("pipeline elements must be commands".into());
            }
        }

        Ok(0)
    }

    fn execute_list(&mut self, list: &List) -> Result<i32, Box<dyn std::error::Error>> {
        let left_code = self.execute(&list.left)?;

        match list.op {
            ListOp::And => {
                if left_code == 0 {
                    self.execute(&list.right)
                } else {
                    Ok(left_code)
                }
            }
            ListOp::Or => {
                if left_code != 0 {
                    self.execute(&list.right)
                } else {
                    Ok(left_code)
                }
            }
        }
    }

    fn execute_if(&mut self, if_stmt: &If) -> Result<i32, Box<dyn std::error::Error>> {
        let condition_code = self.execute(&if_stmt.condition)?;

        if condition_code == 0 {
            return self.execute(&if_stmt.then_body);
        }

        for (elif_condition, elif_body) in &if_stmt.elif_branches {
            let elif_code = self.execute(elif_condition)?;
            if elif_code == 0 {
                return self.execute(elif_body);
            }
        }

        if let Some(else_body) = &if_stmt.else_body {
            return self.execute(else_body);
        }

        Ok(0)
    }

    fn execute_for(&mut self, for_stmt: &For) -> Result<i32, Box<dyn std::error::Error>> {
        let items: Vec<String> = for_stmt
            .items
            .iter()
            .map(|item| self.expand_word(item))
            .collect();

        let mut last_code = 0;
        for item in items {
            self.state.set_var(for_stmt.var.clone(), item);
            last_code = self.execute(&for_stmt.body)?;
        }

        Ok(last_code)
    }

    fn execute_while(&mut self, while_stmt: &While) -> Result<i32, Box<dyn std::error::Error>> {
        let mut last_code = 0;
        loop {
            let condition_code = self.execute(&while_stmt.condition)?;
            if condition_code != 0 {
                break;
            }
            last_code = self.execute(&while_stmt.body)?;
        }
        Ok(last_code)
    }

    fn execute_sequence(&mut self, stmts: &[Ast]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut last_code = 0;
        for stmt in stmts {
            last_code = self.execute(stmt)?;
            if self.state.should_exit {
                break;
            }
        }
        Ok(last_code)
    }

    fn expand_word(&self, word: &Word) -> String {
        if word.literal {
            return word.value.clone();
        }
        self.expand_variables(&word.value)
    }

    fn expand_variables(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' {
                match chars.peek() {
                    Some('{') => {
                        chars.next();
                        let mut var_name = String::new();
                        loop {
                            match chars.next() {
                                Some('}') => break,
                                Some(c) => var_name.push(c),
                                None => break,
                            }
                        }
                        if let Some(value) = self.state.get_var(&var_name) {
                            result.push_str(value);
                        }
                    }
                    Some('?') => {
                        chars.next();
                        result.push_str(&self.state.last_exit_code.to_string());
                    }
                    Some(c) if c.is_alphabetic() || *c == '_' => {
                        let mut var_name = String::new();
                        while let Some(&c) = chars.peek() {
                            if c.is_alphanumeric() || c == '_' {
                                var_name.push(c);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        if let Some(value) = self.state.get_var(&var_name) {
                            result.push_str(value);
                        }
                    }
                    _ => result.push(ch),
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    fn dispatch_applet(
        &self,
        name: &str,
        args: &[String],
    ) -> Result<i32, Box<dyn std::error::Error>> {
        self.dispatcher.dispatch(name, args)
    }

    fn execute_external(
        &self,
        name: &str,
        args: &[String],
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let mut cmd = ProcessCommand::new(name);
        cmd.args(args);

        for (key, value) in &self.state.env {
            cmd.env(key, value);
        }

        let status = cmd.status()?;
        Ok(status.code().unwrap_or(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use crate::applets::sh::lexer::RedirectOp;
    #[cfg(unix)]
    use crate::applets::sh::parser::Redirection;
    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use tempfile::TempDir;

    #[test]
    fn test_expand_variables_simple() {
        let mut evaluator = Evaluator::new();
        evaluator
            .state
            .set_var("HOME".to_string(), "/home/user".to_string());
        let result = evaluator.expand_variables("$HOME");
        assert_eq!(result, "/home/user");
    }

    #[test]
    fn test_expand_variables_braced() {
        let mut evaluator = Evaluator::new();
        evaluator
            .state
            .set_var("USER".to_string(), "alice".to_string());
        let result = evaluator.expand_variables("Hello ${USER}!");
        assert_eq!(result, "Hello alice!");
    }

    #[test]
    fn test_expand_variables_undefined() {
        let evaluator = Evaluator::new();
        let result = evaluator.expand_variables("$UNDEFINED");
        assert_eq!(result, "");
    }

    #[test]
    fn test_execute_simple_command() {
        let mut evaluator = Evaluator::new();
        let ast = Ast::Command(Command {
            name: Word {
                value: "true".to_string(),
                literal: false,
            },
            args: vec![],
            redirections: vec![],
        });
        let result = evaluator.execute(&ast).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_execute_builtin_export() {
        let mut evaluator = Evaluator::new();
        let ast = Ast::Command(Command {
            name: Word {
                value: "export".to_string(),
                literal: false,
            },
            args: vec![Word {
                value: "TEST=value".to_string(),
                literal: false,
            }],
            redirections: vec![],
        });
        let result = evaluator.execute(&ast).unwrap();
        assert_eq!(result, 0);
        assert_eq!(evaluator.state.get_var("TEST"), Some(&"value".to_string()));
    }

    #[test]
    #[cfg(unix)]
    fn test_execute_with_stdout_redirect() {
        let temp_dir = TempDir::new().unwrap();
        let output_file = temp_dir.path().join("output.txt");

        let mut evaluator = Evaluator::new();
        let ast = Ast::Command(Command {
            name: Word {
                value: "echo".to_string(),
                literal: false,
            },
            args: vec![Word {
                value: "hello".to_string(),
                literal: false,
            }],
            redirections: vec![Redirection {
                op: RedirectOp::Out,
                target: Word {
                    value: output_file.to_str().unwrap().to_string(),
                    literal: false,
                },
            }],
        });

        let result = evaluator.execute(&ast).unwrap();
        assert_eq!(result, 0);

        let content = fs::read_to_string(&output_file).unwrap();
        assert_eq!(content.trim(), "hello");
    }

    #[test]
    #[cfg(unix)]
    fn test_execute_with_append_redirect() {
        let temp_dir = TempDir::new().unwrap();
        let output_file = temp_dir.path().join("output.txt");

        fs::write(&output_file, "first\n").unwrap();

        let mut evaluator = Evaluator::new();
        let ast = Ast::Command(Command {
            name: Word {
                value: "echo".to_string(),
                literal: false,
            },
            args: vec![Word {
                value: "second".to_string(),
                literal: false,
            }],
            redirections: vec![Redirection {
                op: RedirectOp::Append,
                target: Word {
                    value: output_file.to_str().unwrap().to_string(),
                    literal: false,
                },
            }],
        });

        let result = evaluator.execute(&ast).unwrap();
        assert_eq!(result, 0);

        let content = fs::read_to_string(&output_file).unwrap();
        assert_eq!(content, "first\nsecond\n");
    }

    #[test]
    fn test_execute_and_list() {
        let mut evaluator = Evaluator::new();
        let ast = Ast::List(List {
            left: Box::new(Ast::Command(Command {
                name: Word {
                    value: "true".to_string(),
                    literal: false,
                },
                args: vec![],
                redirections: vec![],
            })),
            op: ListOp::And,
            right: Box::new(Ast::Command(Command {
                name: Word {
                    value: "true".to_string(),
                    literal: false,
                },
                args: vec![],
                redirections: vec![],
            })),
        });

        let result = evaluator.execute(&ast).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_execute_or_list() {
        let mut evaluator = Evaluator::new();
        let ast = Ast::List(List {
            left: Box::new(Ast::Command(Command {
                name: Word {
                    value: "false".to_string(),
                    literal: false,
                },
                args: vec![],
                redirections: vec![],
            })),
            op: ListOp::Or,
            right: Box::new(Ast::Command(Command {
                name: Word {
                    value: "true".to_string(),
                    literal: false,
                },
                args: vec![],
                redirections: vec![],
            })),
        });

        let result = evaluator.execute(&ast).unwrap();
        assert_eq!(result, 0);
    }
}
