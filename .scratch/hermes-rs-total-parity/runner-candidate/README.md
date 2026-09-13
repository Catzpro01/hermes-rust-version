# Pre-commit Rust validation request

This directory can transport an **unapplied test proposal**, not an unchecked
production-source change. The branch-scoped push workflow checks source/patch hashes, permits
only welcome.rs changes in its isolated runner, formats/checks it, and requires
the named regression to fail in RED mode.

Current request is `none`: the GREEN-tested delta has been applied locally
and the consumed proposal removed. The next push captures committed source.

After observing RED, replace the proposal with the smallest correction plus
regression and set phase GREEN. The runner must pass the named regression,
fmt/check/clippy/full tests before its checksummed delta is applied locally.
Once that delta is committed as actual Rust, set phase none and remove the
consumed candidate patch to capture from the clean committed source.

A startup/compile/policy failure is NOT a successful RED test. Push validation
now runs using official rustup; third-party wrapper actions were removed.
Manual dispatch remains unavailable but is not required. No server policy
change or additional permissions are needed for the current push route.
See request.json, T12 and PROGRESS.md for the latest phase and observed runs.
