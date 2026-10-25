#!/usr/bin/env python3
"""Stalium: writes the scrambled directory address of crates/omsi-app/src/stalium_bcs.rs.
   python3 scripts/stalium/servers.py 82.165.67.127:27027
(the web port of the Stalium server that holds bcs.txt). Prints the bytes to paste into DIRECTORY."""
import sys, textwrap
KEY = b"Stalium-BCS-2026"
data = sys.argv[1].strip().encode()
x = bytes(b ^ KEY[i % len(KEY)] for i, b in enumerate(data))
print("\n".join("    " + l for l in textwrap.wrap(", ".join(f"0x{b:02x}" for b in x), 96)))
