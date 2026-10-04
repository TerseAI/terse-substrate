#!/usr/bin/env python3
"""Verify or update the pinned, unmodified Agent Substrate API schema."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
FILES = {"proto/ateapi.proto": "pkg/proto/ateapipb/ateapi.proto", "LICENSE": "LICENSE"}
REPOSITORY = "https://github.com/agent-substrate/substrate"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--revision", help="Full 40-character upstream commit SHA")
    parser.add_argument("--release", help="Corresponding upstream runtime release")
    args = parser.parse_args()
    if args.check and (args.revision or args.release):
        parser.error("--check verifies the existing pin; omit --revision and --release")
    if bool(args.revision) != bool(args.release):
        parser.error("--revision and --release must be supplied together")
    manifest = json.loads((ROOT / "upstream.json").read_text())
    revision = args.revision or manifest["revision"]
    if not re.fullmatch(r"[0-9a-f]{40}", revision):
        parser.error("revision must be a full lowercase commit SHA")
    downloads = {local: fetch(revision, upstream) for local, upstream in FILES.items()}
    checksums = {name: hashlib.sha256(data).hexdigest() for name, data in downloads.items()}
    if not args.revision and checksums != manifest["sha256"]:
        raise SystemExit("Upstream content does not match the pinned checksums")
    if args.check:
        for name, data in downloads.items():
            if (ROOT / name).read_bytes() != data:
                raise SystemExit(f"{name} differs from pinned upstream content")
        print(f"Verified upstream revision {revision}")
        return
    for name, data in downloads.items():
        (ROOT / name).write_bytes(data)
    manifest = {"repository": REPOSITORY, "revision": revision,
                "release": args.release or manifest["release"], "sha256": checksums}
    (ROOT / "upstream.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Synced upstream revision {revision}; review the diff and test before releasing")


def fetch(revision, path):
    url = f"https://raw.githubusercontent.com/agent-substrate/substrate/{revision}/{path}"
    request = urllib.request.Request(url, headers={"User-Agent": "terse-substrate-updater"})
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read()


if __name__ == "__main__":
    main()
