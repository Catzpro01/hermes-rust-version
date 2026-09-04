# ADR 0002: Reproduce Hermes behavior in staged slices

- Status: accepted
- Date: 2026-09-04

## Decision

Hermes-RS will mirror the behavior of the upstream Hermes Agent and will be implemented in vertical slices. The first slice is the interactive CLI: configuration loading, provider selection, prompt submission, streamed response output, and session persistence.

## Rationale

A behavior-first sequence gives a usable Rust agent early while preserving a clear parity path for tools, skills, memory, MCP, gateways, scheduling, and dashboard features.

## Primary source

The current upstream implementation is checked out at `/home/fern/.hermes/hermes-agent`. Hermes-RS must not mutate that installation while using it as a reference.
