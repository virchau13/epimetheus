use std::io::{self, Write};

use epimetheus::dice;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    eprint!("> ");
    io::stderr().flush().unwrap();
    for line in std::io::stdin().lines() {
        let Ok(line) = line else { break };
        match dice::eval(&line).await {
            Ok(val) => println!("{val}"),
            Err(e) => eprintln!("error: {e}"),
        }
        eprint!("> ");
        io::stderr().flush().unwrap();
    }
}
