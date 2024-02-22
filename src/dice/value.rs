use async_recursion::async_recursion;
use az::Az;
use rug::Integer;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(Integer),
    Float(f64),
    Array(Vec<Value>),
    LazyDice {
        num: u32,
        sides: u32,
        lowest_idx: u32,
        highest_idx: u32,
        explode: Vec<u32>,
    },
}

macro_rules! dimensional_broadcast {
    ($a:ident, $f:expr, $b:ident) => {
        async {
            for (xr, y) in $a.iter_mut().zip($b.into_iter().cycle()) {
                let x = std::mem::replace(xr, Value::Float(0.));
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
                let x = std::mem::replace(xr, Value::Float(0.));
                let x = $f(x, $b.clone().unresolve()).await;
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
                let x = std::mem::replace(xr, Value::Float(0.));
                let x = $f($a.clone().unresolve(), x).await;
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
    Array(Vec<Value>),
}

impl RVal {
    pub fn truthy(&self) -> bool {
        match self {
            RVal::Int(v) => v.cmp0() != std::cmp::Ordering::Equal,
            RVal::Float(f) => *f != 0.,
            RVal::Array(a) => a.len() > 0,
        }
    }
    pub fn unresolve(self) -> Value {
        match self {
            RVal::Int(n) => Value::Int(n),
            RVal::Float(f) => Value::Float(f),
            RVal::Array(a) => Value::Array(a),
        }
    }
    fn norm_float(f: f64) -> f64 {
        (f * 1_000_000.) / (1_000_000.)
    }
    pub fn to_i32(self) -> Result<i32, String> {
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
            RVal::Array(_) => Err(format!("cannot cast array to integer")),
        }
    }
    async fn op_eq(self, other: RVal) -> RVal {
        match (self, other) {
            (RVal::Array(mut a), RVal::Array(b)) => {
                dimensional_broadcast!(a, Value::op_eq, b).await
            }
            (RVal::Array(mut a), v) => broadcast!(#a, Value::op_eq, v).await,
            (v, RVal::Array(mut a)) => broadcast!(v, Value::op_eq, #a).await,
            (RVal::Int(n), RVal::Int(m)) => RVal::Int(if n == m { 1 } else { 0 }.into()),
            (RVal::Int(n), RVal::Float(f)) | (RVal::Float(f), RVal::Int(n)) => {
                let f = RVal::norm_float(f);
                RVal::Int(if n == f { 1 } else { 0 }.into())
            }
            (RVal::Float(a), RVal::Float(b)) => RVal::Int(
                if RVal::norm_float(a) == RVal::norm_float(b) {
                    1
                } else {
                    0
                }
                .into(),
            ),
        }
    }
    async fn op_or(self, other: RVal) -> RVal {
        match (self, other) {
            (RVal::Array(mut a), RVal::Array(b)) => {
                dimensional_broadcast!(a, Value::op_or, b).await
            }
            (RVal::Array(mut a), v) => broadcast!(#a, Value::op_or, v).await,
            (v, RVal::Array(mut a)) => broadcast!(v, Value::op_or, #a).await,
            (v, w) => {
                if v.truthy() {
                    v
                } else {
                    w
                }
            }
        }
    }

    #[async_recursion]
    async fn add(self, rhs: RVal) -> RVal {
        match (self, rhs) {
            (RVal::Int(n), RVal::Int(m)) => RVal::Int(n + m),
            (RVal::Int(n), RVal::Float(f)) | (RVal::Float(f), RVal::Int(n)) => {
                RVal::Float(f + n.az::<f64>())
            }
            (RVal::Float(a), RVal::Float(b)) => RVal::Float(a + b),
            (RVal::Array(mut a), RVal::Array(b)) => dimensional_broadcast!(a, Value::add, b).await,
            (RVal::Array(mut a), v) => broadcast!(#a, Value::add, v).await,
            (v, RVal::Array(mut a)) => broadcast!(v, Value::add, #a).await,
        }
    }

    #[async_recursion]
    async fn sub(self, rhs: RVal) -> RVal {
        match (self, rhs) {
            (RVal::Int(n), RVal::Int(m)) => RVal::Int(n - m),
            (RVal::Int(n), RVal::Float(f)) | (RVal::Float(f), RVal::Int(n)) => {
                RVal::Float(f - n.az::<f64>())
            }
            (RVal::Float(a), RVal::Float(b)) => RVal::Float(a - b),
            (RVal::Array(mut a), RVal::Array(b)) => dimensional_broadcast!(a, Value::sub, b).await,
            (RVal::Array(mut a), v) => broadcast!(#a, Value::sub, v).await,
            (v, RVal::Array(mut a)) => broadcast!(v, Value::sub, #a).await,
        }
    }

    #[async_recursion]
    async fn mul(self, rhs: RVal) -> RVal {
        match (self, rhs) {
            (RVal::Int(n), RVal::Int(m)) => RVal::Int(n * m),
            (RVal::Int(n), RVal::Float(f)) | (RVal::Float(f), RVal::Int(n)) => {
                RVal::Float(f * n.az::<f64>())
            }
            (RVal::Float(a), RVal::Float(b)) => RVal::Float(a * b),
            (RVal::Array(mut a), RVal::Array(b)) => dimensional_broadcast!(a, Value::mul, b).await,
            (RVal::Array(mut a), v) => broadcast!(#a, Value::mul, v).await,
            (v, RVal::Array(mut a)) => broadcast!(v, Value::mul, #a).await,
        }
    }

    #[async_recursion]
    async fn div(self, rhs: RVal) -> RVal {
        match (self, rhs) {
            (RVal::Int(n), RVal::Int(m)) => RVal::Float(n.az::<f64>() / m.az::<f64>()),
            (RVal::Int(n), RVal::Float(f)) => RVal::Float(n.az::<f64>() / f),
            (RVal::Float(f), RVal::Int(n)) => RVal::Float(f / n.az::<f64>()),
            (RVal::Float(a), RVal::Float(b)) => RVal::Float(a / b),
            (RVal::Array(mut a), RVal::Array(b)) => dimensional_broadcast!(a, Value::div, b).await,
            (RVal::Array(mut a), v) => broadcast!(#a, Value::div, v).await,
            (v, RVal::Array(mut a)) => broadcast!(v, Value::div, #a).await,
        }
    }

    async fn neg(self) -> RVal {
        match self {
            RVal::Int(n) => RVal::Int(-n),
            RVal::Float(f) => RVal::Float(-f),
            RVal::Array(mut a) => {
                // broadcast
                for xr in a.iter_mut() {
                    let x = std::mem::replace(xr, Value::Float(0.));
                    let x = x.neg().await;
                    let _ = std::mem::replace(xr, x);
                }
                RVal::Array(a)
            }
        }
    }
}

impl Value {
    pub async fn add(self, rhs: Value) -> Value {
        self.resolve()
            .await
            .add(rhs.resolve().await)
            .await
            .unresolve()
    }

    pub async fn sub(self, rhs: Value) -> Value {
        self.resolve()
            .await
            .sub(rhs.resolve().await)
            .await
            .unresolve()
    }

    pub async fn mul(self, rhs: Value) -> Value {
        self.resolve()
            .await
            .mul(rhs.resolve().await)
            .await
            .unresolve()
    }

    pub async fn div(self, rhs: Value) -> Value {
        self.resolve()
            .await
            .div(rhs.resolve().await)
            .await
            .unresolve()
    }

    #[async_recursion]
    pub async fn neg(self) -> Value {
        self.resolve().await.neg().await.unresolve()
    }
}

async fn resolve_dice(
    num: u32,
    sides: u32,
    lowest_idx: u32,
    highest_idx: u32,
    explode: Vec<u32>,
) -> u32 {
    if highest_idx < lowest_idx {
        return 0;
    }
    use rand::distributions::{Distribution, Uniform};
    let mut rng = crate::dice::get_rng();
    let between = Uniform::from(1..sides + 1);
    async fn do_explode(
        rng: &mut (impl rand::Rng + Send),
        between: &Uniform<u32>,
        explode_nums: &[u32],
    ) -> u32 {
        let mut sum = 0;
        /* do-while loop, cough cough... */
        while {
            let x = between.sample(rng);
            sum += x;
            explode_nums.contains(&x)
        } {
            // just in case.
            crate::util::yield_point().await;
        }
        sum
    }
    if lowest_idx == 0 && highest_idx == sides - 1 {
        let mut sum = 0;
        for _ in 0..num {
            let sample = if explode.len() > 0 {
                do_explode(&mut rng, &between, &explode).await
            } else {
                between.sample(&mut rng)
            };
            sum += sample;
        }
        sum
    } else {
        let mut res = Vec::new();
        res.reserve_exact(num as usize);
        for _ in 0..num {
            let sample = if explode.len() > 0 {
                do_explode(&mut rng, &between, &explode).await
            } else {
                between.sample(&mut rng)
            };
            res.push(sample);
        }
        res.sort_unstable();
        res[lowest_idx as usize..=highest_idx as usize]
            .into_iter()
            .sum()
    }
}

impl Value {
    pub async fn resolve(self) -> RVal {
        match self {
            Value::Int(n) => RVal::Int(n),
            Value::Float(f) => RVal::Float(f),
            Value::Array(a) => RVal::Array(a),
            Value::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx,
                explode,
            } => RVal::Int(
                resolve_dice(num, sides, lowest_idx, highest_idx, explode)
                    .await
                    .into(),
            ),
        }
    }

    pub async fn to_i32(self) -> Result<i32, String> {
        self.resolve().await.to_i32()
    }

    #[async_recursion]
    pub async fn op_eq(self, other: Value) -> Value {
        self.resolve()
            .await
            .op_eq(other.resolve().await)
            .await
            .unresolve()
    }

    #[async_recursion]
    pub async fn op_or(self, other: Value) -> Value {
        self.resolve()
            .await
            .op_or(other.resolve().await)
            .await
            .unresolve()
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Int(value.into())
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

pub async fn array_partition_idx(
    a: Vec<Value>,
    idx: usize,
    take_lower: bool,
) -> Result<Vec<Value>, String> {
    // we filter NaNs, so this is reasonable
    #[derive(PartialEq, Clone)]
    enum IntOrFloatOrd {
        Int(Integer),
        Float(f64),
    }
    impl PartialOrd for IntOrFloatOrd {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(match (self, other) {
                (IntOrFloatOrd::Int(a), IntOrFloatOrd::Int(b)) => a.cmp(b),
                (IntOrFloatOrd::Int(a), IntOrFloatOrd::Float(b)) => a.partial_cmp(b)?,
                (IntOrFloatOrd::Float(a), IntOrFloatOrd::Int(b)) => a.partial_cmp(b)?,
                (IntOrFloatOrd::Float(a), IntOrFloatOrd::Float(b)) => a.partial_cmp(b)?,
            })
        }
    }
    impl Eq for IntOrFloatOrd {}
    impl Ord for IntOrFloatOrd {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.partial_cmp(other).unwrap()
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
        })
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
            IntOrFloatOrd::Int(i) => Value::Int(i),
            IntOrFloatOrd::Float(f) => Value::Float(f),
        })
        .collect())
}

impl<T> From<Vec<T>> for Value
where
    T: Into<Value>,
{
    fn from(value: Vec<T>) -> Self {
        Value::Array(value.into_iter().map(|x| x.into()).collect())
    }
}

// lmfao
pub trait AsyncTryInto<T> {
    type Error;

    async fn async_try_into(self) -> Result<T, Self::Error>;
}

impl AsyncTryInto<i32> for RVal {
    type Error = String;

    async fn async_try_into(self) -> Result<i32, Self::Error> {
        self.to_i32()
    }
}

impl AsyncTryInto<u32> for RVal {
    type Error = String;

    async fn async_try_into(self) -> Result<u32, Self::Error> {
        self.to_i32()
            .and_then(|v| v.try_into().map_err(|_| format!("value {v} is negative")))
    }
}

impl<T> AsyncTryInto<Vec<T>> for RVal
where
    RVal: AsyncTryInto<T>,
    String: From<<RVal as AsyncTryInto<T>>::Error>,
{
    type Error = String;

    async fn async_try_into(self) -> Result<Vec<T>, Self::Error> {
        match self {
            RVal::Int(_) => Err(format!("integer cannot be casted to array")),
            RVal::Float(_) => Err(format!("integer cannot be casted to array")),
            RVal::Array(a) => {
                let mut res = Vec::new();
                res.reserve_exact(a.len());
                for x in a {
                    res.push(
                        <RVal as AsyncTryInto<T>>::async_try_into(x.resolve().await)
                            .await
                            .map_err(|e| String::from(e))?,
                    );
                }
                Ok(res)
            }
        }
    }
}

impl<T> Into<RVal> for Vec<T>
where
    T: Into<RVal>,
{
    fn into(self) -> RVal {
        RVal::Array(self.into_iter().map(|x| x.into().unresolve()).collect())
    }
}

impl Into<RVal> for i32 {
    fn into(self) -> RVal {
        RVal::Int(self.into())
    }
}

impl Into<RVal> for u32 {
    fn into(self) -> RVal {
        RVal::Int(self.into())
    }
}

impl RVal {
    #[async_recursion]
    pub async fn display(&self, s: &mut String) {
        match self {
            RVal::Int(n) => *s += &format!("{n}"),
            RVal::Float(a) => *s += &format!("{a}"),
            RVal::Array(arr) => {
                s.push('(');
                let mut i = arr.iter();
                if let Some(v) = i.next() {
                    v.display(s).await;
                    for el in i {
                        s.push(',');
                        s.push(' ');
                        el.display(s).await;
                    }
                }
                s.push(')');
            }
        }
    }
}

impl Value {
    #[async_recursion]
    async fn display(&self, s: &mut String) {
        self.clone().resolve().await.display(s).await
    }
}
