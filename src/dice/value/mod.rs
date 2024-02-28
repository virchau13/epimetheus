mod lazy_value;
pub use lazy_value::LazyValue;

mod rval;
pub use rval::RVal;

mod rrval;
pub use rrval::RRVal;

use rug::Integer;

pub async fn resolve_dice(
    num: u32,
    sides: Vec<RRVal>,
    lowest_idx: u32,
    highest_idx: u32,
    explode: Vec<RRVal>,
) -> RRVal {
    if highest_idx < lowest_idx {
        return RRVal::Int(Integer::ZERO);
    }
    use rand::distributions::{Distribution, Uniform};
    let mut rng = crate::dice::get_rng();
    let between = Uniform::from(0..sides.len());
    async fn do_explode(
        rng: &mut (impl rand::Rng + Send),
        sides: &[RRVal],
        between: &Uniform<usize>,
        explode: &[RRVal],
    ) -> RRVal {
        let mut sum: Option<RRVal> = None;
        /* do-while loop, cough cough... */
        while {
            let i = between.sample(rng);
            let x = &sides[i];
            if let Some(s) = sum {
                sum = Some(s.add(x.clone()).await);
            } else {
                sum = Some(x.clone());
            }
            explode.contains(x)
        } {
            // just in case.
            crate::util::yield_point().await;
        }
        sum.unwrap_or(RRVal::Int(Integer::ZERO))
    }
    if lowest_idx == 0 && highest_idx as usize == sides.len() - 1 {
        let mut sum: Option<RRVal> = None;
        for _ in 0..num {
            let sample = if !explode.is_empty() {
                do_explode(&mut rng, &sides, &between, &explode).await
            } else {
                let i = between.sample(&mut rng);
                sides[i].clone()
            };
            if let Some(s) = sum {
                sum = Some(s.add(sample).await);
            } else {
                sum = Some(sample);
            }
        }
        sum.unwrap_or(RRVal::Int(Integer::ZERO))
    } else {
        let mut res = Vec::new();
        res.reserve_exact(num as usize);
        for _ in 0..num {
            let sample = if !explode.is_empty() {
                do_explode(&mut rng, &sides, &between, &explode).await
            } else {
                let i = between.sample(&mut rng);
                sides[i].clone()
            };
            res.push(sample);
        }
        fn cmp_rrvals(lhs: &RRVal, rhs: &RRVal) -> std::cmp::Ordering {
            use std::cmp::Ordering::*;
            match (lhs, rhs) {
                (RRVal::Int(n), RRVal::Int(m)) => n.cmp(m),
                (RRVal::Int(n), RRVal::Float(f)) => n.partial_cmp(f).unwrap_or(Greater) /* All integers > NaN */,
                (RRVal::Float(f), RRVal::Int(n)) => f.partial_cmp(n).unwrap_or(Less) /* NaN < all integers */,
                (RRVal::Float(a), RRVal::Float(b)) => a.partial_cmp(b).unwrap_or_else(|| {
                    if a.is_nan() {
                        if b.is_nan() {
                            // a, b are both NaN
                            Equal // lmfao
                        } else {
                            // a is the only NaN
                            // all is greater than NaN
                            Greater
                        }
                    } else {
                        // b is the only NaN
                        // NaN is less than all
                        Less
                    }
                }),
                (RRVal::Array(a), RRVal::Array(b)) => {
                    // lexographic comparison
                    if a.len() != b.len() {
                        return a.len().cmp(&b.len());
                    }
                    todo!()
                },
                (v, RRVal::Array(a)) => {
                    // compare by first element
                    if let Some(first) = a.first() {
                        cmp_rrvals(v, first)
                    } else {
                        // empty arrays are below everything else
                        Less
                    }
                },
                (RRVal::Array(a), v) => {
                    // compare by first element
                    if let Some(first) = a.first() {
                        cmp_rrvals(first, v)
                    } else {
                        // empty arrays are below everything else
                        Greater
                    }
                },
            }
        }
        res.sort_unstable_by(cmp_rrvals);
        let mut sum: Option<RRVal> = None;
        for item in res.drain(lowest_idx as usize..=highest_idx as usize) {
            if let Some(s) = sum {
                sum = Some(s.add(item).await);
            } else {
                sum = Some(item);
            }
        }
        sum.unwrap_or(RRVal::Int(Integer::ZERO))
    }
}


impl From<i32> for LazyValue {
    fn from(value: i32) -> Self {
        Self::Int(value.into())
    }
}

impl From<f64> for LazyValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

pub async fn array_partition_idx(
    a: Vec<LazyValue>,
    idx: usize,
    take_lower: bool,
) -> Result<Vec<LazyValue>, String> {
    // we filter NaNs, so this is reasonable
    #[derive(PartialEq, Clone)]
    enum IntOrFloatOrd {
        Int(Integer),
        Float(f64),
    }
    impl PartialOrd for IntOrFloatOrd {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Eq for IntOrFloatOrd {}
    impl Ord for IntOrFloatOrd {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            match (self, other) {
                (IntOrFloatOrd::Int(a), IntOrFloatOrd::Int(b)) => a.cmp(b),
                (IntOrFloatOrd::Int(a), IntOrFloatOrd::Float(b)) => a.partial_cmp(b).unwrap(),
                (IntOrFloatOrd::Float(a), IntOrFloatOrd::Int(b)) => a.partial_cmp(b).unwrap(),
                (IntOrFloatOrd::Float(a), IntOrFloatOrd::Float(b)) => a.partial_cmp(b).unwrap(),
            }
        }
    }
    let mut raw = Vec::new();
    raw.reserve_exact(a.len());
    for v in a.iter() {
        raw.push(match v.clone().resolve().await {
            RVal::Int(i) => IntOrFloatOrd::Int(i),
            RVal::Float(f) => {
                if f.is_nan() {
                    continue;
                } else {
                    IntOrFloatOrd::Float(f)
                }
            }
            RVal::Array(_) => {
                return Err("cannot perform keep-highest operation on nested array".into())
            }
        });
    }
    let mut idxs: Vec<usize> = (0..raw.len()).collect();
    let low_idx = idxs.select_nth_unstable_by_key(idx, |i| &raw[*i]).1;
    let low_elem = &raw[*low_idx].clone();
    Ok(raw
        .into_iter()
        .filter(|a| {
            if take_lower {
                a < low_elem
            } else {
                a >= low_elem
            }
        })
        .map(|y| match y {
            IntOrFloatOrd::Int(i) => LazyValue::Int(i),
            IntOrFloatOrd::Float(f) => LazyValue::Float(f),
        })
        .collect())
}


// lmfao
pub trait AsyncTryInto<T> {
    type Error;

    async fn async_try_into(self) -> Result<T, Self::Error>;
}

impl AsyncTryInto<i32> for RVal {
    type Error = String;

    async fn async_try_into(self) -> Result<i32, Self::Error> {
        self.into_i32()
    }
}

impl AsyncTryInto<u32> for RVal {
    type Error = String;

    async fn async_try_into(self) -> Result<u32, Self::Error> {
        self.into_i32()
            .and_then(|v| v.try_into().map_err(|_| format!("value {v} is negative")))
    }
}
