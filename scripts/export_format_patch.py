"""Export runner-generated Rust formatting changes for human/agent review.

Run only after cargo fmt and cargo check. No commit/push is performed.
The artifact is primary; compressed check annotations also let restricted
sandboxes retrieve the patch when signed artifact downloads are blocked.
Only tracked *.rs diffs are included, never config, credentials, or logs.
"""

import base64
import gzip
import hashlib
from pathlib import Path
import subprocess


def main():
    patch = subprocess.check_output(["git", "diff", "--binary", "--", "*.rs"])
    Path("fmt.patch").write_bytes(patch)
    digest = hashlib.sha256(patch).hexdigest()
    encoded = base64.b64encode(gzip.compress(patch, mtime=0)).decode("ascii")
    # Some GitHub API proxies truncate each annotation to 4096 characters.
    # Stay below that boundary so the compressed stream survives retrieval.
    chunk_size = 3000
    chunks = [encoded[i:i + chunk_size] for i in range(0, len(encoded), chunk_size)]
    # Avoid exhausting GitHub's annotation budget on a pathological diff.
    # The full patch remains available in the artifact in that case.
    print(f"::notice title=rustfmt patch digest::sha256={digest}; bytes={len(patch)}")
    if len(chunks) > 40:
        print("::notice title=rustfmt patch::Patch too large for annotations; use fmt.patch artifact.")
        return
    for i, chunk in enumerate(chunks, 1):
        print(f"::notice title=rustfmt patch {i}/{len(chunks)}::{chunk}")


if __name__ == "__main__":
    main()
