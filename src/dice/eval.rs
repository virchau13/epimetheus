// TODO FIXME rate limit this. somehow.
// - explode should terminate.
use crate::dice::{
    lex::{Op, Token},
    parse::ParseIns,
    value::{array_partition_idx, RVal, Value},
};

use super::value::AsyncTryInto;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Evaluator {}

impl Evaluator {
    fn new() -> Self {
        Self {}
    }
}

impl ParseIns for Evaluator {
    type Value = Value;

    async fn literal<'t>(&self, v: crate::dice::lex::Token<'t>) -> Result<Self::Value, String> {
        match v {
            Token::Number(x) => Ok(Value::Int(x.into())),
            Token::Ident(_i) => todo!("resolve identifiers"),
            Token::Eof => Err(format!("incomplete expression")),
            _ => Err(format!("invalid literal {:?}", v)),
        }
    }

    async fn binop(&self, left: Self::Value, right: Self::Value, c: Op) -> Result<Self::Value, String> {
        match c {
            Op::Plus => Ok(left.add(right).await),
            Op::Minus => Ok(left.sub(right).await),
            Op::Star => Ok(left.mul(right).await),
            Op::Comma => match (left.resolve().await, right.resolve().await) {
                (RVal::Array(mut a), RVal::Array(mut b)) => {
                    a.append(&mut b);
                    Ok(Value::Array(a))
                }
                (RVal::Array(mut a), v) => {
                    a.push(v.unresolve());
                    Ok(Value::Array(a))
                }
                (v, RVal::Array(mut a)) => {
                    a.insert(0, v.unresolve());
                    Ok(Value::Array(a))
                }
                (v, w) => Ok(Value::Array(vec![v.unresolve(), w.unresolve()])),
            },
            Op::Semicolon => /* lmfao */ Ok(right),
            Op::Equal => Ok(left.op_eq(right).await),
            Op::Or => Ok(left.op_or(right).await),
            Op::And => {
                todo!()
            }
            _ => Err(format!("invalid infix operator {:?}", c)),
        }
    }

    async fn pfxop(&self, inner: Self::Value, c: Op) -> Result<Self::Value, String> {
        match c {
            Op::Plus => Ok(inner),
            Op::Minus => Ok(inner.neg().await),
            Op::Comma => {
                /* Enlist! */
                Ok(Value::Array(vec![inner]))
            }
            _ => Err(format!("invalid prefix operator {:?}", c)),
        }
    }

    async fn sfxop(&self, inner: Self::Value, c: Op) -> Result<Self::Value, String> {
        match c {
            Op::Percent => Ok(inner.div(100.into()).await),
            Op::Bang => {
                /* explode! */
                match inner {
                    Value::LazyDice {
                        num,
                        sides,
                        lowest_idx,
                        highest_idx,
                        mut explode,
                    } => Ok(Value::LazyDice {
                        num,
                        sides,
                        lowest_idx,
                        highest_idx,
                        explode: {
                            explode.push(sides);
                            explode
                        },
                    }),
                    Value::Int(_) => Err(format!("factorial isn't implemented yet, sorry :P")),
                    Value::Float(_) => Err(format!(
                        "floating point factorial isn't implemented yet, sorry :P"
                    )),
                    Value::Array(_) => {
                        Err(format!("the operator `!` is not defined on arrays yet"))
                    }
                }
            }
            _ => Err(format!("invalid suffix operator {:?}", c)),
        }
    }

    async fn dice(
        &mut self,
        num: Option<Self::Value>,
        sides: Self::Value,
    ) -> Result<Self::Value, String> {
        const DICE_LIMIT_SIDES: u32 = 65535;
        // TODO: large dice optimization
        let num: u32 = match num {
            Some(nv) => nv
                .to_i32()
                .await
                .and_then(|v| u32::try_from(v).map_err(|_| format!("negative number of dice")))
                .and_then(|v| {
                    if v > DICE_LIMIT_SIDES {
                        Err(format!("too many dice: {v} > {DICE_LIMIT_SIDES}"))
                    } else {
                        Ok(v)
                    }
                }),
            None => Ok(1),
        }
        .map_err(|e| format!("invalid number of dice: {:?}", e))?;
        let sides: u32 = sides
            .to_i32()
            .await
            .and_then(|v| u32::try_from(v).map_err(|_| format!("negative number of sides")))
            .map_err(|e| format!("invalid number of sides: {:?}", e))?;
        Ok(Value::LazyDice {
            num,
            sides,
            lowest_idx: 0,
            highest_idx: num - 1,
            explode: vec![],
        })
    }

    async fn keep_highest(
        &mut self,
        dice: Self::Value,
        keep: Self::Value,
    ) -> Result<Self::Value, String> {
        let kh: u32 = keep
            .to_i32()
            .await
            .and_then(|v| v.try_into().map_err(|_| format!("is negative {v}")))
            .map_err(|e| format!("invalid keep-highest criterion: {e}"))?;
        match dice {
            Value::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx,
                explode,
            } => Ok(Value::LazyDice {
                num,
                sides,
                lowest_idx: lowest_idx.max(highest_idx.saturating_sub(kh) + 1),
                highest_idx,
                explode,
            }),
            Value::Int(_) => Err(format!("keep-highest operation is invalid on integers")),
            Value::Float(_) => Err(format!("keep-highest operation is invalid on numbers")),
            Value::Array(a) => {
                let idx = a.len() - kh as usize;
                Ok(Value::Array(array_partition_idx(a, idx, false).await?))
            }
        }
    }

    async fn keep_lowest(&mut self, dice: Self::Value, keep: Self::Value) -> Result<Self::Value, String> {
        let kl: u32 = keep
            .to_i32()
            .await
            .and_then(|v| v.try_into().map_err(|_| format!("is negative {v}")))
            .map_err(|e| format!("invalid keep-lowest criterion: {e}"))?;
        match dice {
            Value::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx,
                explode,
            } => Ok(Value::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx: highest_idx.min((lowest_idx + kl).saturating_sub(1)),
                explode,
            }),
            Value::Int(_) => Err(format!("keep-lowest operation is invalid on integers")),
            Value::Float(_) => Err(format!("keep-lowest operation is invalid on numbers")),
            Value::Array(a) => {
                let idx = kl as usize;
                Ok(Value::Array(array_partition_idx(a, idx, true).await?))
            }
        }
    }

    async fn explode(&mut self, dice: Self::Value, inner: Self::Value) -> Result<Self::Value, String> {
        match dice {
            Value::Int(_) => Err(format!("cannot explode integers")),
            Value::Float(_) => Err(format!("cannot explode numbers")),
            Value::Array(_) => Err(format!("cannot explode arrays")),
            Value::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx,
                mut explode,
            } => Ok(Value::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx,
                explode: {
                    let mut res = match inner.resolve().await {
                        r @ (RVal::Int(_) | RVal::Float(_)) => {
                            match r.to_i32().ok().and_then(|x| u32::try_from(x).ok()) {
                                Some(x) => vec![x],
                                None => vec![],
                            }
                        }
                        r @ RVal::Array(_) => <RVal as AsyncTryInto<Vec<u32>>>::async_try_into(r).await?,
                    };
                    explode.append(&mut res);
                    explode
                },
            }),
        }
    }
}

pub async fn eval(s: &str) -> Result<RVal, String> {
    Ok(crate::dice::parse::run_parser(s, Evaluator::new()).await?.resolve().await)
}

#[tokio::test]
async fn eval_test() {
    macro_rules! good {
        ($x:expr, $y:literal / $z:literal) => {
            let m;
            if let RVal::Float(m1) = eval($x).await.unwrap() {
                m = m1;
            } else {
                panic!();
            }
            let r = ($y / $z);
            if (m - r).abs() >= f64::EPSILON {
                panic!("{} != {}", m, r);
            }
        };
        ($x:expr, $y:literal) => {
            assert_eq!(eval($x).await.unwrap().to_i32().unwrap(), $y)
        };
        ($x:expr, #$y:expr) => {
            assert_eq!(eval($x).await.unwrap(), $y.into())
        };
    }

    macro_rules! bad {
        ($x:expr) => {
            let e = eval($x).await;
            if e.is_ok() {
                panic!("no error thrown for {:?}", e.unwrap());
            }
        };
    }

    good!("2", 2);
    good!("414", 414);
    good!("3+4", 7);
    good!("3-4", -1);
    good!("3+4+5-6+7", 13);
    good!("3*4", 12);
    good!("1+2*3", 7);
    good!("1*2+3", 5);
    good!("1+2*3+4", 11);
    good!("1*2+3*4", 14);
    good!("-3", -3);
    good!("-3+4", 1);
    good!("+3-4", -1);
    good!("++++++++++3-4", -1);
    good!("++++++++++3-++++++++4", -1);
    good!("--3 - --4", -1);
    good!("3%", 3. / 100.);
    good!("4+3%", 403. / 100.);
    good!("3%+4", 403. / 100.);
    good!("3%%%", 3. / 1_000_000.);
    good!("(3+4)*5", 35);
    good!("5*(3+4)", 35);
    good!("-(3+4)", -7);
    good!("++++(3+4)%", 7. / 100.);

    good!("3d4", 8);
    good!("15d1", 15);
    bad!("4810954d1093491");
    good!("65535d65535", 2140437410);
    good!("-3d4+7", -1);

    good!("d2", 1);
    good!("d2+d2", 2);
    good!("2d2", 3);

    good!("4d1KH3", 3);
    good!("4d1K0", 0);
    good!("15d1h1", 1);
    good!("3d4KH2", 6);
    good!("3d4KL2", 5);

    good!("2d2!", 10);
    good!("2d2!(2)!", 10);
    good!("2d2!(,2)!", 10);

    good!("d3!", 1);
    good!("2d3!", 9);
    good!("2d3!(2,3)!", 12);

    good!("d4!", 2);
    good!("d4!(2)!", 5);

    good!(",2", #vec![2]);
    good!("1,2", #vec![1,2]);
    good!("1,2,3", #vec![1,2,3]);
    good!(",4-,3", #vec![vec![1]]);

    good!("12d6; d4", 2);
    good!("d4;d4;d4;d4;d4", 2);
}
