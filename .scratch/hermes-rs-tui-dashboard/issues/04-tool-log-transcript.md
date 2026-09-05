# 012-04: Tool log panel + streaming transcript panel + redaksi

**Status:** breakdown.

**What to build:** Panel bawah: (a) transkrip percakapan streaming (chunk dari
model muncul live), (b) tool log panel yang menampilkan tiap tool call
(nama `{server}__{tool}` / bawaan, argumen, status) saat dieksekusi — sumber
dari event `ToolStarted`/`ToolDone` dan/atau `tool_calls` di `state.db`.

## Desain

- Transcript panel: menampilkan `Turn` terakhir + chunk streaming; scroll;
  input user di baris bawah. Teks model/tool di-sanitasi & di-redaksi.
- Tool log panel: daftar `{name}` + status (`Success/Error/Denied/Timeout`)
  + baris argumen ringkas (di-redaksi; potong panjang). Data dari event live
  (`ToolStarted`/`ToolDone`) dan, bila diminta, replay dari `store` utk session
  lama.
- Reuse parser/status yang sudah ada (`ToolExecutionStatus::as_str`).
- Tidak menambah jalur output yang tak di-sanitasi.

## Kriteria

- [ ] Chunk streaming tampil live di panel transcript.
- [ ] Tool call live muncul di tool log dgn nama/status; argumen diringkas &
      di-redaksi.
- [ ] Replay tool log dari `state.db` utk session lama (opsional, bila murah).
- [ ] Scroll transcript/tool log berfungsi.
- [ ] Unit test helper (formatting ringkas, redaksi) + clippy bersih.
