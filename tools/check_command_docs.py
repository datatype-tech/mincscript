#!/usr/bin/env python3
"""Sanity-check generated Minecraft command specs."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "docs/minecraft-commands"


def main() -> int:
    java = json.loads((ROOT / "java/data/command-index.json").read_text())
    be = json.loads((ROOT / "bedrock/data/command-index.json").read_text())
    errors: list[str] = []

    if len(java["root_commands"]) < 90:
        errors.append(f"java root_commands too small: {len(java['root_commands'])}")
    if len(be["root_commands"]) != 82:
        errors.append(f"bedrock should have 82 official commands, got {len(be['root_commands'])}")

    je_exec = "\n".join(java["commands"]["execute"]["usages"])
    for needle in (
        "execute store result",
        "execute on vehicle",
        "execute if slots",
        "execute if items",
        "execute summon",
        "execute run",
    ):
        if needle not in je_exec:
            errors.append(f"java execute missing {needle!r}")

    be_exec = "\n".join(be["commands"]["execute"]["official_usages"])
    if "execute store" in be_exec:
        errors.append("bedrock official execute must not include store")
    if "execute as" not in be_exec or "execute run" not in be_exec:
        errors.append("bedrock execute missing as/run")

    if "data" not in java["root_commands"] or "data" in be["root_commands"]:
        errors.append("data must be java-only")
    if "replaceitem" not in be["root_commands"] or "replaceitem" in java["root_commands"]:
        errors.append("replaceitem must be bedrock-only")

    if errors:
        print("FAIL")
        for e in errors:
            print(e)
        return 1
    print(
        f"ok java={len(java['root_commands'])} bedrock={len(be['root_commands'])} "
        f"parsers={len(java['parsers'])}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
