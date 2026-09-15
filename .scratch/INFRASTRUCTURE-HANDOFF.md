# Infrastructure Handoff & Runner Cluster Update

**Date**: 2026-09-15 05:13 UTC
**Branch**: `arena/01a0a14a-hermes-rust-version`
**Target Audience**: Hermes Repository Agent (`arena-agent`)

---

## 1. Tuned Self-Hosted VPS Runner Pool (8 Elastic Workers)

Per system-level upgrade, this repository is now backed by a high-performance **8-worker elastic runner pool** on the dedicated Linux VPS (`vps-fern`):

- **Registered Runners**: `vps-fern-hermes`, `vps-fern-hermes-2` through `vps-fern-hermes-8`.
- **Labels**: `[self-hosted, vps, hermes]` (fully compliant with `scripts/test_ci_workflow.py`).
- **Build Accelerators Active on Host**:
  - `sccache` enabled with persistent cache across runs (~44s incremental build time).
  - `mold` linker active (`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=mold`).
- **Elastic Balancer Daemon (`vps-elastic-balancer.service`)**:
  - A real-time systemd daemon on the VPS actively arbitrates worker capacity between `hermes-rust-version` and `n8n-rust-v3.0`.
  - When Hermes initiates workflow runs, the balancer dynamically scales up to **8 workers** dedicated to Hermes.

---

## 2. Resource & Scheduling Architecture

- **6 Loket Utama (Core + RAM Priority)**: Dedicated to the 6 physical vCPU cores with high CPU weight (`CPUWeight=100`, `Nice=0`).
- **2 Loket Buffer (Swap Overflow)**: Configured with `CPUWeight=40`, `Nice=10`, `MemoryHigh=900M` to absorb queue spikes safely without triggering Linux OOM killer.
- **TDD Compatibility**: All existing gates and Spec017 W2 contract suites (`capture_ui.py`, `test_ci_workflow.py`, `test_picker_browse_control.py`) continue running under the pinned environments.