# logscan

A concurrent, multi-threaded log file analyzer written in Rust, using only the standard library (no dependencies).

It parses blockchain-node-style logs and reports line counts by level and component, plus the most frequent errors.

## Log format

```
<timestamp> <LEVEL> <component> <message...>
2026-10-01T12:00:01Z ERROR mempool tx failed: invalid signature
```

Levels: `DEBUG`, `INFO`, `WARN`, `ERROR`. Malformed lines are counted, not fatal.

## Quick start

```bash
# 1. generate a sample log (3 million lines)
cargo run --release --example gen_log -- 3000000 > node.log

# 2. analyze it
cargo run --release -- node.log --threads 4

# 3. run the tests
cargo test
```

## How the concurrency works

1. The file is read into one `String`.
2. `split_chunks` cuts it into N chunks **on newline boundaries**, so no line is ever split between threads.
3. `std::thread::scope` spawns one worker per chunk. Scoped threads can borrow the text directly, so nothing is copied or wrapped in `Arc`.
4. Each worker builds its own `Stats`, so there are no locks and no shared mutable state, and sends it back over an `mpsc` channel.
5. The main thread merges the partial results.

A test checks that the parallel result is identical to the single-threaded result for 1, 2, 3, 8, and 64 threads.

## Layout

```
src/lib.rs            parsing, stats, chunking, parallel analysis
src/main.rs           CLI
examples/gen_log.rs   synthetic log generator
tests/parallel.rs     unit + integration tests
```
