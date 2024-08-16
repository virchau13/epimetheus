use rug::Integer;

use crate::dice::eval::Evaluator;

use super::{resolve_dice, Place, RRVal, RVal, ResolveError};

#[derive(Debug, Clone, PartialEq)]
pub enum LazyValue {
    Int(Integer),
    Float(f64),
    Char(char),
    Array(Vec<LazyValue>),
    /// A place (aka lvalue) is a reference to some variable (and possibly array indexes in to that
    /// variable).
    Place(Place),
    LazyDice {
        num: u32,
        sides: Vec<RRVal>,
        lowest_idx: u32,
        highest_idx: u32,
        explode: Vec<RRVal>,
    },
}

impl LazyValue {
    pub async fn resolve(self, eval: &Evaluator) -> Result<RVal, ResolveError> {
        Ok(match self {
            LazyValue::Int(n) => RVal::Int(n),
            LazyValue::Float(f) => RVal::Float(f),
            LazyValue::Array(a) => RVal::Array(a),
            LazyValue::Char(c) => RVal::Char(c),
            LazyValue::Place(place) => eval.var_get(&place)?.clone().into(),
            LazyValue::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx,
                explode,
            } => resolve_dice(num, sides, lowest_idx, highest_idx, explode).await.into(),
        })
    }

    pub async fn deep_resolve(self, eval: &Evaluator) -> Result<RRVal, ResolveError> {
        Ok(match self {
            LazyValue::Int(n) => RRVal::Int(n),
            LazyValue::Float(f) => RRVal::Float(f),
            LazyValue::Array(a) => RRVal::Array(RRVal::deep_resolve_vec(a, eval).await?),
            LazyValue::Char(c) => RRVal::Char(c),
            LazyValue::Place(place) => eval.var_get(&place)?.clone(),
            LazyValue::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx,
                explode,
            } => resolve_dice(num, sides, lowest_idx, highest_idx, explode).await,
        })
    }
}

impl<T> From<Vec<T>> for LazyValue
where
    T: Into<LazyValue>,
{
    fn from(value: Vec<T>) -> Self {
        LazyValue::Array(value.into_iter().map(|x| x.into()).collect())
    }
}
