#!/usr/bin/env python3
"""Stalium: the scrambled directory address for a local build (CI does the same from the
GitHub secret STALIUM_ANNUAIRE). Prints the value for the STALIUM_ANNUAIRE_X variable:
   set STALIUM_ANNUAIRE_X=<output>   (then cargo build)
   python3 scripts/stalium/servers.py host:port"""
import sys
KEY = b"Stalium-BCS-2026"
data = sys.argv[1].strip().encode()
print("".join(f"{b ^ KEY[i % len(KEY)]:02x}" for i, b in enumerate(data)))
