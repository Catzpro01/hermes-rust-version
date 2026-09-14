# Evidence documentation corrections

## 2026-09-14 — package name in ui-3b39bd7 reproduction

The immutable `ui-3b39bd7/REPORT.md` reproduction example incorrectly uses
`cargo build -p hermes-cli`. The directory is named `crates/hermes-cli`, but
its Cargo package name is **hermes-rs**. The corrected command is:

```sh
cargo build --locked -p hermes-rs --bin hermes-rs --example visual_summary
```

Actual capture workflows used `-p hermes-rs`; this documentation error does not
change their evidence provenance. The original packet and checksum are preserved.
