//Generates a synthetic blockchain-node log for testing.
//Usage: cargo run --release --example gen_log -- <lines> > node.log

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(100_000);
    let mut seed: u64 = 0x2545F4914F6CDD1D; // tiny deterministic PRNG, no dependencies
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };

    let comps = ["consensus", "mempool", "network", "wallet", "rpc"];
    let infos = ["block produced", "tx confirmed", "peer connected", "state synced"];
    let warns = ["slow peer response", "mempool nearly full"];
    let errors = [
        "tx failed: insufficient funds",
        "tx failed: invalid signature",
        "peer disconnected unexpectedly",
        "block rejected: bad proof of work",
    ];

    for i in 0..n {
        let r = next() % 100;
        let (level, msg) = match r {
            0..=69 => ("INFO", infos[(next() % 4) as usize]),
            70..=84 => ("DEBUG", "heartbeat"),
            85..=93 => ("WARN", warns[(next() % 2) as usize]),
            _ => ("ERROR", errors[(next() % 4) as usize]),
        };
        let comp = comps[(next() % 5) as usize];
        let secs = i % 86_400;
        println!("2026-10-01T{:02}:{:02}:{:02}Z {level} {comp} {msg}", secs / 3600, (secs / 60) % 60, secs % 60);
    }
    if n > 10 {
        println!("this line is malformed");
    }
}
