# PortWatch

PortWatch is a Rust library for decoding offline harbor-operations exchange files.
The format models the messy handoff between vessel traffic systems, berth planners,
pilot stations, hazardous-cargo offices, and tide-window services when those
systems synchronize by file drop rather than by live network access.

The repository is intentionally dependency-free for hermetic fuzzing. It contains
three cargo-fuzz targets under `fuzz/` and a ClusterFuzzLite build script that
builds each harness into `$OUT` without fetching anything from the network.

## Fuzz Targets

- `stream_fuzzer` exercises the framed exchange stream, compression layer,
  dictionaries, templates, session assembly, and analyzer.
- `manifest_fuzzer` exercises the archive manifest decoder and section routing.
- `ledger_fuzzer` drives the stateful vessel ledger entry point directly.

Seed corpora live under `fuzz/corpus/<target>/`; the shared dictionary is
`fuzz/dictionary.txt`.
