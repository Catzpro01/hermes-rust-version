# ADR 0001: Compatibility-first rewrite

- Status: accepted
- Date: 2026-09-04

## Decision

Hermes-RS will be developed as a compatibility-first Rust rewrite. The existing Python Hermes installation and `~/.hermes` data remain intact. Rust adapters will initially prefer read-only compatibility and explicit migration steps over implicit mutation.

## Rationale

This permits incremental replacement, preserves existing memories and skills, and makes rollback possible while the Rust implementation is incomplete.
