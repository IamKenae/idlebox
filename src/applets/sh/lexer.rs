use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Word(String),
    Pipe,
    And,
    Or,
    Semi,
    Amp,
    Redirect(RedirectOp),
    If,
    Then,
    Elif,
    Else,
    Fi,
    For,
    In,
    Do,
    Done,
    While,
    Newline,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectOp {
    Out,
    Append,
    In,
    Err,
}

pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
    raw: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.chars().peekable(),
            raw: input,
            pos: 0,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                None => {
                    tokens.push(Token::Eof);
                    break;
                }
                Some(ch) => {
                    let token = match ch {
                        '\n' => {
                            self.advance();
                            Token::Newline
                        }
                        '|' => {
                            self.advance();
                            if self.peek() == Some('|') {
                                self.advance();
                                Token::Or
                            } else {
                                Token::Pipe
                            }
                        }
                        '&' => {
                            self.advance();
                            if self.peek() == Some('&') {
                                self.advance();
                                Token::And
                            } else {
                                Token::Amp
                            }
                        }
                        ';' => {
                            self.advance();
                            Token::Semi
                        }
                        '>' => {
                            self.advance();
                            if self.peek() == Some('>') {
                                self.advance();
                                Token::Redirect(RedirectOp::Append)
                            } else {
                                Token::Redirect(RedirectOp::Out)
                            }
                        }
                        '<' => {
                            self.advance();
                            Token::Redirect(RedirectOp::In)
                        }
                        '2' => {
                            if self.peek_at(1) == Some('>') {
                                self.advance();
                                self.advance();
                                Token::Redirect(RedirectOp::Err)
                            } else {
                                Token::Word(self.read_word()?)
                            }
                        }
                        '\'' | '"' => Token::Word(self.read_quoted_string()?),
                        '$' => Token::Word(self.read_word()?),
                        '#' => {
                            self.skip_comment();
                            continue;
                        }
                        _ => Token::Word(self.read_word()?),
                    };
                    tokens.push(token);
                }
            }
        }
        Ok(tokens)
    }

    fn peek(&mut self) -> Option<char> {
        self.input.peek().copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.raw.chars().nth(self.pos + offset)
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.next();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn read_word(&mut self) -> Result<String, String> {
        let mut word = String::new();
        loop {
            match self.peek() {
                None => break,
                Some(ch) => match ch {
                    ' ' | '\t' | '\n' | '|' | '&' | ';' | '>' | '<' => break,
                    '\'' | '"' => {
                        let quoted = self.read_quoted_string()?;
                        word.push_str(&quoted);
                    }
                    '$' => {
                        self.advance();
                        let expanded = self.expand_variable()?;
                        word.push_str(&expanded);
                    }
                    '\\' => {
                        self.advance();
                        if let Some(escaped) = self.advance() {
                            word.push(escaped);
                        }
                    }
                    _ => {
                        word.push(ch);
                        self.advance();
                    }
                },
            }
        }
        Ok(word)
    }

    fn read_quoted_string(&mut self) -> Result<String, String> {
        let quote = self.advance().ok_or("unterminated string")?;
        let mut result = String::new();

        loop {
            match self.advance() {
                None => return Err(format!("unterminated {} string", quote)),
                Some(ch) if ch == quote => break,
                Some('\\') if quote == '"' => {
                    if let Some(escaped) = self.advance() {
                        match escaped {
                            'n' => result.push('\n'),
                            't' => result.push('\t'),
                            'r' => result.push('\r'),
                            '\\' | '"' | '$' | '`' => result.push(escaped),
                            _ => {
                                result.push('\\');
                                result.push(escaped);
                            }
                        }
                    }
                }
                Some('$') if quote == '"' => {
                    let expanded = self.expand_variable()?;
                    result.push_str(&expanded);
                }
                Some(ch) => result.push(ch),
            }
        }
        Ok(result)
    }

    fn expand_variable(&mut self) -> Result<String, String> {
        match self.peek() {
            Some('{') => {
                self.advance();
                let mut var_name = String::new();
                loop {
                    match self.advance() {
                        Some('}') => break,
                        Some(ch) if ch.is_alphanumeric() || ch == '_' => var_name.push(ch),
                        Some(_) => return Err("invalid character in variable name".to_string()),
                        None => return Err("unterminated variable expansion".to_string()),
                    }
                }
                Ok(format!("${{{}}}", var_name))
            }
            Some(ch) if ch.is_alphabetic() || ch == '_' => {
                let mut var_name = String::new();
                while let Some(ch) = self.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        var_name.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }
                Ok(format!("${}", var_name))
            }
            _ => Ok("$".to_string()),
        }
    }
}

pub fn keyword_or_word(word: &str) -> Token {
    match word {
        "if" => Token::If,
        "then" => Token::Then,
        "elif" => Token::Elif,
        "else" => Token::Else,
        "fi" => Token::Fi,
        "for" => Token::For,
        "in" => Token::In,
        "do" => Token::Do,
        "done" => Token::Done,
        "while" => Token::While,
        _ => Token::Word(word.to_string()),
    }
}

pub fn tokenize_with_keywords(input: &str) -> Result<Vec<Token>, String> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize()?;
    Ok(tokens
        .into_iter()
        .map(|token| {
            if let Token::Word(ref word) = token {
                keyword_or_word(word)
            } else {
                token
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let mut lexer = Lexer::new("echo hello");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_pipe() {
        let mut lexer = Lexer::new("ls | grep foo");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("ls".to_string()),
                Token::Pipe,
                Token::Word("grep".to_string()),
                Token::Word("foo".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_logical_operators() {
        let mut lexer = Lexer::new("cmd1 && cmd2 || cmd3");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("cmd1".to_string()),
                Token::And,
                Token::Word("cmd2".to_string()),
                Token::Or,
                Token::Word("cmd3".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_redirections() {
        let mut lexer = Lexer::new("echo hello > file.txt");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello".to_string()),
                Token::Redirect(RedirectOp::Out),
                Token::Word("file.txt".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_append_redirection() {
        let mut lexer = Lexer::new("echo hello >> file.txt");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello".to_string()),
                Token::Redirect(RedirectOp::Append),
                Token::Word("file.txt".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_input_redirection() {
        let mut lexer = Lexer::new("cat < file.txt");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("cat".to_string()),
                Token::Redirect(RedirectOp::In),
                Token::Word("file.txt".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_stderr_redirection() {
        let mut lexer = Lexer::new("cmd 2> error.log");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("cmd".to_string()),
                Token::Redirect(RedirectOp::Err),
                Token::Word("error.log".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_single_quoted_string() {
        let mut lexer = Lexer::new("echo 'hello world'");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello world".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_double_quoted_string() {
        let mut lexer = Lexer::new("echo \"hello world\"");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello world".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_variable_expansion_simple() {
        let mut lexer = Lexer::new("echo $HOME");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("$HOME".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_variable_expansion_braced() {
        let mut lexer = Lexer::new("echo ${HOME}");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("${HOME}".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_semicolon_separator() {
        let mut lexer = Lexer::new("cmd1; cmd2; cmd3");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("cmd1".to_string()),
                Token::Semi,
                Token::Word("cmd2".to_string()),
                Token::Semi,
                Token::Word("cmd3".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_background_operator() {
        let mut lexer = Lexer::new("sleep 10 &");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("sleep".to_string()),
                Token::Word("10".to_string()),
                Token::Amp,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_if_keywords() {
        let tokens = tokenize_with_keywords("if true; then echo yes; fi").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::If,
                Token::Word("true".to_string()),
                Token::Semi,
                Token::Then,
                Token::Word("echo".to_string()),
                Token::Word("yes".to_string()),
                Token::Semi,
                Token::Fi,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_for_keywords() {
        let tokens = tokenize_with_keywords("for i in 1 2 3; do echo $i; done").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::For,
                Token::Word("i".to_string()),
                Token::In,
                Token::Word("1".to_string()),
                Token::Word("2".to_string()),
                Token::Word("3".to_string()),
                Token::Semi,
                Token::Do,
                Token::Word("echo".to_string()),
                Token::Word("$i".to_string()),
                Token::Semi,
                Token::Done,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_while_keywords() {
        let tokens = tokenize_with_keywords("while true; do echo loop; done").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::While,
                Token::Word("true".to_string()),
                Token::Semi,
                Token::Do,
                Token::Word("echo".to_string()),
                Token::Word("loop".to_string()),
                Token::Semi,
                Token::Done,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_complex_pipeline() {
        let mut lexer = Lexer::new("cat file.txt | grep pattern | sort | uniq");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("cat".to_string()),
                Token::Word("file.txt".to_string()),
                Token::Pipe,
                Token::Word("grep".to_string()),
                Token::Word("pattern".to_string()),
                Token::Pipe,
                Token::Word("sort".to_string()),
                Token::Pipe,
                Token::Word("uniq".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_comment() {
        let mut lexer = Lexer::new("echo hello # this is a comment\necho world");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello".to_string()),
                Token::Newline,
                Token::Word("echo".to_string()),
                Token::Word("world".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_newline() {
        let mut lexer = Lexer::new("cmd1\ncmd2");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("cmd1".to_string()),
                Token::Newline,
                Token::Word("cmd2".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_escape_character() {
        let mut lexer = Lexer::new("echo hello\\ world");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello world".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_double_quote_escape() {
        let mut lexer = Lexer::new("echo \"hello\\nworld\"");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello\nworld".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_variable_in_double_quotes() {
        let mut lexer = Lexer::new("echo \"Hello $USER\"");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("Hello $USER".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_multiple_redirections() {
        let mut lexer = Lexer::new("cmd > out.txt 2> err.txt");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("cmd".to_string()),
                Token::Redirect(RedirectOp::Out),
                Token::Word("out.txt".to_string()),
                Token::Redirect(RedirectOp::Err),
                Token::Word("err.txt".to_string()),
                Token::Eof,
            ]
        );
    }
}
