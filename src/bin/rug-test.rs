use rug::{Complete, Integer};

fn main() {
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).unwrap();
    s.pop();
    let x = s.parse::<u32>().unwrap();
    println!("{}", Integer::factorial(x).complete());
}
