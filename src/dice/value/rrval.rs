use async_recursion::async_recursion;
use az::Az;
use rug::Integer;

use super::{LazyValue, RVal};

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
#[derive(Debug, Clone, PartialEq)]
pub enum RRVal {
    Int(Integer),
    Float(f64),
    Array(Vec<RRVal>),
}

impl From<RRVal> for RVal {
    fn from(val: RRVal) -> Self {
        match val {
            RRVal::Int(n) => RVal::Int(n),
            RRVal::Float(f) => RVal::Float(f),
            RRVal::Array(a) => RVal::Array(a.into_iter().map(|x| x.into()).collect()),
        }
    }
}

impl From<RRVal> for LazyValue {
    fn from(val: RRVal) -> Self {
        match val {
            RRVal::Int(n) => LazyValue::Int(n),
            RRVal::Float(f) => LazyValue::Float(f),
            RRVal::Array(a) => LazyValue::Array(a.into_iter().map(|x| x.into()).collect()),
        }
    }
}

impl RRVal {
    #[async_recursion]
    pub async fn deep_resolve_vec(a: Vec<LazyValue>) -> Vec<RRVal> {
        let mut v = Vec::new();
        v.reserve_exact(a.len());
        for x in a {
            v.push(x.deep_resolve().await);
        }
        v
    }

    #[async_recursion]
    pub async fn add(self, rhs: RRVal) -> RRVal {
        match (self, rhs) {
            (RRVal::Int(n), RRVal::Int(m)) => RRVal::Int(n + m),
            (RRVal::Int(n), RRVal::Float(f)) | (RRVal::Float(f), RRVal::Int(n)) => {
                RRVal::Float(f + n.az::<f64>())
            }
            (RRVal::Float(a), RRVal::Float(b)) => RRVal::Float(a + b),
            (RRVal::Array(mut a), RRVal::Array(b)) => dimensional_broadcast!(a, RRVal::add, b).await,
            (RRVal::Array(mut a), v) => broadcast!(#a, RRVal::add, v).await,
            (v, RRVal::Array(mut a)) => broadcast!(v, RRVal::add, #a).await,
        }
    }

    #[async_recursion]
    pub async fn sub(self, rhs: RRVal) -> RRVal {
        match (self, rhs) {
            (RRVal::Int(n), RRVal::Int(m)) => RRVal::Int(n - m),
            (RRVal::Int(n), RRVal::Float(f)) | (RRVal::Float(f), RRVal::Int(n)) => {
                RRVal::Float(f - n.az::<f64>())
            }
            (RRVal::Float(a), RRVal::Float(b)) => RRVal::Float(a - b),
            (RRVal::Array(mut a), RRVal::Array(b)) => dimensional_broadcast!(a, RRVal::sub, b).await,
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
            (RRVal::Array(mut a), RRVal::Array(b)) => dimensional_broadcast!(a, RRVal::mul, b).await,
            (RRVal::Array(mut a), v) => broadcast!(#a, RRVal::mul, v).await,
            (v, RRVal::Array(mut a)) => broadcast!(v, RRVal::mul, #a).await,
        }
    }

    #[async_recursion]
    pub async fn div(self, rhs: RRVal) -> RRVal {
        match (self, rhs) {
            (RRVal::Int(n), RRVal::Int(m)) => RRVal::Float(n.az::<f64>() / m.az::<f64>()),
            (RRVal::Int(n), RRVal::Float(f)) => RRVal::Float(n.az::<f64>() / f),
            (RRVal::Float(f), RRVal::Int(n)) => RRVal::Float(f / n.az::<f64>()),
            (RRVal::Float(a), RRVal::Float(b)) => RRVal::Float(a / b),
            (RRVal::Array(mut a), RRVal::Array(b)) => dimensional_broadcast!(a, RRVal::div, b).await,
            (RRVal::Array(mut a), v) => broadcast!(#a, RRVal::div, v).await,
            (v, RRVal::Array(mut a)) => broadcast!(v, RRVal::div, #a).await,
        }
    }
}
