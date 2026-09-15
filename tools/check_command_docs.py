#!/usr/bin/env python3
"""Sanity-check generated Minecraft command specs and the Bedrock example pack."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "docs/minecraft-commands"
EXAMPLE = Path(__file__).resolve().parents[1] / "examples/bedrock-metro-escape"
GAMEPLAY = ROOT / "bedrock/gameplay"

ALIASES = {"tp": "teleport", "w": "tell", "msg": "tell"}

JAVA_LEAKS = (
    "execute store",
    "execute on ",
    "execute if data",
    "execute unless data",
    "execute if predicate",
    "execute if items",
    "execute if slots",
    "nbt=",
    "distance=",
    "limit=",
    "sort=",
    "@n[",
    "item replace",
    "objectives add random test",
)

COMMAND_ROOT = re.compile(r"^[A-Za-z][A-Za-z0-9]*")
FUNCTION_TOKEN = re.compile(r"\b(?:function|schedule delay add)\s+([A-Za-z0-9_./-]+)")
METRO_FN = re.compile(r"\b(metro/[A-Za-z0-9_/-]+)")


def command_verb(line: str) -> str | None:
    text = line.strip()
    if not text or text.startswith("#"):
        return None
    if text.startswith("execute "):
        idx = text.rfind(" run ")
        if idx == -1:
            return "execute"
        text = text[idx + 5 :].lstrip()
    match = COMMAND_ROOT.match(text)
    return match.group(0) if match else None


def iter_mcfunction_lines(path: Path) -> list[tuple[int, str]]:
    rows: list[tuple[int, str]] = []
    for i, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = raw.strip()
        if stripped and not stripped.startswith("#"):
            rows.append((i, stripped))
    return rows


def check_example(be_roots: set[str], errors: list[str]) -> int:
    if not EXAMPLE.is_dir():
        errors.append("missing examples/bedrock-metro-escape")
        return 0

    tick_path = EXAMPLE / "functions/tick.json"
    if not tick_path.is_file():
        errors.append("example pack missing functions/tick.json")
        return 0
    tick = json.loads(tick_path.read_text(encoding="utf-8"))
    values = tick.get("values")
    if not isinstance(values, list) or not values:
        errors.append("tick.json must contain a non-empty values array")
        return 0

    manifest = json.loads((EXAMPLE / "manifest.json").read_text(encoding="utf-8"))
    mev = manifest.get("header", {}).get("min_engine_version") or [0, 0, 0]
    if mev < [1, 19, 50]:
        errors.append("example min_engine_version must be >= 1.19.50 for new execute")

    json.loads((EXAMPLE / "dialogue/metro_lobby.json").read_text(encoding="utf-8"))

    functions = {p.relative_to(EXAMPLE / "functions").with_suffix("").as_posix(): p for p in (EXAMPLE / "functions").rglob("*.mcfunction")}
    cmd_count = 0
    for rel, path in sorted(functions.items()):
        for lineno, line in iter_mcfunction_lines(path):
            cmd_count += 1
            loc = f"{path.relative_to(EXAMPLE.parent.parent)}:{lineno}"
            if line.startswith("/"):
                errors.append(f"{loc} mcfunction must not start with /")
            for leak in JAVA_LEAKS:
                if leak in line:
                    errors.append(f"{loc} Java-only token {leak!r}")
            if line.startswith("scoreboard objectives add") and " dummy" not in line:
                errors.append(f"{loc} scoreboard add must use dummy")
            verb = command_verb(line)
            if verb is None:
                errors.append(f"{loc} could not read command verb")
                continue
            verb = ALIASES.get(verb, verb)
            if verb not in be_roots:
                errors.append(f"{loc} unknown Bedrock root {verb!r}")

    for name in values:
        if name not in functions:
            errors.append(f"tick.json value {name!r} has no .mcfunction")

    referenced = set(values)
    for path in functions.values():
        text = path.read_text(encoding="utf-8")
        referenced.update(FUNCTION_TOKEN.findall(text))
        referenced.update(METRO_FN.findall(text))
    for name in sorted(referenced):
        if name.startswith("metro/") and name not in functions:
            errors.append(f"missing function file for {name}")
    return cmd_count


def check_gameplay_docs(errors: list[str]) -> None:
    if not GAMEPLAY.is_dir():
        errors.append("missing docs/minecraft-commands/bedrock/gameplay")
        return
    fence = re.compile(r"```(?:json|mcfunction|text)?\n(.*?)```", re.S)
    for path in GAMEPLAY.glob("*.md"):
        text = path.read_text(encoding="utf-8")
        for block in fence.findall(text):
            for leak in (
                "execute store",
                "nbt=",
                "distance=",
                "execute if data",
                "execute on vehicle",
                "@n[",
            ):
                if leak in block:
                    errors.append(f"{path.name} code block contains Java-only {leak!r}")


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

    example_cmds = check_example(set(be["root_commands"]), errors)
    check_gameplay_docs(errors)

    if errors:
        print("FAIL")
        for e in errors:
            print(e)
        return 1
    print(
        f"ok java={len(java['root_commands'])} bedrock={len(be['root_commands'])} "
        f"parsers={len(java['parsers'])} example_cmds={example_cmds}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
