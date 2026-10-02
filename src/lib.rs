//! logscan: a concurrent log analyzer built on `std` only.
//!
//! Expected line format:
//!   `2026-10-01T12:00:01Z ERROR consensus tx failed: insufficient funds`
//!   `<timestamp> <LEVEL> <component> <message...>`

use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    fn parse(s: &str) -> Option<Level> {
        match s {
            "DEBUG" => Some(Level::Debug),
            "INFO" => Some(Level::Info),
            "WARN" => Some(Level::Warn),
            "ERROR" => Some(Level::Error),
            _ => None,
        }
    }
}

/// One successfully parsed log line (borrows from the input text, no allocation).
#[derive(Debug, PartialEq, Eq)]
pub struct Entry<'a> {
    pub timestamp: &'a str,
    pub level: Level,
    pub component: &'a str,
    pub message: &'a str,
}

/// Parse a single line. Returns `None` for malformed lines.
pub fn parse_line(line: &str) -> Option<Entry<'_>> {
    let mut parts = line.trim().splitn(4, ' ');
    let timestamp = parts.next()?;
    let level = Level::parse(parts.next()?)?;
    let component = parts.next()?;
    let message = parts.next()?;
    Some(Entry { timestamp, level, component, message })
}

/// Aggregated statistics. Each worker builds its own `Stats`; they are
/// merged at the end, so no locks are needed while parsing.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub total_lines: usize,
    pub malformed: usize,
    pub by_level: HashMap<Level, usize>,
    pub by_component: HashMap<String, usize>,
    pub error_messages: HashMap<String, usize>,
}

impl Stats {
    pub fn merge(&mut self, other: Stats) {
        self.total_lines += other.total_lines;
        self.malformed += other.malformed;
        for (k, v) in other.by_level {
            *self.by_level.entry(k).or_insert(0) += v;
        }
        for (k, v) in other.by_component {
            *self.by_component.entry(k).or_insert(0) += v;
        }
        for (k, v) in other.error_messages {
            *self.error_messages.entry(k).or_insert(0) += v;
        }
    }

    /// The `n` most frequent ERROR messages, ties broken alphabetically.
    pub fn top_errors(&self, n: usize) -> Vec<(&str, usize)> {
        let mut v: Vec<(&str, usize)> =
            self.error_messages.iter().map(|(k, c)| (k.as_str(), *c)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        v.truncate(n);
        v
    }
}

/// Analyze a chunk of text (whole lines only) on the current thread.
pub fn analyze_chunk(chunk: &str) -> Stats {
    let mut stats = Stats::default();
    for line in chunk.lines().filter(|l| !l.trim().is_empty()) {
        stats.total_lines += 1;
        match parse_line(line) {
            Some(e) => {
                *stats.by_level.entry(e.level).or_insert(0) += 1;
                *stats.by_component.entry(e.component.to_string()).or_insert(0) += 1;
                if e.level == Level::Error {
                    *stats.error_messages.entry(e.message.to_string()).or_insert(0) += 1;
                }
            }
            None => stats.malformed += 1,
        }
    }
    stats
}

/// Split `text` into at most `n` chunks, always cutting on a newline so no
/// line is ever split between two threads.
pub fn split_chunks(text: &str, n: usize) -> Vec<&str> {
    let n = n.max(1);
    let target = text.len() / n + 1;
    let mut chunks = Vec::with_capacity(n);
    let mut start = 0;
    while start < text.len() {
        let mut end = (start + target).min(text.len());
        if end < text.len() {
            // advance to the end of the current line
            end = match text[end..].find('\n') {
                Some(i) => end + i + 1,
                None => text.len(),
            };
        }
        // `end` is now on a char boundary (we land right after '\n' or at len)
        chunks.push(&text[start..end]);
        start = end;
    }
    chunks
}

/// Single-threaded baseline (used by tests and benchmarks).
pub fn analyze_sequential(text: &str) -> Stats {
    analyze_chunk(text)
}

/// Multi-threaded analysis: one scoped worker thread per chunk, each sends its
/// partial `Stats` over an `mpsc` channel; the main thread merges the results.
pub fn analyze_parallel(text: &str, threads: usize) -> Stats {
    let chunks = split_chunks(text, threads);
    let (tx, rx) = mpsc::channel::<Stats>();

    thread::scope(|scope| {
        for chunk in chunks {
            let tx = tx.clone();
            scope.spawn(move || {
                // ignore send errors: receiver only drops if main thread panicked
                let _ = tx.send(analyze_chunk(chunk));
            });
        }
        drop(tx); // close our own sender so `rx` ends when workers finish
    });

    let mut total = Stats::default();
    for partial in rx {
        total.merge(partial);
    }
    total
}
