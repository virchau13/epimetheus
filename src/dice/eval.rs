use crate::dice::{
    lex::{Op, Token},
    parse::ParseIns,
    value::{array_partition_idx, LazyValue, RVal},
};

use super::{value::RRVal, vec_async_map, vec_into};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Evaluator {}

impl Evaluator {
    fn new() -> Self {
        Self {}
    }
}

impl ParseIns for Evaluator {
    type Value = LazyValue;

    async fn literal<'t>(&self, v: crate::dice::lex::Token<'t>) -> Result<Self::Value, String> {
        match v {
            Token::Number(x) => Ok(LazyValue::Int(x.into())),
            Token::Char(c) => Ok(LazyValue::Char(c)),
            Token::Str(s) => Ok(LazyValue::Array(
                s.chars().map(LazyValue::Char).collect(),
            )),
            Token::Ident(_i) => todo!("resolve identifiers"),
            Token::Eof => Err("incomplete expression".to_string()),
            _ => Err(format!("invalid literal `{}`", v)),
        }
    }

    async fn binop(
        &self,
        left: Self::Value,
        right: Self::Value,
        c: Op,
    ) -> Result<Self::Value, String> {
        match c {
            Op::Plus => Ok(left.add(right).await),
            Op::Minus => Ok(left.sub(right).await),
            Op::Star => Ok(left.mul(right).await),
            Op::Comma => match (left.resolve().await, right.resolve().await) {
                (RVal::Array(mut a), RVal::Array(mut b)) => {
                    a.append(&mut b);
                    Ok(LazyValue::Array(a))
                }
                (RVal::Array(mut a), v) => {
                    a.push(v.into());
                    Ok(LazyValue::Array(a))
                }
                (v, RVal::Array(mut a)) => {
                    a.insert(0, v.into());
                    Ok(LazyValue::Array(a))
                }
                (v, w) => Ok(LazyValue::Array(vec![v.into(), w.into()])),
            },
            Op::Semicolon =>
            /* lmfao */
            {
                Ok(right)
            }
            Op::Equal => Ok(left.op_eq(right).await),
            Op::Or => Ok(left.op_or(right).await),
            Op::And => {
                todo!()
            }
            _ => Err(format!("invalid infix operator `{}`", c.as_str())),
        }
    }

    async fn pfxop(&self, inner: Self::Value, c: Op) -> Result<Self::Value, String> {
        match c {
            Op::Plus => Ok(inner),
            Op::Minus => Ok(inner.neg().await),
            Op::Comma => {
                /* Enlist! */
                Ok(LazyValue::Array(vec![inner]))
            }
            _ => Err(format!("invalid prefix operator `{}`", c.as_str())),
        }
    }

    async fn sfxop(&self, inner: Self::Value, c: Op) -> Result<Self::Value, String> {
        match c {
            Op::Percent => Ok(inner.div(100.into()).await),
            Op::Bang => {
                /* explode! */
                match inner {
                    LazyValue::LazyDice {
                        num,
                        sides,
                        lowest_idx,
                        highest_idx,
                        mut explode,
                    } => {
                        let l = sides.len();
                        Ok(LazyValue::LazyDice {
                            num,
                            sides,
                            lowest_idx,
                            highest_idx,
                            explode: {
                                explode.push(RRVal::Int(l.into()));
                                explode
                            },
                        })
                    }
                    LazyValue::Int(_) => {
                        Err("factorial isn't implemented yet, sorry :P".to_string())
                    }
                    LazyValue::Float(_) => {
                        Err("floating point factorial isn't implemented yet, sorry :P".to_string())
                    }
                    LazyValue::Char(_) => Err("you can't explode a character".to_string()),
                    LazyValue::Array(_) => {
                        Err("the operator `!` is not defined on arrays yet".to_string())
                    }
                }
            }
            _ => Err(format!("invalid suffix operator `{}`", c.as_str())),
        }
    }

    async fn dice(
        &mut self,
        num: Option<Self::Value>,
        sides_raw: Self::Value,
    ) -> Result<Self::Value, String> {
        const DICE_LIMIT_SIDES: u32 = 65535;
        // TODO: large dice optimization
        let num: u32 = match num {
            Some(nv) => nv
                .into_i32()
                .await
                .and_then(|v| u32::try_from(v).map_err(|_| "negative number of dice".to_string()))
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
        let sides;
        if let LazyValue::Array(a) = sides_raw {
            sides = RRVal::deep_resolve_vec(a).await;
        } else {
            let sides_num: u32 = sides_raw
                .into_i32()
                .await
                .and_then(|v| u32::try_from(v).map_err(|_| "negative number of sides".to_string()))
                .map_err(|e| format!("invalid number of sides: {:?}", e))?;
            sides = (1..=sides_num).map(|y| RRVal::Int(y.into())).collect();
        }
        Ok(LazyValue::LazyDice {
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
            .into_i32()
            .await
            .and_then(|v| v.try_into().map_err(|_| format!("is negative {v}")))
            .map_err(|e| format!("invalid keep-highest criterion: {e}"))?;
        match dice {
            LazyValue::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx,
                explode,
            } => Ok(LazyValue::LazyDice {
                num,
                sides,
                lowest_idx: lowest_idx.max(highest_idx.saturating_sub(kh) + 1),
                highest_idx,
                explode,
            }),
            LazyValue::Int(_) => Err("keep-highest operation is invalid on integers".to_string()),
            LazyValue::Float(_) => Err("keep-highest operation is invalid on numbers".to_string()),
            LazyValue::Char(_) => {
                Err("keep-highest operation is invalid on characters".to_string())
            }
            LazyValue::Array(a) => {
                let idx = a.len() - kh as usize;
                Ok(LazyValue::Array(vec_into(
                    array_partition_idx(
                        vec_async_map(a, LazyValue::deep_resolve).await,
                        idx,
                        false,
                    )
                    .await?,
                )))
            }
        }
    }

    async fn keep_lowest(
        &mut self,
        dice: Self::Value,
        keep: Self::Value,
    ) -> Result<Self::Value, String> {
        let kl: u32 = keep
            .into_i32()
            .await
            .and_then(|v| v.try_into().map_err(|_| format!("is negative {v}")))
            .map_err(|e| format!("invalid keep-lowest criterion: {e}"))?;
        match dice {
            LazyValue::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx,
                explode,
            } => Ok(LazyValue::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx: highest_idx.min((lowest_idx + kl).saturating_sub(1)),
                explode,
            }),
            LazyValue::Int(_) => Err("keep-lowest operation is invalid on integers".to_string()),
            LazyValue::Float(_) => Err("keep-lowest operation is invalid on numbers".to_string()),
            LazyValue::Char(_) => Err("keep-lowest operation is invalid on characters".to_string()),
            LazyValue::Array(a) => {
                let idx = kl as usize;
                Ok(LazyValue::Array(vec_into(
                    array_partition_idx(vec_async_map(a, LazyValue::deep_resolve).await, idx, true)
                        .await?,
                )))
            }
        }
    }

    async fn explode(
        &mut self,
        dice: Self::Value,
        inner: Self::Value,
    ) -> Result<Self::Value, String> {
        match dice {
            LazyValue::Int(_) => Err("cannot explode integers".to_string()),
            LazyValue::Float(_) => Err("cannot explode numbers".to_string()),
            LazyValue::Array(_) => Err("cannot explode arrays".to_string()),
            LazyValue::Char(_) => Err("cannot explode characters".to_string()),
            LazyValue::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx,
                mut explode,
            } => Ok(LazyValue::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx,
                explode: {
                    let mut res = match inner.deep_resolve().await {
                        RRVal::Array(a) => a,
                        r => vec![r],
                    };
                    explode.append(&mut res);
                    explode
                },
            }),
        }
    }

    async fn mk_array(&mut self, arr: Vec<Self::Value>) -> Result<Self::Value, String> {
        Ok(LazyValue::Array(arr))
    }
}

pub async fn eval(s: &str) -> Result<RVal, String> {
    Ok(crate::dice::parse::run_parser(s, Evaluator::new())
        .await?
        .resolve()
        .await)
}

#[tokio::test]
async fn eval_positive_test() {
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
            assert_eq!(eval($x).await.unwrap().into_i32().unwrap(), $y)
        };
        ($x:expr, #$y:expr) => {
            assert_eq!(eval($x).await.unwrap(), $y.into())
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

    good!("3d4", 7);
    good!("15d1", 15);
    good!("65535d65535", 2143601837);
    good!("-3d4+7", 0);

    good!("d2", 2);
    good!("d2+d2", 4);
    good!("2d2", 4);
    good!("10d2", 15);

    good!("4d1KH3", 3);
    good!("4d1K0", 0);
    good!("15d1h1", 1);
    good!("3d4KH2", 6);
    good!("3d4KL2", 4);

    good!("2d2!", 8);
    good!("2d2!(2)!", 8);
    good!("2d2!(,2)!", 8);

    good!("d3!", 5);
    good!("2d3", 5);
    good!("2d3!", 6);
    good!("2d3!(2,3)!", 12);

    good!("d4!", 3);
    good!("d4!(3)!", 7);

    good!(",2", #vec![2]);
    good!("1,2", #vec![1,2]);
    good!("1,2,3", #vec![1,2,3]);
    good!(",4-,3", #vec![vec![1]]);

    good!("12d6; d4", 3);
    good!("d4;d4;d4;d4;d4", 3);

    good!("d(,2)", 2);
    good!("d(0,0,0,0)", 0);
    good!("d(0,0,0,0,0,7,0,0,0,0,0)", 0);

    good!("[1,2]", #vec![1,2]);
    good!("[1]", #vec![1]);
    good!("[]", #Vec::<i32>::new());
    good!("[1,]", #vec![1]);
    good!("[1, 2,]", #vec![1,2]);

    good!(r#"d["left", "right", "up", "down"]"#, #"up");
    // This is *intentional*. (For now.)
    good!("d[]", 0);
    good!("d0", 0);
}

#[tokio::test]
async fn eval_negative_test() {
    macro_rules! bad {
        ($x:expr) => {
            let e = eval($x).await;
            if e.is_ok() {
                panic!("no error thrown for {:?}", e.unwrap());
            }
        };
    }

    bad!("4810954d1093491"); // dice x > 65535
    bad!("2 2");
    bad!("$2");
    bad!("2$");
}
