# Token Generator Micro-Benchmark

## Target

- Function: `log_level(verbose: bool) -> log::LevelFilter`
- Harness: ignored unit test `benchmark_log_level`
- Command:

```bash
SSL_CERT_FILE=/etc/ssl/cert.pem \
cargo test -p token-generator --release benchmark_log_level -- --ignored --nocapture
```

## Baseline

Recorded on branch `agadgil/ex-5` after adding the benchmark harness.

| Run | Iterations | Elapsed (ns) | ns/iter |
| --- | --- | ---: | ---: |
| 1 | 5,000,000 | 7,941,542 | 1.59 |
| 2 | 5,000,000 | 4,454,916 | 0.89 |
| 3 | 5,000,000 | 4,349,833 | 0.87 |

## Variance notes

- This is a very small helper, so absolute times are sensitive to CPU scheduling, turbo/thermal state, and other machine activity.
- Compare `ns/iter` values across repeated release-mode runs rather than relying on a single sample.
- The benchmark includes `black_box` to reduce optimization artifacts, but it is still a lightweight timing harness rather than a full statistical benchmark suite.
- The first run was noticeably slower than runs 2-3, so treat it as warm-up affected. The steadier baseline here is roughly **0.87-0.89 ns/iter** after warm-up.
- On this machine, release-mode benchmark runs also required `SSL_CERT_FILE=/etc/ssl/cert.pem` because a transitive `habitat_core` build script checks for that environment variable.
