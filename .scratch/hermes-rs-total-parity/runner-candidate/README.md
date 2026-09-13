# Pre-commit Rust validation request

These files transport an **unapplied test proposal**, not a production-source
change. The branch-scoped push workflow checks source/patch hashes, permits
only welcome.rs changes in its isolated runner, formats/checks it, and requires
the named regression to fail in RED mode. No Rust fix is validated yet.

After observing RED, replace the proposal with the smallest correction plus
regression and set phase GREEN. The runner must pass the named regression,
fmt/check/clippy/full tests before its checksummed delta is applied locally.
Once that delta is committed as actual Rust, set phase none and remove the
consumed candidate patch to capture from the clean committed source.

A startup/compile/policy failure is NOT a successful RED test. Current blocker
is the malformed selected-action allowlist reported in T12, not patch logic.
To retry after policy repair, add/update a non-secret retry marker in
request.json and push the same assigned branch. No manual dispatch is needed.
