use smol::future::FutureExt;

fn main() {
    afl::fuzz(true, |data| {
        if let Ok(expr) = std::str::from_utf8(data) {
            _ = smol::block_on(
                async { Ok(epimetheus::dice::eval(expr).await) }
                    .or(async { Err(smol::Timer::after(std::time::Duration::from_millis(50))) }),
            );
        }
    });
}
