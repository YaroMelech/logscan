use std::{env, fs, process, time::Instant};

use logscan::{analyze_parallel, Level};

fn usage() -> ! {
    eprintln!("usage: logscan <logfile> [--threads N]");
    process::exit(2);
}

fn main() {
    let mut args = env::args().skip(1);
    let path = args.next().unwrap_or_else(|| usage());
    let mut threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);

    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--threads" => {
                threads = args.next().and_then(|v| v.parse().ok()).unwrap_or_else(|| usage());
            }
            _ => usage(),
        }
    }

    let text = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {path}: {e}");
        process::exit(1);
    });

    let start = Instant::now();
    let stats = analyze_parallel(&text, threads);
    let elapsed = start.elapsed();

    println!("File:        {path}");
    println!("Threads:     {threads}");
    println!("Lines:       {} ({} malformed)", stats.total_lines, stats.malformed);
    println!("Time:        {elapsed:.2?}\n");

    println!("By level:");
    for level in [Level::Error, Level::Warn, Level::Info, Level::Debug] {
        println!("  {:<6} {}", format!("{level:?}").to_uppercase(), stats.by_level.get(&level).unwrap_or(&0));
    }

    println!("\nBy component:");
    let mut comps: Vec<_> = stats.by_component.iter().collect();
    comps.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (name, count) in comps {
        println!("  {name:<12} {count}");
    }

    println!("\nTop errors:");
    for (msg, count) in stats.top_errors(5) {
        println!("  {count:>7}  {msg}");
    }
}
