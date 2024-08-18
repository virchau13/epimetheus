use std::io::{self, Write};

use epimetheus::dice::{
    self,
    value::{cmp_rrvals, RRVal},
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    eprint!("> ");
    io::stderr().flush().unwrap();
    for line in std::io::stdin().lines() {
        let Ok(line) = line else { break };
        if line.starts_with("sort ") {
            let rest = line.strip_prefix("sort ").unwrap();
            match dice::eval(rest).await {
                Ok(val) => {
                    if let RRVal::Array(mut arr) = val {
                        arr.sort_unstable_by(cmp_rrvals);
                        println!("{}", RRVal::Array(arr));
                    } else {
                        eprintln!("error: sort given not-array");
                    }
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    break;
                }
            };
        } else {
            match dice::eval(&line).await {
                Ok(val) => println!("{val}"),
                Err(e) => eprintln!("error: {e}"),
            }
        }
        eprint!("> ");
        io::stderr().flush().unwrap();
    }
}
