#!/usr/bin/env python3
"""Stalium: writes the scrambled server list of crates/omsi-app/src/stalium_bcs.rs.
   python3 scripts/stalium/servers.py "maps/st-servan/global.cfg=host:port" ...
(map names in lower case, as maps/<folder>/global.cfg). Prints the bytes to paste into SERVERS."""
import sys, textwrap
KEY = b"Stalium-BCS-2026"
data = "\n".join(sys.argv[1:]).encode()
x = bytes(b ^ KEY[i % len(KEY)] for i, b in enumerate(data))
print("\n".join("    " + l for l in textwrap.wrap(", ".join(f"0x{b:02x}" for b in x), 96)))
