use super::token::*;

/// **Lexer receives the source code in the from of a string and returns tokenized data.**
///
/// for example:
/// ```rust
/// use sntk_core::tokenizer::{token::*, lexer::*};
///
/// let mut lexer = Lexer::new(r#"let x_32z = y != "Hello, World\n";"#);
/// let mut token = Token::new(Tokens::ILLEGAL(String::new()), (0, 0));
///
/// while token.token_type != Tokens::EOF {
///     token = lexer.next_token();
///     println!("{}", token);
/// }
/// ```
#[derive(Debug)]
pub struct Lexer {
    pub input: String,
    pub position: usize,
    pub read_position: usize,
    pub current_char: char,
    pub current_position: (usize, usize),
}

impl Default for Lexer {
    fn default() -> Self {
        Self {
            input: String::new(),
            position: 0,
            read_position: 0,
            current_char: '\0',
            current_position: (1, 0),
        }
    }
}

impl Lexer {
    /// **Creates a new Lexer instance.**
    /// it takes an argument of type `&str` or `String` (`Into<String>`).
    pub fn new<T>(input: T) -> Self
    where
        T: Into<String>,
    {
        let mut lexer = Lexer {
            input: input.into(),
            ..Default::default()
        };

        lexer.read_char();
        lexer
    }

    /// **Reads the next character from the input string.**
    /// if `read_position` is greater than the length of the input string, it sets `current_char` to `'\0'` (EOF).
    pub fn read_char(&mut self) {
        if self.read_position >= self.input.len() {
            self.current_char = '\0';
        } else {
            self.current_char = self.input.chars().nth(self.read_position).unwrap();
        }

        self.position = self.read_position;
        self.read_position += 1;

        self.current_position.1 += 1;
    }

    /// **Peeks the next character from the input string.**
    /// if `read_position` is greater than the length of the input string, it returns `'\0'` (EOF).
    pub fn peek_char(&self) -> char {
        if self.read_position >= self.input.len() {
            '\0'
        } else {
            self.input.chars().nth(self.read_position).unwrap()
        }
    }

    /// **Skips the whitespaces and newlines.**
    ///
    /// if `current_char` is a whitespace or newline, it calls `read_char()` until it finds a non-whitespace character.
    /// also, if current_char is a newline, it increments the `current_position.0` (line number) by 1 and sets `current_position.1` (column number) to 0.
    pub fn skip_whitespace(&mut self) {
        while self.current_char.is_whitespace() {
            if self.current_char == '\n' {
                self.current_position.0 += 1;
                self.current_position.1 = 0;
            }

            self.read_char();
        }
    }

    /// **Read the identifier. it rules are:**
    ///
    /// - starts with a letter or an underscore.
    /// - can contain letters, underscores, and numbers.
    /// - can't be a keyword.
    ///
    /// this follows the `snake_case` naming convention.
    pub fn read_identifier(&mut self) -> String {
        let position = self.position;
        while self.current_char.is_alphanumeric() || self.current_char == '_' {
            self.read_char();
        }

        let result = self.input[position..self.position].to_string();

        macro_rules! replace_all {
            ($s:expr, $($t:expr => $r:expr),*) => {{
                let mut s = String::from($s);
                $( s = s.replace($t, $r); )*
                s
            }};
        }

        replace_all! {
            result,
            "\\r" => "\r",
            "\\t" => "\t",
            "\\n" => "\n",
            "\\\"" => "\"",
            "\\\\" => "\\"
        }
    }

    /// **Read the number. it rules are:**
    ///
    /// - starts with a digit.
    /// - can contain digits and a single dot.
    ///     - can't contain more than one dot.
    pub fn read_number(&mut self) -> f64 {
        let position = self.position;
        let mut has_dot = false;

        while self.current_char.is_numeric() || self.current_char == '.' {
            if self.current_char == '.' {
                if has_dot {
                    break;
                }

                has_dot = true;
            }

            self.read_char();
        }

        self.input[position..self.position].parse().unwrap_or(0.)
    }

    /// **Read the string. it rules are:**
    ///
    /// - starts with a double quote (`"`).
    ///     - ends with a double quote (`"`).
    /// - supports escape sequences.
    /// - supports unicode characters.
    pub fn read_string(&mut self) -> String {
        let position = self.position + 1;
        while self.peek_char() != '"' && self.current_char != '\0' {
            self.read_char();
        }

        self.read_char();
        self.input[position..self.position].to_string()
    }

    /// **Read the next token.**
    ///
    /// it calls `skip_whitespace()` and then checks the `current_char` and returns the corresponding token.
    ///
    /// if `current_char` is not a valid character, it returns `Tokens::ILLEGAL(...)`.
    /// if 'current_char` is `'\0'` (EOF), it returns `Tokens::EOF`.
    pub fn next_token(&mut self) -> Token {
        use super::token::Tokens::*;

        self.skip_whitespace();

        macro_rules! match_token {
            ($($token:expr => $token_type:expr),*) => {
                match self.current_char {
                    $( $token => Token::new($token_type, self.current_position), )*
                    token => Token::new(Tokens::ILLEGAL(token.to_string()), self.current_position)
                }
            }
        }

        macro_rules! next {
            ($n_token:expr => $t_token:expr; $e_token:expr) => {
                if self.peek_char() == $n_token {
                    self.read_char();
                    $t_token
                } else {
                    $e_token
                }
            };
        }

        let token = match_token! {
            '+' => Plus,
            '-' => Minus,
            '*' => Asterisk,
            '/' => Slash,
            '<' => LT,
            '>' => GT,
            ',' => Comma,
            ';' => Semicolon,
            ':' => Colon,
            '(' => LParen,
            ')' => RParen,
            '{' => LBrace,
            '}' => RBrace,
            '[' => LBracket,
            ']' => RBracket,
            '"' => String(self.read_string()),
            '=' => next!('=' => EQ; Assign),
            '!' => next!('=' => NEQ; Bang),
            '\0' => EOF
        };

        match self.current_char {
            c if c.is_alphabetic() => Token::new(Tokens::from(self.read_identifier()), self.current_position),
            c if c.is_numeric() => Token::new(Tokens::Number(self.read_number()), self.current_position),
            _ => {
                self.read_char();
                token
            }
        }
    }
}
