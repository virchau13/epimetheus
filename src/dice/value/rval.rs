use super::{escape_string_for_discord_inplace, LazyValue};
use async_recursion::async_recursion;
use rug::Integer;

macro_rules! dimensional_broadcast {
    ($a:ident, $f:expr, $b:ident) => {
        async {
            for (xr, y) in $a.iter_mut().zip($b.into_iter().cycle()) {
                let x = std::mem::replace(xr, LazyValue::Float(0.));
                let x = $f(x, y).await;
                let _ = std::mem::replace(xr, x);
            }
            /* you never know... */
            crate::util::yield_point().await;
            RVal::Array($a)
        }
    };
}

macro_rules! broadcast {
    (#$arr:ident, $f:expr, $b:ident) => {
        async {
            for xr in $arr.iter_mut() {
                let x = std::mem::replace(xr, LazyValue::Float(0.));
                let x = $f(x, $b.clone().into()).await;
                let _ = std::mem::replace(xr, x);
            }
            /* you never know... */
            crate::util::yield_point().await;
            RVal::Array($arr)
        }
    };
    ($a:ident, $f:expr, #$arr:ident) => {
        async {
            for xr in $arr.iter_mut() {
                let x = std::mem::replace(xr, LazyValue::Float(0.));
                let x = $f($a.clone().into(), x).await;
                let _ = std::mem::replace(xr, x);
            }
            /* you never know... */
            crate::util::yield_point().await;
            RVal::Array($arr)
        }
    };
}

/// Resolved version of [`Value`].
#[derive(Debug, Clone, PartialEq)]
pub enum RVal {
    Int(Integer),
    Float(f64),
    Array(Vec<LazyValue>),
    Char(char),
}

impl RVal {
    fn norm_float(f: f64) -> f64 {
        (f * 1_000_000.) / (1_000_000.)
    }

    pub fn truthy(&self) -> bool {
        match self {
            RVal::Int(v) => v.cmp0() != std::cmp::Ordering::Equal,
            RVal::Float(f) => *f != 0.,
            RVal::Array(a) => !a.is_empty(),
            RVal::Char(c) => *c != '\0',
        }
    }

    pub fn into_i32(self) -> Result<i32, String> {
        match self {
            RVal::Int(v) => v
                .to_i32()
                .ok_or(format!("Integer {v} too large for 32-bit integer")),
            RVal::Float(f) => {
                let im = RVal::norm_float(f);
                if im.trunc() == im {
                    Ok(im.trunc() as i32)
                } else {
                    Err(format!("{im} is not an integer value"))
                }
            }
            RVal::Array(_) => Err("cannot cast array to integer".to_string()),
            RVal::Char(c) => Ok(c as i32),
        }
    }
    pub async fn op_eq(self, other: RVal) -> RVal {
        match (self, other) {
            (RVal::Array(mut a), RVal::Array(b)) => {
                dimensional_broadcast!(a, LazyValue::op_eq, b).await
            }
            (RVal::Array(mut a), v) => broadcast!(#a, LazyValue::op_eq, v).await,
            (v, RVal::Array(mut a)) => broadcast!(v, LazyValue::op_eq, #a).await,
            (RVal::Int(n), RVal::Int(m)) => RVal::Int((n == m).into()),
            (RVal::Char(c), RVal::Char(d)) => RVal::Int((c == d).into()),
            (RVal::Char(c), RVal::Int(n)) | (RVal::Int(n), RVal::Char(c)) => {
                RVal::Int((n == (c as u32)).into())
            }
            (RVal::Char(c), RVal::Float(f)) | (RVal::Float(f), RVal::Char(c)) => {
                let f = RVal::norm_float(f);
                RVal::Int((f == (c as u32 as f64)).into())
            }
            (RVal::Int(n), RVal::Float(f)) | (RVal::Float(f), RVal::Int(n)) => {
                let f = RVal::norm_float(f);
                RVal::Int((n == f).into())
            }
            (RVal::Float(a), RVal::Float(b)) => {
                RVal::Int((RVal::norm_float(a) == RVal::norm_float(b)).into())
            }
        }
    }
    pub async fn op_or(self, other: RVal) -> RVal {
        match (self, other) {
            (RVal::Array(mut a), RVal::Array(b)) => {
                dimensional_broadcast!(a, LazyValue::op_or, b).await
            }
            (RVal::Array(mut a), v) => broadcast!(#a, LazyValue::op_or, v).await,
            (v, RVal::Array(mut a)) => broadcast!(v, LazyValue::op_or, #a).await,
            (v, w) => {
                if v.truthy() {
                    v
                } else {
                    w
                }
            }
        }
    }

    pub async fn neg(self) -> RVal {
        match self {
            RVal::Int(n) => RVal::Int(-n),
            RVal::Float(f) => RVal::Float(-f),
            RVal::Array(mut a) => {
                // broadcast
                for xr in a.iter_mut() {
                    let x = std::mem::replace(xr, LazyValue::Float(0.));
                    let x = x.neg().await;
                    let _ = std::mem::replace(xr, x);
                }
                RVal::Array(a)
            },
            RVal::Char(c) => RVal::Int((-(c as i32)).into()),
        }
    }

    #[async_recursion]
    pub async fn display(&self, s: &mut String) {
        match self {
            RVal::Int(n) => *s += &format!("{n}"),
            RVal::Float(a) => *s += &format!("{a}"),
            RVal::Array(arr) => {
                if arr.is_empty() {
                    // empty array
                    *s += "[]";
                } else if arr.iter().all(|x| matches!(x, LazyValue::Char(_))) {
                    // string
                    let mut tmp = String::new();
                    tmp.reserve(arr.len());
                    for x in arr.iter() {
                        let LazyValue::Char(cr) = x else { unreachable!("we checked these were all-characters already") };
                        tmp.push(*cr);
                    }
                    escape_string_for_discord_inplace(&tmp, s);
                } else {
                    // general array
                    s.push('[');
                    let mut i = arr.iter();
                    if let Some(v) = i.next() {
                        v.display(s).await;
                        for el in i {
                            s.push(',');
                            s.push(' ');
                            el.display(s).await;
                        }
                    }
                    s.push(']');
                }
            },
            RVal::Char(c) => *s += &format!("'{c}'"),
        }
    }
}

impl<T> From<Vec<T>> for RVal
where
    T: Into<RVal>,
{
    fn from(val: Vec<T>) -> Self {
        RVal::Array(val.into_iter().map(|x| x.into().into()).collect())
    }
}

impl From<i32> for RVal {
    fn from(val: i32) -> Self {
        RVal::Int(val.into())
    }
}

impl From<u32> for RVal {
    fn from(val: u32) -> Self {
        RVal::Int(val.into())
    }
}

impl From<&str> for RVal {
    fn from(value: &str) -> Self {
        RVal::Array(value.chars().map(LazyValue::Char).collect())
    }
}

impl From<RVal> for LazyValue {
    fn from(val: RVal) -> Self {
        match val {
            RVal::Int(n) => LazyValue::Int(n),
            RVal::Float(f) => LazyValue::Float(f),
            RVal::Array(a) => LazyValue::Array(a),
            RVal::Char(c) => LazyValue::Char(c),
        }
    }
}
