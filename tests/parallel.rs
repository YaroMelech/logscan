use logscan::*;

const SAMPLE: &str = "\
2026-10-01T00:00:01Z INFO consensus block produced
2026-10-01T00:00:02Z ERROR mempool tx failed: invalid signature
2026-10-01T00:00:03Z WARN network slow peer response
2026-10-01T00:00:04Z ERROR mempool tx failed: invalid signature
garbage line
2026-10-01T00:00:05Z DEBUG rpc heartbeat
";

#[test]
fn parses_a_valid_line() {
    let e = parse_line("2026-10-01T00:00:01Z ERROR wallet tx failed: bad nonce").unwrap();
    assert_eq!(e.level, Level::Error);
    assert_eq!(e.component, "wallet");
    assert_eq!(e.message, "tx failed: bad nonce");
}

#[test]
fn rejects_malformed_lines() {
    assert!(parse_line("").is_none());
    assert!(parse_line("only two").is_none());
    assert!(parse_line("2026-10-01T00:00:01Z LOUD wallet hi").is_none());
}

#[test]
fn counts_levels_and_errors() {
    let s = analyze_sequential(SAMPLE);
    assert_eq!(s.total_lines, 6);
    assert_eq!(s.malformed, 1);
    assert_eq!(s.by_level[&Level::Error], 2);
    assert_eq!(s.top_errors(1), vec![("tx failed: invalid signature", 2)]);
}

#[test]
fn chunks_never_split_lines_and_cover_everything() {
    for n in 1..=10 {
        let chunks = split_chunks(SAMPLE, n);
        assert_eq!(chunks.concat(), SAMPLE);
        for c in &chunks {
            assert!(c.ends_with('\n'), "chunk must end on a line boundary");
        }
    }
}

#[test]
fn parallel_matches_sequential_for_any_thread_count() {
    let big = SAMPLE.repeat(500);
    let expected = analyze_sequential(&big);
    for threads in [1, 2, 3, 8, 64] {
        assert_eq!(analyze_parallel(&big, threads), expected, "threads = {threads}");
    }
}

#[test]
fn handles_empty_input() {
    let s = analyze_parallel("", 4);
    assert_eq!(s.total_lines, 0);
}
