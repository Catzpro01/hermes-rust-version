# 012-03: Status panels — token meter, provider, goal/plan/reflection, sessions

**Status:** breakdown.

**What to build:** Panel status yang menampilkan data yang sudah tersedia:
token meter (Spec 008 `estimated_tokens`/`context_limit`), provider aktif +
`/provider` list, goal status + plan + reflection (Spec 009 aksesor publik),
session list (Spec 003 `SessionStore::list`), MCP server status (Spec 011).

## Desain

- Header bar: provider aktif, nama session, estimated context ~N / limit,
  compression on/off (reuse `compression_label` / `ResolvedContext`).
- Status panel: goal [status] + teks (sanitized/redacted), plan steps
  (sanitized), reflection on/off + reflections_used, pinned count, MCP servers
  (nama + status + tool count dari `McpHandle`).
- Session strip: list session id + turn count dari `SessionStore`.
- Semua teks melewati `sanitize_untrusted_output` + `redact_credentials`
  sebelum ditampilkan (reuse fungsi yang dipakai REPL).

## Kriteria

- [ ] Token meter & limit ditampilkan (sumber: `runner.estimated_tokens()`,
      `ctx.limit`).
- [ ] Goal/plan/reflection/pinned/provider/MCP ditampilkan dari aksesor publik.
- [ ] Session list ditampilkan dari `store`.
- [ ] Semua nilai user/model di-sanitasi & di-redaksi di render boundary.
- [ ] Unit test pemformat baris (helper pure) + clippy bersih.
