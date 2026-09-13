# T06 — Provider catalog (39) / `hermes model` picker (Spec 017)

Status: IN REVIEW (CI-verified)

- Katalog `CANONICAL_PROVIDERS` (39 `ProviderEntry{id,label,description}`)
  sudah verbatim di `wizard/catalog.rs` (T05), diuji ulang vs
  `verbatim/providers_canonical.txt`.
- `hermes model` interaktif (TTY, tanpa `--provider`) sekarang **adalah**
  section `Model & Provider` wizard — satu code path seperti Python
  (`setup.py` mendelegasikan ke `cmd_model`, spec §C.3). Stub lama
  (41 string hardcode + `No change.` palsu) dihapus.
- Piped / `--provider <name>`: listing lama `Providers:` tetap byte-stable
  (E2E Spec 014 tidak berubah).
- Tidak diport: live model list per provider (§G.3 remote catalog) —
  model diisi manual (prefill dari config).
- E2E: `model_tty_is_the_provider_picker`.
