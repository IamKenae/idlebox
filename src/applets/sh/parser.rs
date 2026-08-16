use crate::applets::sh::lexer::{tokenize_with_keywords, RedirectOp, Token};

#[derive(Debug, Clone, PartialEq)]
pub enum Ast {
    Command(Command),
    Pipeline(Pipeline),
    List(List),
    If(If),
    For(For),
    While(While),
    Sequence(Vec<Ast>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    pub value: String,
    pub literal: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub name: Word,
    pub args: Vec<Word>,
    pub redirections: Vec<Redirection>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Redirection {
    pub op: RedirectOp,
    pub target: Word,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pipeline {
    pub commands: Vec<Ast>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub left: Box<Ast>,
    pub op: ListOp,
    pub right: Box<Ast>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListOp {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub struct If {
    pub condition: Box<Ast>,
    pub then_body: Box<Ast>,
    pub elif_branches: Vec<(Ast, Ast)>,
    pub else_body: Option<Box<Ast>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct For {
    pub var: String,
    pub items: Vec<Word>,
    pub body: Box<Ast>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct While {
    pub condition: Box<Ast>,
    pub body: Box<Ast>,
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(input: &str) -> Result<Ast, String> {
        let tokens = tokenize_with_keywords(input)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn parse_program(&mut self) -> Result<Ast, String> {
        let mut statements = Vec::new();
        self.skip_newlines();

        while !self.is_at_end() {
            let stmt = self.parse_list()?;
            statements.push(stmt);

            if self.is_at_end() {
                break;
            }

            match self.peek() {
                Token::Semi => {
                    self.advance();
                    self.skip_newlines();
                }
                Token::Newline => {
                    self.skip_newlines();
                }
                _ => break,
            }
        }

        if statements.len() == 1 {
            Ok(statements.pop().unwrap())
        } else {
            Ok(Ast::Sequence(statements))
        }
    }

    fn parse_list(&mut self) -> Result<Ast, String> {
        let mut left = self.parse_list_element()?;

        loop {
            match self.peek() {
                Token::And => {
                    self.advance();
                    self.skip_newlines();
                    let right = self.parse_list_element()?;
                    left = Ast::List(List {
                        left: Box::new(left),
                        op: ListOp::And,
                        right: Box::new(right),
                    });
                }
                Token::Or => {
                    self.advance();
                    self.skip_newlines();
                    let right = self.parse_list_element()?;
                    left = Ast::List(List {
                        left: Box::new(left),
                        op: ListOp::Or,
                        right: Box::new(right),
                    });
                }
                Token::Amp => {
                    return Err("background execution (&) is not supported".to_string());
                }
                _ => break,
            }
        }

        Ok(left)
    }

    fn parse_list_element(&mut self) -> Result<Ast, String> {
        match self.peek() {
            Token::If => self.parse_if(),
            Token::For => self.parse_for(),
            Token::While => self.parse_while(),
            _ => self.parse_pipeline(),
        }
    }

    fn parse_pipeline(&mut self) -> Result<Ast, String> {
        let mut commands = vec![self.parse_simple_command()?];

        while let Token::Pipe = self.peek() {
            self.advance();
            self.skip_newlines();
            commands.push(self.parse_simple_command()?);
        }

        if commands.len() == 1 {
            Ok(commands.pop().unwrap())
        } else {
            Ok(Ast::Pipeline(Pipeline { commands }))
        }
    }

    fn parse_simple_command(&mut self) -> Result<Ast, String> {
        let mut name = None;
        let mut args = Vec::new();
        let mut redirections = Vec::new();

        loop {
            match self.peek() {
                Token::Word(word) => {
                    let word = Word {
                        value: word.clone(),
                        literal: false,
                    };
                    self.advance();
                    if name.is_none() {
                        name = Some(word);
                    } else {
                        args.push(word);
                    }
                }
                Token::LiteralWord(word) => {
                    let word = Word {
                        value: word.clone(),
                        literal: true,
                    };
                    self.advance();
                    if name.is_none() {
                        name = Some(word);
                    } else {
                        args.push(word);
                    }
                }
                Token::Redirect(op) => {
                    let op = op.clone();
                    self.advance();
                    let target = match self.peek() {
                        Token::Word(word) => {
                            let w = Word {
                                value: word.clone(),
                                literal: false,
                            };
                            self.advance();
                            w
                        }
                        Token::LiteralWord(word) => {
                            let w = Word {
                                value: word.clone(),
                                literal: true,
                            };
                            self.advance();
                            w
                        }
                        _ => return Err("expected filename after redirection".to_string()),
                    };
                    redirections.push(Redirection { op, target });
                }
                _ => break,
            }
        }

        let name = name.ok_or("expected command name")?;
        Ok(Ast::Command(Command {
            name,
            args,
            redirections,
        }))
    }

    fn parse_if(&mut self) -> Result<Ast, String> {
        self.expect(Token::If)?;
        self.skip_newlines();

        let condition = self.parse_list()?;
        self.skip_newlines();
        if let Token::Semi = self.peek() {
            self.advance();
            self.skip_newlines();
        }
        self.expect(Token::Then)?;
        self.skip_newlines();

        let then_body = self.parse_compound_body(&[Token::Elif, Token::Else, Token::Fi])?;
        self.skip_newlines();

        let mut elif_branches = Vec::new();
        while let Token::Elif = self.peek() {
            self.advance();
            self.skip_newlines();
            let elif_condition = self.parse_list()?;
            self.skip_newlines();
            if let Token::Semi = self.peek() {
                self.advance();
                self.skip_newlines();
            }
            self.expect(Token::Then)?;
            self.skip_newlines();
            let elif_body = self.parse_compound_body(&[Token::Elif, Token::Else, Token::Fi])?;
            self.skip_newlines();
            elif_branches.push((elif_condition, elif_body));
        }

        let else_body = if let Token::Else = self.peek() {
            self.advance();
            self.skip_newlines();
            Some(Box::new(self.parse_compound_body(&[Token::Fi])?))
        } else {
            None
        };

        self.skip_newlines();
        self.expect(Token::Fi)?;

        Ok(Ast::If(If {
            condition: Box::new(condition),
            then_body: Box::new(then_body),
            elif_branches,
            else_body,
        }))
    }

    fn parse_for(&mut self) -> Result<Ast, String> {
        self.expect(Token::For)?;

        let var = if let Token::Word(var) = self.peek() {
            let var = var.clone();
            self.advance();
            var
        } else {
            return Err("expected variable name after 'for'".to_string());
        };

        self.skip_newlines();
        self.expect(Token::In)?;

        let mut items = Vec::new();
        loop {
            match self.peek() {
                Token::Word(item) => {
                    let item = Word {
                        value: item.clone(),
                        literal: false,
                    };
                    self.advance();
                    items.push(item);
                }
                Token::LiteralWord(item) => {
                    let item = Word {
                        value: item.clone(),
                        literal: true,
                    };
                    self.advance();
                    items.push(item);
                }
                _ => break,
            }
        }

        self.skip_newlines();
        if let Token::Semi = self.peek() {
            self.advance();
        }
        self.skip_newlines();

        self.expect(Token::Do)?;
        self.skip_newlines();

        let body = self.parse_compound_body(&[Token::Done])?;
        self.skip_newlines();
        self.expect(Token::Done)?;

        Ok(Ast::For(For {
            var,
            items,
            body: Box::new(body),
        }))
    }

    fn parse_while(&mut self) -> Result<Ast, String> {
        self.expect(Token::While)?;
        self.skip_newlines();

        let condition = self.parse_list()?;
        self.skip_newlines();
        if let Token::Semi = self.peek() {
            self.advance();
            self.skip_newlines();
        }
        self.expect(Token::Do)?;
        self.skip_newlines();

        let body = self.parse_compound_body(&[Token::Done])?;
        self.skip_newlines();
        self.expect(Token::Done)?;

        Ok(Ast::While(While {
            condition: Box::new(condition),
            body: Box::new(body),
        }))
    }

    fn parse_compound_body(&mut self, terminators: &[Token]) -> Result<Ast, String> {
        let mut statements = Vec::new();
        self.skip_newlines();

        while !self.is_at_end() && !terminators.contains(&self.peek()) {
            let stmt = self.parse_list()?;
            statements.push(stmt);
            self.skip_newlines();

            if !self.is_at_end() && !terminators.contains(&self.peek()) {
                match self.peek() {
                    Token::Semi | Token::Newline => {
                        self.advance();
                        self.skip_newlines();
                    }
                    _ => break,
                }
            }
        }

        if statements.len() == 1 {
            Ok(statements.pop().unwrap())
        } else {
            Ok(Ast::Sequence(statements))
        }
    }

    fn peek(&self) -> Token {
        self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(format!("expected {:?}, got {:?}", expected, self.peek()))
        }
    }

    fn skip_newlines(&mut self) {
        while let Token::Newline = self.peek() {
            self.advance();
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || matches!(self.peek(), Token::Eof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let ast = Parser::parse("echo hello").unwrap();
        match ast {
            Ast::Command(cmd) => {
                assert_eq!(cmd.name.value, "echo");
                assert_eq!(
                    cmd.args
                        .iter()
                        .map(|w| w.value.as_str())
                        .collect::<Vec<_>>(),
                    vec!["hello"]
                );
                assert!(cmd.redirections.is_empty());
            }
            _ => panic!("expected Command"),
        }
    }

    #[test]
    fn test_command_with_redirection() {
        let ast = Parser::parse("echo hello > file.txt").unwrap();
        match ast {
            Ast::Command(cmd) => {
                assert_eq!(cmd.name.value, "echo");
                assert_eq!(
                    cmd.args
                        .iter()
                        .map(|w| w.value.as_str())
                        .collect::<Vec<_>>(),
                    vec!["hello"]
                );
                assert_eq!(cmd.redirections.len(), 1);
                assert_eq!(cmd.redirections[0].op, RedirectOp::Out);
                assert_eq!(cmd.redirections[0].target.value, "file.txt");
            }
            _ => panic!("expected Command"),
        }
    }

    #[test]
    fn test_pipeline() {
        let ast = Parser::parse("ls | grep foo").unwrap();
        match ast {
            Ast::Pipeline(pipe) => {
                assert_eq!(pipe.commands.len(), 2);
                match &pipe.commands[0] {
                    Ast::Command(cmd) => assert_eq!(cmd.name.value, "ls"),
                    _ => panic!("expected Command"),
                }
                match &pipe.commands[1] {
                    Ast::Command(cmd) => {
                        assert_eq!(cmd.name.value, "grep");
                        assert_eq!(
                            cmd.args
                                .iter()
                                .map(|w| w.value.as_str())
                                .collect::<Vec<_>>(),
                            vec!["foo"]
                        );
                    }
                    _ => panic!("expected Command"),
                }
            }
            _ => panic!("expected Pipeline"),
        }
    }

    #[test]
    fn test_and_list() {
        let ast = Parser::parse("cmd1 && cmd2").unwrap();
        match ast {
            Ast::List(list) => {
                assert_eq!(list.op, ListOp::And);
                match *list.left {
                    Ast::Command(cmd) => assert_eq!(cmd.name.value, "cmd1"),
                    _ => panic!("expected Command"),
                }
                match *list.right {
                    Ast::Command(cmd) => assert_eq!(cmd.name.value, "cmd2"),
                    _ => panic!("expected Command"),
                }
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn test_or_list() {
        let ast = Parser::parse("cmd1 || cmd2").unwrap();
        match ast {
            Ast::List(list) => {
                assert_eq!(list.op, ListOp::Or);
                match *list.left {
                    Ast::Command(cmd) => assert_eq!(cmd.name.value, "cmd1"),
                    _ => panic!("expected Command"),
                }
                match *list.right {
                    Ast::Command(cmd) => assert_eq!(cmd.name.value, "cmd2"),
                    _ => panic!("expected Command"),
                }
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn test_sequence() {
        let ast = Parser::parse("cmd1; cmd2; cmd3").unwrap();
        match ast {
            Ast::Sequence(stmts) => {
                assert_eq!(stmts.len(), 3);
            }
            _ => panic!("expected Sequence"),
        }
    }

    #[test]
    fn test_if_statement() {
        let ast = Parser::parse("if true; then echo yes; fi").unwrap();
        match ast {
            Ast::If(if_stmt) => {
                match *if_stmt.condition {
                    Ast::Command(cmd) => assert_eq!(cmd.name.value, "true"),
                    _ => panic!("expected Command"),
                }
                assert!(if_stmt.elif_branches.is_empty());
                assert!(if_stmt.else_body.is_none());
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn test_if_else() {
        let ast = Parser::parse("if true; then echo yes; else echo no; fi").unwrap();
        match ast {
            Ast::If(if_stmt) => {
                assert!(if_stmt.else_body.is_some());
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn test_for_loop() {
        let ast = Parser::parse("for i in 1 2 3; do echo $i; done").unwrap();
        match ast {
            Ast::For(for_stmt) => {
                assert_eq!(for_stmt.var, "i");
                assert_eq!(
                    for_stmt
                        .items
                        .iter()
                        .map(|w| w.value.as_str())
                        .collect::<Vec<_>>(),
                    vec!["1", "2", "3"]
                );
            }
            _ => panic!("expected For"),
        }
    }

    #[test]
    fn test_while_loop() {
        let ast = Parser::parse("while true; do echo loop; done").unwrap();
        match ast {
            Ast::While(while_stmt) => match *while_stmt.condition {
                Ast::Command(cmd) => assert_eq!(cmd.name.value, "true"),
                _ => panic!("expected Command"),
            },
            _ => panic!("expected While"),
        }
    }

    #[test]
    fn test_complex_pipeline() {
        let ast = Parser::parse("cat file.txt | grep pattern | sort | uniq").unwrap();
        match ast {
            Ast::Pipeline(pipe) => {
                assert_eq!(pipe.commands.len(), 4);
            }
            _ => panic!("expected Pipeline"),
        }
    }

    #[test]
    fn test_multiple_redirections() {
        let ast = Parser::parse("cmd > out.txt 2> err.txt").unwrap();
        match ast {
            Ast::Command(cmd) => {
                assert_eq!(cmd.redirections.len(), 2);
                assert_eq!(cmd.redirections[0].op, RedirectOp::Out);
                assert_eq!(cmd.redirections[1].op, RedirectOp::Err);
            }
            _ => panic!("expected Command"),
        }
    }

    #[test]
    fn test_multiline_script() {
        let ast = Parser::parse("echo line1\necho line2\nexit 0\n").unwrap();
        match ast {
            Ast::Sequence(stmts) => {
                assert_eq!(stmts.len(), 3);
            }
            _ => panic!("expected Sequence, got {:?}", ast),
        }
    }
}
