use std::iter::Peekable;

use async_recursion::async_recursion;

use crate::dice::lex::{Lexer, Op, Token};

#[trait_variant::make(ParseIns: Send)]
pub trait UnusedParseIns {
    type Value: std::fmt::Debug + Send;

    async fn literal<'t>(&self, v: Token<'t>) -> Result<Self::Value, String>;
    async fn binop(&self, left: Self::Value, right: Self::Value, c: Op) -> Result<Self::Value, String>;
    async fn pfxop(&self, inner: Self::Value, c: Op) -> Result<Self::Value, String>;
    async fn sfxop(&self, inner: Self::Value, c: Op) -> Result<Self::Value, String>;
    async fn dice(&mut self, num: Option<Self::Value>, sides: Self::Value)
        -> Result<Self::Value, String>;
    async fn keep_highest(&mut self, dice: Self::Value, keep: Self::Value)
        -> Result<Self::Value, String>;
    async fn keep_lowest(&mut self, dice: Self::Value, keep: Self::Value)
        -> Result<Self::Value, String>;
    async fn explode(&mut self, dice: Self::Value, keep: Self::Value) -> Result<Self::Value, String>;
}

pub struct Parser<'s, I: ParseIns> {
    lex: Peekable<Lexer<'s>>,
    ins: I,
}

// stupid
macro_rules! pres {
    () => { Result<I::Value, String> }
}

fn infix_prec(op: Op) -> Option<(u8, u8)> {
    Some(match op {
        Op::Star => (15, 16),
        Op::Plus => (13, 14),
        Op::Minus => (13, 14),
        Op::Comma => (9, 10),
        Op::Equal => (5, 6),
        Op::Or => (3, 4),
        Op::Semicolon => (0, 1),
        _ => return None,
    })
}

fn prefix_prec(op: Op) -> Option<u8> {
    Some(match op {
        Op::Plus => 20,
        Op::Minus => 20,
        Op::Comma => 10,
        _ => return None,
    })
}

fn suffix_prec(op: Op) -> Option<u8> {
    Some(match op {
        Op::Percent => 20,
        Op::Bang => 30,
        _ => return None,
    })
}

impl<'s, I: ParseIns + Send> Parser<'s, I> {
    pub fn new(lex: Lexer<'s>, i: I) -> Self {
        Self {
            lex: lex.peekable(),
            ins: i,
        }
    }

    fn peek(&mut self) -> &Token<'s> {
        self.lex.peek().unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token<'s> {
        self.lex.next().unwrap_or(Token::Eof)
    }

    fn eat<'a>(&mut self, t: &Token<'a>) -> bool {
        if self.peek() == t {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, t: &Token<'_>) -> Result<(), String> {
        if !self.eat(t) {
            Err(format!("expected {:?} but got {:?}", t, self.peek()))
        } else {
            Ok(())
        }
    }

    pub async fn entry(&mut self) -> pres!() {
        self.expr(0).await
    }

    #[async_recursion]
    async fn expr(&mut self, min_prec: u8) -> pres!() {
        let t = self.peek().clone();
        let mut first = match t {
            Token::Op(op) => {
                // TODO treat BangLPar and RParBang as logical not of (expr)
                self.advance();
                if let Some(p) = prefix_prec(op) {
                    let inner = self.expr(p).await?;
                    self.ins.pfxop(inner, op).await?
                } else if op == Op::LPar {
                    let inner = self.expr(0).await?;
                    self.expect(&Token::Op(Op::RPar))?;
                    inner
                } else {
                    return Err(format!("invalid prefix operator {:?}", t));
                }
            }
            Token::Ident("d") => {
                self.advance();
                let p = 40;
                let inner = self.expr(p).await?;
                self.ins.dice(None, inner).await?
            }
            _ => self.literal().await?,
        };
        loop {
            let t = self.peek().clone();
            match t {
                Token::Op(op) => {
                    if let Some(p) = suffix_prec(op) {
                        if min_prec <= p {
                            self.advance();
                            first = self.ins.sfxop(first, op).await?;
                            continue;
                        }
                    } else if op == Op::BangLPar {
                        /* precedence doesn't really work the same way here... */
                        /* it's basically a long suffix operator. */
                        let p = 20;
                        if min_prec <= p {
                            self.advance();
                            let inner = self.expr(0).await?;
                            self.expect(&Token::Op(Op::RParBang))?;
                            first = self.ins.explode(first, inner).await?;
                            continue;
                        }
                    }
                    if let Some((lp, rp)) = infix_prec(op) {
                        if min_prec <= lp {
                            self.advance();
                            let rhs = self.expr(rp).await?;
                            first = self.ins.binop(first, rhs, op).await?;
                            continue;
                        }
                    }
                    return Ok(first);
                }
                Token::Ident("d") => {
                    let (lp, rp) = (39, 40);
                    // state, oh state, oh state
                    if min_prec <= lp {
                        self.advance();
                        let rhs = self.expr(rp).await?;
                        first = self.ins.dice(Some(first), rhs).await?;
                        continue;
                    }
                    return Ok(first);
                }
                Token::Ident("KH" | "kh" | "Kh" | "kH" | "H" | "h" | "K") => {
                    let (lp, rp) = (35, 36);
                    if min_prec <= lp {
                        self.advance();
                        let rhs = self.expr(rp).await?;
                        first = self.ins.keep_highest(first, rhs).await?;
                        continue;
                    }
                    return Ok(first);
                }
                Token::Ident("KL" | "kl" | "Kl" | "kL" | "L" | "l") => {
                    let (lp, rp) = (35, 36);
                    if min_prec <= lp {
                        self.advance();
                        let rhs = self.expr(rp).await?;
                        first = self.ins.keep_lowest(first, rhs).await?;
                        continue;
                    }
                    return Ok(first);
                }
                Token::Eof => return Ok(first),
                _l => {
                    /* probably literal */
                    todo!()
                }
            };
        }
    }

    async fn literal(&mut self) -> pres!() {
        let t = self.peek().clone();
        match self.ins.literal(t).await {
            Ok(r) => {
                self.advance();
                Ok(r)
            }
            Err(e) => Err(e),
        }
    }
}

pub async fn run_parser<I: ParseIns + Send>(s: &str, i: I) -> Result<I::Value, String> {
    Parser::new(Lexer::new(s), i).entry().await
}
