mod eval;
mod lex;
mod parse;
mod value;

#[cfg(not(test))]
fn get_rng() -> impl rand::Rng + Send {
    use rand::{rngs::StdRng, SeedableRng};
    StdRng::from_entropy()
}

#[cfg(test)]
pub fn get_rng() -> impl rand::Rng + Send {
    // colon three
    use rand::{rngs::StdRng, SeedableRng};
    StdRng::seed_from_u64(0x909090)
}

pub fn get_op_string_list() -> String {
    lex::Op::list_of_ops()
        .iter()
        .fold(String::new(), |mut all, op| {
            if !all.is_empty() {
                all.push_str(", ");
            }
            all.push('`');
            all.push_str(op.as_str());
            all.push('`');
            all
        })
}

pub fn vec_into<A, B: From<A>>(v: Vec<A>) -> Vec<B> {
    v.into_iter().map(|x| x.into()).collect()
}

pub async fn vec_async_map<A, B, F, Fut>(v: Vec<A>, mut f: F) -> Vec<B> 
    where F: FnMut(A) -> Fut,
          Fut: Future<Output = B>,
{
    let mut res = Vec::new();
    res.reserve_exact(v.len());
    for x in v {
        res.push(f(x).await);
    }
    res
}

#[test]
fn op_string_fits_in_field() {
    assert!(get_op_string_list().len() <= 1024);
}

use std::future::Future;

pub use eval::eval;
