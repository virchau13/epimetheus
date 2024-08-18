mod lazy_value;
use std::error::Error;

pub use lazy_value::LazyValue;

mod rval;
pub use rval::RVal;

mod rrval;
pub use rrval::RRVal;

use rug::Integer;
use smallvec::SmallVec;
use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveErrorType {
    IndexOutOfBounds,
    IndexingIntoInvalidType,
    /// Undefined variable.
    UndefVar,
}

#[derive(Debug, Clone)]
pub struct ResolveError {
    /// Condition: if `ty == IndexOutOfBounds`,
    /// `place` must already have its index list truncated for the *last* element of
    /// `place.indexes` to be the
    /// one out of range.
    place: Place,
    ty: ResolveErrorType,
}

impl ResolveError {
    pub fn index_out_of_bounds(mut place: Place, ii: usize) -> Self {
        place.indexes.truncate(ii + 1);
        Self {
            place,
            ty: ResolveErrorType::IndexOutOfBounds,
        }
    }

    pub fn index_into_invalid_type(mut place: Place, ii: usize) -> Self {
        place.indexes.truncate(ii + 1);
        Self {
            place,
            ty: ResolveErrorType::IndexingIntoInvalidType,
        }
    }

    pub fn undef_var(place: Place) -> Self {
        Self {
            place,
            ty: ResolveErrorType::UndefVar,
        }
    }
}

impl Error for ResolveError {}
impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.ty {
            ResolveErrorType::IndexOutOfBounds => {
                let actual_idx = self.place.indexes.last().unwrap();
                write!(f, "Index `{actual_idx}` out of bounds in {}", &self.place)
            }
            ResolveErrorType::UndefVar => {
                write!(
                    f,
                    "Variable name {} undefined",
                    escape_string_for_discord(&self.place.varname)
                )
            }
            ResolveErrorType::IndexingIntoInvalidType => {
                write!(f, "Attempt to index into non-array type at {}", &self.place)
            }
        }
    }
}

/// A place, aka lvalue, is a reference to a variable (and/or indexes into that variable.)
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub varname: SmolStr,
    pub indexes: SmallVec<[i32; 4]>,
}

impl std::fmt::Display for Place {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "``")?;
        write!(f, "{}", self.varname)?;
        for i in &self.indexes {
            write!(f, "[{i}]")?;
        }
        write!(f, "``")
    }
}

pub fn norm_float(f: f64) -> f64 {
    (f * 1_000_000.) / (1_000_000.)
}

pub async fn resolve_dice(
    num: u32,
    sides: Vec<RRVal>,
    lowest_idx: u32,
    highest_idx: u32,
    explode: Vec<RRVal>,
) -> RRVal {
    if sides.is_empty() || highest_idx < lowest_idx {
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
    if lowest_idx == 0 && highest_idx == num - 1 {
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
        res.sort_unstable();
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

/// If `lower_half` is true, does the equivalent of
///   1. sorting `a`
///   2. marking the `n` lowest elements
///   3. assembling those elements in an array,
///      with their relative order the same as in the original array `a`.
/// If `lower_half` is false, it does the above, but step (2) is replaced with
///   marking the `n` highest elements.
pub fn array_take_most_extreme_n(mut a: Vec<RRVal>, n: usize, lower_half: bool) -> Vec<RRVal> {
    let mut indices: Vec<_> = (0..a.len()).collect();
    indices.sort_unstable_by_key(|i| &a[*i]);
    let which_array_half = if lower_half {
        &indices[..n]
    } else {
        &indices[(a.len() - n)..]
    };
    let new_vals = which_array_half
        .iter()
        .map(|i| std::mem::replace(&mut a[*i], RRVal::Char(' ')))
        .collect();
    new_vals
}

pub fn escape_string_for_discord_inplace(inp: &str, s: &mut String) {
    s.push_str("``"); // outer code formatting to prevent abusing mentions
    s.push('"');
    for c in inp.chars() {
        if c == '`' {
            s.push('`');
            s.push('\u{200b}'); // zero-width space, prevent "``"
        } else {
            for x in c.escape_default() {
                s.push(x);
            }
        }
    }
    s.push('"');
    s.push_str("``"); // outer code formatting to prevent abusing mentions
}

pub fn escape_string_for_discord(inp: &str) -> String {
    let mut s = String::new();
    escape_string_for_discord_inplace(inp, &mut s);
    s
}
