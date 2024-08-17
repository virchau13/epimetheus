use phf::phf_map;
use smol_str::SmolStr;

use super::value::escape_string_for_discord;

pub struct Lexer<'s> {
    s: &'s str,
    prev_s: &'s str,
}

impl<'s> Lexer<'s> {
    pub fn new(s: &'s str) -> Self {
        Self { s, prev_s: s }
    }

    fn peek(&self) -> char {
        self.s.chars().next().unwrap_or('\0')
    }

    fn advance(&mut self) {
        let mut cs = self.s.chars();
        cs.next();
        self.s = cs.as_str();
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() == c {
            self.advance();
            true
        } else {
            false
        }
    }

    fn so_far(&self) -> &'s str {
        // cough, cough.
        &self.prev_s[..self.s.as_ptr() as usize - self.prev_s.as_ptr() as usize]
    }

    fn reset(&mut self) {
        self.prev_s = self.s;
    }

    fn tok<'a>(&'a mut self, t: Token<'s>) -> Option<Token<'s>> {
        self.reset();
        Some(t)
    }
}

impl<'s> Iterator for Lexer<'s> {
    type Item = Token<'s>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.reset();
            let c = self.peek();
            return match c {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                    continue;
                }
                '0'..='9' => {
                    while self.peek().is_ascii_digit() {
                        self.advance();
                    }
                    let text = self.so_far();
                    self.tok(match text.parse::<u64>() {
                        Ok(n) => Token::Number(n),
                        Err(_) => Token::UnexpectedStr(text),
                    })
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    while matches!(self.peek(), 'a'..='z' | 'A'..='Z' | '_') {
                        self.advance();
                    }
                    self.tok(Token::Ident(self.so_far()))
                }
                '\'' => {
                    self.advance();
                    let c = self.peek();
                    self.advance();
                    // TODO support escapes
                    let closing = self.peek();
                    self.advance();
                    if closing == '\'' {
                        self.tok(Token::Char(c))
                    } else {
                        self.tok(Token::UnexpectedChar(closing))
                    }
                }
                '"' => {
                    self.advance();
                    self.reset();
                    // TODO escapes
                    while self.peek() != '"' && self.peek() != '\0' {
                        self.advance();
                    }
                    let res = self.so_far();
                    self.advance();
                    self.tok(Token::Str(SmolStr::new(res)))
                }
                '!' => {
                    self.advance();
                    let n = self.peek();
                    if n == '(' {
                        self.advance();
                        self.tok(Token::Op(Op::BangLPar))
                    } else {
                        self.tok(Token::Op(Op::Bang))
                    }
                }
                ')' => {
                    self.advance();
                    if self.peek() == '!' {
                        self.advance();
                        self.tok(Token::Op(Op::RParBang))
                    } else {
                        self.tok(Token::Op(Op::RPar))
                    }
                }
                '=' => {
                    self.advance();
                    if self.peek() == '=' {
                        self.advance();
                        self.tok(Token::Op(Op::Equal))
                    } else {
                        self.tok(Token::Op(Op::Assign))
                    }
                }
                '|' => {
                    self.advance();
                    if self.eat('|') {
                        self.tok(Token::Op(Op::Or))
                    } else {
                        None
                    }
                }
                '&' => {
                    self.advance();
                    if self.eat('&') {
                        self.tok(Token::Op(Op::And))
                    } else {
                        None
                    }
                }
                _ => {
                    if let Some(v) = OP_MAP.get(&c) {
                        self.advance();
                        self.tok(Token::Op(*v))
                    } else if c != '\0' {
                        self.tok(Token::UnexpectedChar(c))
                    } else {
                        None
                    }
                }
            };
        }
    }
}

const OP_MAP: phf::Map<char, Op> = phf_map! {
    '+' => Op::Plus,
    '-' => Op::Minus,
    '*' => Op::Star,
    '%' => Op::Percent,
    '(' => Op::LPar,
    ')' => Op::RPar,
    '!' => Op::Bang,
    ',' => Op::Comma,
    ';' => Op::Semicolon,
    '=' => Op::Assign,
    '[' => Op::LBrack,
    ']' => Op::RBrack,
    '/' => Op::Slash,
    '#' => Op::Hash,
};

macro_rules! declare_ops {
    { $($n:ident),+ $(,)? } => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        pub enum Op {
            $($n),+
        }

        impl Op {
            pub fn list_of_ops() -> &'static [Op] {
                return &[
                    $(Self::$n),+
                ]
            }
        }
    }
}

declare_ops! {
    Plus,
    Minus,
    Star,
    Percent,
    LPar,
    RPar,
    Bang,
    BangLPar,
    RParBang,
    Comma,
    Semicolon,
    Assign,
    Equal,
    Or,
    And,
    LBrack,
    RBrack,
    Slash,
    Hash,
}

impl Op {
    pub fn as_str(&self) -> &'static str {
        match self {
            Op::Plus => "+",
            Op::Minus => "-",
            Op::Star => "*",
            Op::Percent => "%",
            Op::LPar => "(",
            Op::RPar => ")",
            Op::Bang => "!",
            Op::BangLPar => "!(",
            Op::RParBang => ")!",
            Op::Comma => ",",
            Op::Semicolon => ";",
            Op::Assign => "=",
            Op::Equal => "==",
            Op::Or => "||",
            Op::And => "&&",
            Op::LBrack => "[",
            Op::RBrack => "]",
            Op::Slash => "/",
            Op::Hash => "#",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token<'s> {
    Number(u64),
    Op(Op),
    Ident(&'s str),
    Str(SmolStr),
    Char(char),
    UnexpectedStr(&'s str),
    UnexpectedChar(char),
    Eof,
}

impl<'s> std::fmt::Display for Token<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Number(n) => n.fmt(f),
            Token::Op(op) => write!(f, "{}", op.as_str()),
            Token::Ident(id) => write!(f, "{}", id),
            Token::Str(s) => write!(f, "{}", escape_string_for_discord(s)),
            Token::Char(c) => write!(f, "'{}'", c.escape_default()),
            Token::UnexpectedStr(s) => write!(f, "{}", s.escape_default()),
            Token::UnexpectedChar(c) => write!(f, "{}", c.escape_default()),
            Token::Eof => write!(f, "end-of-input"),
        }
    }
}

#[test]
pub fn lex_test() {
    pub fn l(s: &str) -> Vec<Token> {
        Lexer::new(s).collect()
    }
    assert_eq!(l("2"), vec![Token::Number(2)]);
    assert_eq!(
        l("21 41849148 3 99"),
        vec![
            Token::Number(21),
            Token::Number(41849148),
            Token::Number(3),
            Token::Number(99)
        ]
    );
    assert_eq!(
        l("-ident14871 + 14871"),
        vec![
            Token::Op(Op::Minus),
            Token::Ident("ident"),
            Token::Number(14871),
            Token::Op(Op::Plus),
            Token::Number(14871),
        ]
    );
    assert_eq!(
        l("d2!"),
        vec![Token::Ident("d"), Token::Number(2), Token::Op(Op::Bang)]
    );
    assert_eq!(
        l("(3+4)*5"),
        vec![
            Token::Op(Op::LPar),
            Token::Number(3),
            Token::Op(Op::Plus),
            Token::Number(4),
            Token::Op(Op::RPar),
            Token::Op(Op::Star),
            Token::Number(5)
        ]
    );
}
