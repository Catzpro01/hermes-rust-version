# Real PTY banner capture and replay

This is the first Spec 017 evidence slice, not a full-CLI parity suite.
See the [current evidence report](../../docs/hermes-ui-spec/017/evidence/README.md).
No image-generation model is used. Python calls upstream's public banner
component with explicit fixture data; Rust launches the actual offline REPL.
The registry mapping is fixture data, not proof of complete Python startup.

## Requirements / isolation

Linux, Python 3.11+, Node 20.11+, git/gh, and Rust for Rust capture (or the
existing GitHub capture workflow). Run commands from the repository root.
Use a new cache directory. Never point `reference` at an installed Hermes
home. The capture child environment is allowlisted and HOME/HERMES_HOME are
temporary directories; no provider secrets are passed. Captures use fake
provider, never send a model request, and never execute user tool commands.

```bash
CACHE="$HOME/.cache/hermes-visual-repro"
mkdir -p "$CACHE"
python3 -m pip install --target "$CACHE/python" -r scripts/visual-renderer/requirements-banner.txt
npm ci --prefix scripts/visual-renderer
```

The pinned npm Chromium package avoids requiring a system browser. On lean
Linux systems, its packaged shared libraries may also be needed:

```bash
node - "$CACHE" <<'JS'
const fs=require('fs'), path=require('path'), z=require('zlib');
const cache=process.argv[2];
const root=path.resolve('scripts/visual-renderer/node_modules/@sparticuz/chromium');
fs.writeFileSync(path.join(cache,'libraries.tar'),
  z.brotliDecompressSync(fs.readFileSync(path.join(root,'bin/al2023.tar.br'))));
JS
mkdir -p "$CACHE/libraries"
tar -xf "$CACHE/libraries.tar" -C "$CACHE/libraries"
export LD_LIBRARY_PATH="$CACHE/libraries/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
```

## Prepare a separate verified Python reference

```bash
gh api repos/NousResearch/hermes-agent/tarball/63279301bcbdc185c1b07b98a9312eb0c862f26d > "$CACHE/upstream.tar.gz"
python3 - "$CACHE" <<'PY'
import hashlib, json, sys, tarfile
from pathlib import Path
cache=Path(sys.argv[1]).resolve()
lock=json.loads(Path('docs/hermes-ui-spec/017/evidence/reference.json').read_text())
archive=cache/'upstream.tar.gz'
assert hashlib.sha256(archive.read_bytes()).hexdigest()==lock['archive_sha256'], 'Archive digest mismatch'
dest=cache/'source'
dest.mkdir()  # Fail rather than overwrite an existing source/install.
with tarfile.open(archive) as t:
    members=t.getmembers()
    for m in members:
        assert m.isfile() or m.isdir(), 'Unsupported archive member'
        assert (dest/m.name).resolve().is_relative_to(dest), 'Unsafe archive path'
    t.extractall(dest, members=members)
roots=list(dest.iterdir()); assert len(roots)==1
ref=roots[0]
for name, digest in lock['source_files_sha256'].items():
    assert hashlib.sha256((ref/name).read_bytes()).hexdigest()==digest, name
(ref/'.visual-evidence-reference').write_text(lock['commit']+'\n')
print(ref)
PY
```

Set `REF` to the printed directory. `pair` checks the prepared-reference
marker and pinned banner/skin/version/constants hashes before importing any
reference code. It adds only the supported `.hermes_build_sha` metadata to
this isolated archive so the real renderer can display the actual commit.
No upstream Python source is edited. If GitHub changes archive packaging,
investigate a digest mismatch; do not bypass it or assume it is harmless.

## Capture and replay

With a local Rust toolchain:

```bash
cargo build --locked -p hermes-rs
python3 scripts/capture_banner.py rust target/debug/hermes-rs "$CACHE/rust-bundle.json"
PYTHONPATH="$CACHE/python" python3 scripts/capture_banner.py pair "$CACHE/rust-bundle.json" "$REF" "$CACHE/paired-bundle.json"
node scripts/visual-renderer/render.cjs "$CACHE/paired-bundle.json" "$CACHE/rendered"
```

Alternatively use `.github/workflows/visual-evidence.yml` on the assigned
branch. The workflow is read-only, never commits or pushes, and can validate
an optional pre-commit patch restricted to `welcome.rs`. RED mode requires
the named geometry assertion to fail (a compiler error or zero matched tests
is not accepted); GREEN requires the named test and full workspace checks.
Do not apply a candidate locally as a verified fix until these checks pass.
Manual dispatch still lacks permission, but authorized branch-scoped push
requests in `.scratch/hermes-rs-total-parity/runner-candidate/` now work.
After applying the verified delta, consume/remove the proposal and set phase
`none` to capture clean committed source. Check its source hash and empty
Rust-worktree diff, plus compiler/Cargo versions; verify that commit's CI too.

## Transport and audit

The artifact is primary. If signed downloads fail, two annotation groups
carry at most 16 chunks of 3,000 base64 characters. Retrieve annotations from
the exact workflow job's `check_run_url`, identify `visual bundle i/N`, reject
duplicate/missing/inconsistent indices, base64-decode with validation, gunzip,
and verify both SHA-256 and byte count against `visual bundle digest`.
Also verify `rust_commit` against the run's head SHA. Never concatenate
truncated output or treat a partial payload as a complete capture.

The renderer verifies each raw SHA-256 and the reconstruction of raw bytes
from timestamped chunks. It preserves `.ansi` and emits `.cast` recordings;
images replay only the explicitly recorded prefix ending at the banner
boundary. Both sides use the same terminal/font/renderer with no stream
normalization. Top/bottom scrollback screenshots retain the original 30-row
viewport. `*-cells.json.gz` contains emulator cell geometry and attributes.

Render/capture success only means artifacts were produced. It does not mean
visual equality, full Python CLI coverage, or user closure acceptance. Save
every rerun under a new path: tools refuse to overwrite originals.
