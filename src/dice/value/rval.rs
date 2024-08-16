use super::LazyValue;
use rug::Integer;

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
