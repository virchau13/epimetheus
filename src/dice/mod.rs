mod eval;
mod lex;
mod parse;
mod value;

#[cfg(not(any(test, fuzzing)))]
fn get_rng() -> impl rand::Rng + Send {
    use rand::{rngs::StdRng, SeedableRng};
    StdRng::from_entropy()
}

#[cfg(any(test, fuzzing))]
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

#[test]
fn op_string_fits_in_field() {
    assert!(get_op_string_list().len() <= 1024);
}

pub use eval::eval;
