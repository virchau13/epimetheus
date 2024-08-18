use std::cmp::Ordering;

use async_recursion::async_recursion;
use az::Az;
use rug::Integer;

use crate::dice::{eval::Evaluator, value::norm_float};

use super::{escape_string_for_discord, LazyValue, RVal, ResolveError};

macro_rules! dimensional_broadcast {
    ($a:ident, $f:expr, $b:ident) => {
        async {
            for (xr, y) in $a.iter_mut().zip($b.into_iter().cycle()) {
                let x = std::mem::replace(xr, RRVal::Float(0.));
                let x = $f(x, y).await;
                let _ = std::mem::replace(xr, x);
            }
            /* you never know... */
            crate::util::yield_point().await;
            RRVal::Array($a)
        }
    };
}

macro_rules! broadcast {
    (#$arr:ident, $f:expr, $b:ident) => {
        async {
            for xr in $arr.iter_mut() {
                let x = std::mem::replace(xr, RRVal::Float(0.));
                let x = $f(x, $b.clone().into()).await;
                let _ = std::mem::replace(xr, x);
            }
            /* you never know... */
            crate::util::yield_point().await;
            RRVal::Array($arr)
        }
    };
    ($a:ident, $f:expr, #$arr:ident) => {
        async {
            for xr in $arr.iter_mut() {
                let x = std::mem::replace(xr, RRVal::Float(0.));
                let x = $f($a.clone().into(), x).await;
                let _ = std::mem::replace(xr, x);
            }
            /* you never know... */
            crate::util::yield_point().await;
            RRVal::Array($arr)
        }
    };
}

/// Deep-resolved version of [`Value`].
#[derive(Debug, Clone)]
pub enum RRVal {
    Int(Integer),
    Float(f64),
    Array(Vec<RRVal>),
    Char(char),
}

impl From<RRVal> for RVal {
    fn from(val: RRVal) -> Self {
        match val {
            RRVal::Int(n) => RVal::Int(n),
            RRVal::Float(f) => RVal::Float(f),
            RRVal::Char(c) => RVal::Char(c),
            RRVal::Array(a) => RVal::Array(a.into_iter().map(|x| x.into()).collect()),
        }
    }
}

impl From<RRVal> for LazyValue {
    fn from(val: RRVal) -> Self {
        match val {
            RRVal::Int(n) => LazyValue::Int(n),
            RRVal::Float(f) => LazyValue::Float(f),
            RRVal::Char(c) => LazyValue::Char(c),
            RRVal::Array(a) => LazyValue::Array(a.into_iter().map(|x| x.into()).collect()),
        }
    }
}

impl RRVal {
    #[async_recursion]
    pub async fn deep_resolve_vec(
        a: Vec<LazyValue>,
        eval: &Evaluator,
    ) -> Result<Vec<RRVal>, ResolveError> {
        let mut v = Vec::new();
        v.reserve_exact(a.len());
        for x in a {
            v.push(x.deep_resolve(eval).await?);
        }
        Ok(v)
    }

    #[async_recursion]
    pub async fn add(self, rhs: RRVal) -> RRVal {
        match (self, rhs) {
            (RRVal::Int(n), RRVal::Int(m)) => RRVal::Int(n + m),
            (RRVal::Int(n), RRVal::Float(f)) | (RRVal::Float(f), RRVal::Int(n)) => {
                RRVal::Float(f + n.az::<f64>())
            }
            (RRVal::Float(a), RRVal::Float(b)) => RRVal::Float(a + b),
            (RRVal::Char(c), RRVal::Char(d)) => RRVal::Char(
                (c as u32)
                    .checked_add(d as u32)
                    .and_then(char::from_u32)
                    .unwrap_or(' '),
            ),
            (RRVal::Char(c), RRVal::Int(n)) | (RRVal::Int(n), RRVal::Char(c)) => {
                RRVal::Int(n + c as u32)
            }
            (RRVal::Float(f), RRVal::Char(c)) | (RRVal::Char(c), RRVal::Float(f)) => {
                RRVal::Float(f + (c as u32 as f64))
            }
            (RRVal::Array(mut a), RRVal::Array(b)) => {
                dimensional_broadcast!(a, RRVal::add, b).await
            }
            (RRVal::Array(mut a), v) => broadcast!(#a, RRVal::add, v).await,
            (v, RRVal::Array(mut a)) => broadcast!(v, RRVal::add, #a).await,
        }
    }

    #[async_recursion]
    pub async fn sub(self, rhs: RRVal) -> RRVal {
        match (self, rhs) {
            (RRVal::Int(n), RRVal::Int(m)) => RRVal::Int(n - m),
            (RRVal::Int(n), RRVal::Float(f)) => RRVal::Float(n.az::<f64>() - f),
            (RRVal::Float(f), RRVal::Int(n)) => RRVal::Float(f - n.az::<f64>()),
            (RRVal::Float(a), RRVal::Float(b)) => RRVal::Float(a - b),
            (RRVal::Char(c), RRVal::Char(d)) => RRVal::Char(
                (c as u32)
                    .checked_sub(d as u32)
                    .and_then(char::from_u32)
                    .unwrap_or(' '),
            ),
            (RRVal::Char(c), RRVal::Int(n)) => RRVal::Int((c as u32) - n),
            (RRVal::Int(n), RRVal::Char(c)) => RRVal::Int(n - (c as u32)),
            (RRVal::Float(f), RRVal::Char(c)) => RRVal::Float(f - (c as u32 as f64)),
            (RRVal::Char(c), RRVal::Float(f)) => RRVal::Float((c as u32 as f64) - f),
            (RRVal::Array(mut a), RRVal::Array(b)) => {
                dimensional_broadcast!(a, RRVal::sub, b).await
            }
            (RRVal::Array(mut a), v) => broadcast!(#a, RRVal::sub, v).await,
            (v, RRVal::Array(mut a)) => broadcast!(v, RRVal::sub, #a).await,
        }
    }

    #[async_recursion]
    pub async fn mul(self, rhs: RRVal) -> RRVal {
        match (self, rhs) {
            (RRVal::Int(n), RRVal::Int(m)) => RRVal::Int(n * m),
            (RRVal::Int(n), RRVal::Float(f)) | (RRVal::Float(f), RRVal::Int(n)) => {
                RRVal::Float(f * n.az::<f64>())
            }
            (RRVal::Float(a), RRVal::Float(b)) => RRVal::Float(a * b),
            (RRVal::Char(c), RRVal::Char(d)) => {
                RRVal::Char(char::from_u32(c as u32 * d as u32).unwrap_or(' '))
            }
            (RRVal::Char(c), RRVal::Int(n)) | (RRVal::Int(n), RRVal::Char(c)) => {
                RRVal::Int(n * (c as u32))
            }
            (RRVal::Float(f), RRVal::Char(c)) | (RRVal::Char(c), RRVal::Float(f)) => {
                RRVal::Float(f * (c as u32 as f64))
            }
            (RRVal::Array(mut a), RRVal::Array(b)) => {
                dimensional_broadcast!(a, RRVal::mul, b).await
            }
            (RRVal::Array(mut a), v) => broadcast!(#a, RRVal::mul, v).await,
            (v, RRVal::Array(mut a)) => broadcast!(v, RRVal::mul, #a).await,
        }
    }

    #[async_recursion]
    pub async fn fdiv(self, rhs: RRVal) -> RRVal {
        match (self, rhs) {
            (RRVal::Int(n), RRVal::Int(m)) => RRVal::Float(n.az::<f64>() / m.az::<f64>()),
            (RRVal::Int(n), RRVal::Float(f)) => RRVal::Float(n.az::<f64>() / f),
            (RRVal::Float(f), RRVal::Int(n)) => RRVal::Float(f / n.az::<f64>()),
            (RRVal::Float(a), RRVal::Float(b)) => RRVal::Float(a / b),
            (RRVal::Char(c), RRVal::Char(d)) => RRVal::Float(c as u32 as f64 / d as u32 as f64),
            (RRVal::Char(c), RRVal::Int(n)) => RRVal::Float(c as u32 as f64 / n.az::<f64>()),
            (RRVal::Int(n), RRVal::Char(c)) => RRVal::Float(n.az::<f64>() / c as u32 as f64),
            (RRVal::Float(f), RRVal::Char(c)) => RRVal::Float(f / c as u32 as f64),
            (RRVal::Char(c), RRVal::Float(f)) => RRVal::Float((c as u32 as f64) / f),
            (RRVal::Array(mut a), RRVal::Array(b)) => {
                dimensional_broadcast!(a, RRVal::fdiv, b).await
            }
            (RRVal::Array(mut a), v) => broadcast!(#a, RRVal::fdiv, v).await,
            (v, RRVal::Array(mut a)) => broadcast!(v, RRVal::fdiv, #a).await,
        }
    }

    #[async_recursion]
    pub async fn op_eq(self, other: RRVal) -> RRVal {
        match (self, other) {
            (RRVal::Array(mut a), RRVal::Array(b)) => {
                dimensional_broadcast!(a, RRVal::op_eq, b).await
            }
            (RRVal::Array(mut a), v) => broadcast!(#a, RRVal::op_eq, v).await,
            (v, RRVal::Array(mut a)) => broadcast!(v, RRVal::op_eq, #a).await,
            (l, r) => RRVal::Int((cmp_rrvals(&l, &r) == Ordering::Equal).into()),
        }
    }

    #[async_recursion]
    pub async fn op_lt(self, other: RRVal) -> RRVal {
        match (self, other) {
            (RRVal::Array(mut a), RRVal::Array(b)) => {
                dimensional_broadcast!(a, RRVal::op_lt, b).await
            }
            (RRVal::Array(mut a), v) => broadcast!(#a, RRVal::op_lt, v).await,
            (v, RRVal::Array(mut a)) => broadcast!(v, RRVal::op_lt, #a).await,
            (l, r) => RRVal::Int((cmp_rrvals(&l, &r) == Ordering::Less).into()),
        }
    }

    #[async_recursion]
    pub async fn op_gt(self, other: RRVal) -> RRVal {
        match (self, other) {
            (RRVal::Array(mut a), RRVal::Array(b)) => {
                dimensional_broadcast!(a, RRVal::op_gt, b).await
            }
            (RRVal::Array(mut a), v) => broadcast!(#a, RRVal::op_gt, v).await,
            (v, RRVal::Array(mut a)) => broadcast!(v, RRVal::op_gt, #a).await,
            (l, r) => RRVal::Int((cmp_rrvals(&l, &r) == Ordering::Greater).into()),
        }
    }

    #[async_recursion]
    pub async fn neg(self) -> RRVal {
        match self {
            RRVal::Int(n) => RRVal::Int(-n),
            RRVal::Float(f) => RRVal::Float(-f),
            RRVal::Array(mut a) => {
                // broadcast
                for xr in a.iter_mut() {
                    let x = std::mem::replace(xr, RRVal::Float(0.));
                    let x = x.neg().await;
                    let _ = std::mem::replace(xr, x);
                }
                RRVal::Array(a)
            }
            RRVal::Char(c) => RRVal::Int((-(c as i32)).into()),
        }
    }

    #[async_recursion]
    pub async fn op_or(self, other: RRVal) -> RRVal {
        match (self, other) {
            (RRVal::Array(mut a), RRVal::Array(b)) => {
                dimensional_broadcast!(a, RRVal::op_or, b).await
            }
            (RRVal::Array(mut a), v) => broadcast!(#a, RRVal::op_or, v).await,
            (v, RRVal::Array(mut a)) => broadcast!(v, RRVal::op_or, #a).await,
            (v, w) => {
                if v.truthy() {
                    v
                } else {
                    w
                }
            }
        }
    }

    pub fn truthy(&self) -> bool {
        match self {
            RRVal::Int(v) => v.cmp0() != std::cmp::Ordering::Equal,
            RRVal::Float(f) => *f != 0.,
            RRVal::Array(a) => !a.is_empty(),
            RRVal::Char(c) => *c != '\0',
        }
    }

    pub fn into_i32(self) -> Result<i32, String> {
        RVal::from(self).into_i32()
    }
}

impl std::fmt::Display for RRVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RRVal::Int(n) => write!(f, "{n}"),
            RRVal::Float(a) => write!(f, "{a}"),
            RRVal::Array(arr) => {
                if arr.is_empty() {
                    // empty array
                    write!(f, "[]")
                } else if arr.iter().all(|x| matches!(x, RRVal::Char(_))) {
                    // string
                    let mut tmp = String::new();
                    tmp.reserve(arr.len());
                    for x in arr.iter() {
                        let RRVal::Char(cr) = x else {
                            unreachable!("we checked this were all characters already")
                        };
                        tmp.push(*cr);
                    }
                    f.write_str(&escape_string_for_discord(&tmp))
                } else {
                    // general array
                    write!(f, "[")?;
                    let mut i = arr.iter();
                    if let Some(v) = i.next() {
                        write!(f, "{v}")?;
                        for el in i {
                            write!(f, ", {el}")?;
                        }
                    }
                    write!(f, "]")
                }
            }
            RRVal::Char(c) => write!(f, "'{c}'"),
        }
    }
}

impl From<&str> for RRVal {
    fn from(value: &str) -> Self {
        RRVal::Array(value.chars().map(RRVal::Char).collect())
    }
}

impl From<i32> for RRVal {
    fn from(value: i32) -> Self {
        Self::Int(value.into())
    }
}

impl From<f64> for RRVal {
    fn from(value: f64) -> Self {
        Self::Float(value.into())
    }
}

impl<T> From<Vec<T>> for RRVal
where
    T: Into<RRVal>,
{
    fn from(val: Vec<T>) -> Self {
        RRVal::Array(val.into_iter().map(|x| x.into()).collect())
    }
}

impl PartialEq for RRVal {
    fn eq(&self, other: &Self) -> bool {
        cmp_rrvals(self, other) == Ordering::Equal
    }
}

impl Eq for RRVal {}

impl PartialOrd<RRVal> for RRVal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RRVal {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_rrvals(self, other)
    }
}

pub fn cmp_rrvals(lhs: &RRVal, rhs: &RRVal) -> std::cmp::Ordering {
    use std::cmp::Ordering::*;
    match (lhs, rhs) {
        (RRVal::Int(n), RRVal::Int(m)) => n.cmp(m),
        (RRVal::Int(n), RRVal::Float(f)) => n.partial_cmp(f).unwrap_or(Greater), /* All integers > NaN */
        (RRVal::Float(f), RRVal::Int(n)) => f.partial_cmp(n).unwrap_or(Less), /* NaN < all integers */
        (RRVal::Char(c), RRVal::Char(d)) => c.cmp(d),
        (RRVal::Char(c), RRVal::Int(n)) => (*c as u32).partial_cmp(n).unwrap(),
        (RRVal::Int(n), RRVal::Char(c)) => n.partial_cmp(&(*c as u32)).unwrap(),
        (RRVal::Float(f), RRVal::Char(c)) => f.partial_cmp(&(*c as u32 as f64)).unwrap_or(Less),
        (RRVal::Char(c), RRVal::Float(f)) => (*c as u32 as f64).partial_cmp(f).unwrap_or(Greater),
        (RRVal::Float(a), RRVal::Float(b)) => {
            if a.is_nan() {
                if b.is_nan() {
                    // a, b are both NaN
                    Equal // lmfao
                } else {
                    // a is the only NaN
                    // (NaN) is less than (any)
                    Less
                }
            } else {
                if b.is_nan() {
                    // b is the only NaN
                    // (any) is greater than (NaN)
                    Greater
                } else {
                    let a = norm_float(*a);
                    let b = norm_float(*b);
                    a.partial_cmp(&b).unwrap()
                }
            }
        }
        (RRVal::Array(a), RRVal::Array(b)) => {
            // lexographic comparison
            if a.len() != b.len() {
                a.len().cmp(&b.len())
            } else {
                a.iter()
                    .zip(b)
                    .find_map(|(a, b)| match cmp_rrvals(a, b) {
                        Equal => None,
                        result => Some(result),
                    })
                    .unwrap_or(Equal)
            }
        }
        (v, RRVal::Array(a)) => {
            // compare by first element
            if let Some(first) = a.first() {
                cmp_rrvals(v, first)
            } else {
                // empty arrays are below everything else
                Less
            }
        }
        (RRVal::Array(a), v) => {
            // compare by first element
            if let Some(first) = a.first() {
                cmp_rrvals(first, v)
            } else {
                // empty arrays are below everything else
                Greater
            }
        }
    }
}

#[tokio::test]
async fn display_test() {
    macro_rules! eq {
        ($x:expr, $y:expr) => {{
            let s = format!("{}", RRVal::from($x));
            assert_eq!(s, $y);
        }};
    }

    eq!(vec![2], "[2]");
    eq!(vec![1, 2, 3], "[1, 2, 3]");
    eq!(
        vec![vec![1, 2], vec![3], vec![4, 5]],
        "[[1, 2], [3], [4, 5]]"
    );
    eq!(Vec::<i32>::new(), "[]");
}
