#[derive(Debug, PartialEq)]
#[rustfmt::skip]
pub enum Tokens {
    ILLEGAL, EOF, IDENT(String),

    Number(f64), String(String), Boolean(bool), Comment(String),

    Assign, Plus, Minus, Bang, Asterisk, Slash, Percent,

    Quote, SingleQuote,

    Comma, Colon, Semicolon,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,

    LT, GT, LTE, GTE, EQ, NEQ,

    Let, If, Else, Return, Function,
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub token_type: Tokens,
    pub position: (usize, usize),
}

impl Default for Token {
    fn default() -> Self {
        Token {
            token_type: Tokens::EOF,
            position: (0, 0),
        }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Token {:?} at {:?}", self.token_type, self.position)
    }
}

impl<T: Into<String>> From<T> for Tokens {
    fn from(s: T) -> Self {
        match s.into().as_str() {
            "let" => Tokens::Let,
            "if" => Tokens::If,
            "else" => Tokens::Else,
            "return" => Tokens::Return,
            "fn" => Tokens::Function,
            s => Tokens::IDENT(s.to_string()),
        }
    }
}

impl Token {
    pub fn new(token_type: Tokens, position: (usize, usize)) -> Self {
        Token { token_type, position }
    }

    pub fn stringify(&self) -> String {
        macro_rules! to_s {
            ($( $x:ident )*) => {
                match &self.token_type {
                    $( Tokens::$x(x) => x.to_string(), )*
                    _ => format!("{:?}", self.token_type)
                }
            }
        }

        to_s! { IDENT String Number Boolean Comment }
    }
}
