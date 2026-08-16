use crate::applets::sh::builtins::{execute_builtin, ShellState};
use crate::applets::sh::parser::{Ast, Command, For, If, List, ListOp, Pipeline, While};
use crate::core::Dispatcher;
use std::process::Command as ProcessCommand;

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

    fn execute_command(&mut self, cmd: &Command) -> Result<i32, Box<dyn std::error::Error>> {
        let name = self.expand_variables(&cmd.name);
        let args: Vec<String> = cmd
            .args
            .iter()
            .map(|arg| self.expand_variables(arg))
            .collect();

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

    fn execute_pipeline(&mut self, pipeline: &Pipeline) -> Result<i32, Box<dyn std::error::Error>> {
        if pipeline.commands.len() == 1 {
            return self.execute(&pipeline.commands[0]);
        }

        use std::process::Stdio;
        let mut prev_child: Option<std::process::Child> = None;

        for (idx, cmd_ast) in pipeline.commands.iter().enumerate() {
            let is_last = idx == pipeline.commands.len() - 1;

            if let Ast::Command(cmd) = cmd_ast {
                let name = self.expand_variables(&cmd.name);
                let args: Vec<String> = cmd
                    .args
                    .iter()
                    .map(|arg| self.expand_variables(arg))
                    .collect();

                if is_last {
                    if let Ok(code) = execute_builtin(&mut self.state, &name, &args) {
                        self.state.last_exit_code = code;
                        return Ok(code);
                    }

                    if let Ok(code) = self.dispatch_applet(&name, &args) {
                        self.state.last_exit_code = code;
                        return Ok(code);
                    }

                    let mut cmd_proc = ProcessCommand::new(&name);
                    cmd_proc.args(&args);
                    
                    if let Some(mut prev) = prev_child {
                        if let Some(stdout) = prev.stdout.take() {
                            cmd_proc.stdin(stdout);
                        }
                    }

                    for (key, value) in &self.state.env {
                        cmd_proc.env(key, value);
                    }

                    let status = cmd_proc.status()?;
                    let code = status.code().unwrap_or(1);
                    self.state.last_exit_code = code;
                    return Ok(code);
                } else {
                    if let Ok(code) = execute_builtin(&mut self.state, &name, &args) {
                        if code != 0 {
                            self.state.last_exit_code = code;
                            return Ok(code);
                        }
                        continue;
                    }

                    if let Ok(code) = self.dispatch_applet(&name, &args) {
                        if code != 0 {
                            self.state.last_exit_code = code;
                            return Ok(code);
                        }
                        continue;
                    }

                    let mut cmd_proc = ProcessCommand::new(&name);
                    cmd_proc.args(&args);
                    cmd_proc.stdout(Stdio::piped());
                    
                    if let Some(mut prev) = prev_child {
                        if let Some(stdout) = prev.stdout.take() {
                            cmd_proc.stdin(stdout);
                        }
                    }

                    for (key, value) in &self.state.env {
                        cmd_proc.env(key, value);
                    }

                    prev_child = Some(cmd_proc.spawn()?);
                }
            } else {
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
            .map(|item| self.expand_variables(item))
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
            name: "true".to_string(),
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
            name: "export".to_string(),
            args: vec!["TEST=value".to_string()],
            redirections: vec![],
        });
        let result = evaluator.execute(&ast).unwrap();
        assert_eq!(result, 0);
        assert_eq!(evaluator.state.get_var("TEST"), Some(&"value".to_string()));
    }
}
