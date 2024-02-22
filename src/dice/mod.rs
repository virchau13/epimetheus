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

pub use eval::eval;
