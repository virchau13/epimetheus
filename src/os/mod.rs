use tokio::fs;

// Returns (virtual memory, rss).
pub async fn get_mem_usage() -> anyhow::Result<(u64, u64)> {
    let pgsize = page_size::get() as u64;
    let statm_raw = fs::read("/proc/self/statm").await?;
    let statm = std::str::from_utf8(&statm_raw)?;
    let (virt, rest) = statm
        .split_once(' ')
        .ok_or_else(|| anyhow::Error::msg("statm bad format 1"))?;
    let (rss, _rest) = rest
        .split_once(' ')
        .ok_or_else(|| anyhow::Error::msg("statm bad format 2"))?;
    let virt = virt.parse::<u64>()?;
    let rss = rss.parse::<u64>()?;
    Ok((virt * pgsize, rss * pgsize))
}

const BIPREFIXES: [&str; 5] = ["", "Ki", "Mi", "Gi", "Ti"];

pub fn fmt_bibytes(size: u64) -> String {
    if size == 0 {
        return "0B".into();
    }
    let idx = (BIPREFIXES.len() as u64 - 1).min(size.ilog2() as u64 / 10);
    format!("{}{}B", size >> (10 * idx), BIPREFIXES[idx as usize])
}

#[test]
fn test_fmt_bibytes() {
    macro_rules! y {
        ($r:expr,$a:expr) => {
            assert_eq!(fmt_bibytes($r), $a)
        };
    }

    y!(0, "0B");
    y!(123, "123B");
    y!(1024, "1KiB");
    y!(1024 * 1024, "1MiB");
    y!(1024 * 1024 * 576, "576MiB");
    y!(1024 * 1024 * 1024, "1GiB");
    y!(1024 * 1024 * 1024 * 1024, "1TiB");
    y!(1024 * 1024 * 1024 * 1024 * 1024, "1024TiB");
}
