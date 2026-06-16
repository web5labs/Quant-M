# Quant-M Fuzz Baseline v0.1.1

## Baseline

Existing baseline runs completed with no crashes:

- `ingest_json`: `549,288` executions in `30s`, `17,718 exec/s`, `122MB` peak RSS
- `forex_ingest_payload`: `166,789` executions in `10s`, `15,162 exec/s`, `69MB` peak RSS
- `forex_macro_rows`: `199,871` executions in `10s`, `18,170 exec/s`, `67MB` peak RSS
- `fuzz/artifacts` remained empty

## Framework Surface Coverage

New framework-surface fuzz runs also completed with no crashes:

- `agent_shell_command`: `281,557` executions in `30s`, `9,082 exec/s`, `454MB` peak RSS
- `config_toml`: `92,780` executions in `30s`, `2,992 exec/s`, `409MB` peak RSS
- `shared_state_record`: `413,636` executions in `30s`, `13,343 exec/s`, `493MB` peak RSS
- `session_event`: `234,917` executions in `30s`, `7,577 exec/s`, `507MB` peak RSS
- `workflow_descriptor`: `316,879` executions in `30s`, `10,221 exec/s`, `447MB` peak RSS
- `fsm_descriptor`: `312,334` executions in `30s`, `10,075 exec/s`, `448MB` peak RSS
- `scheduler_descriptor`: `203,885` executions in `30s`, `6,576 exec/s`, `436MB` peak RSS
- `execution_runtime_request`: `469,158` executions in `30s`, `15,134 exec/s`, `358MB` peak RSS

## Commands Used

```bash
cargo +nightly fuzz run ingest_json -- -max_total_time=30 -print_final_stats=1 -verbosity=0
cargo +nightly fuzz run forex_ingest_payload -- -max_total_time=10 -print_final_stats=1 -verbosity=0
cargo +nightly fuzz run forex_macro_rows -- -max_total_time=10 -print_final_stats=1 -verbosity=0
cargo +nightly fuzz run agent_shell_command -- -max_total_time=30 -print_final_stats=1 -verbosity=0
cargo +nightly fuzz run config_toml -- -max_total_time=30 -print_final_stats=1 -verbosity=0
cargo +nightly fuzz run shared_state_record -- -max_total_time=30 -print_final_stats=1 -verbosity=0
cargo +nightly fuzz run session_event -- -max_total_time=30 -print_final_stats=1 -verbosity=0
cargo +nightly fuzz run workflow_descriptor -- -max_total_time=30 -print_final_stats=1 -verbosity=0
cargo +nightly fuzz run fsm_descriptor -- -max_total_time=30 -print_final_stats=1 -verbosity=0
cargo +nightly fuzz run scheduler_descriptor -- -max_total_time=30 -print_final_stats=1 -verbosity=0
cargo +nightly fuzz run execution_runtime_request -- -max_total_time=30 -print_final_stats=1 -verbosity=0
```

## Notes

- The sanitizer build cost was paid after the local `target/` prune, so the first run included the full fuzz-harness rebuild.
- macOS symbolizer warnings were non-fatal and did not indicate Quant-M crashes.
- The older forex targets still look healthy, but they now measure use-case parsing more than the newer framework shell/runtime surfaces.
- The framework-surface targets are slower and heavier by design because they exercise typed Serde normalization and registry validation instead of narrow ingest shims.
- `session_event` initially failed to compile due to a malformed fuzz-target format string; that target was fixed and rerun successfully before this baseline was finalized.
- `fuzz/target` grew to roughly `1.7G` after the expanded harness build. It can be pruned again when disk pressure matters more than warm fuzz-build reuse.

## v0.1.1 Shift

The fuzz scope now covers the Rust agent-runtime surfaces added after v0.1:

- agent shell command parsing
- typed Serde config parsing and validation
- shared-state record parsing and validation
- session event parsing and replay-safe decoding
- workflow descriptor validation
- fsm descriptor validation
- scheduler descriptor cadence validation
- execution runtime request parsing without execution

That keeps fuzzing aligned with Quant-M as a terminal-native Rust agentic framework rather than only a trading-ingest runtime.
