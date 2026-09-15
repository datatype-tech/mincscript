#!/usr/bin/env python3
"""Extract Java (brigadier) and Bedrock (official Creator docs + wiki) command syntax.

Sources are expected at:
  /tmp/mc-src/mcmeta/commands.json
  /tmp/mc-src/mcmeta/versions.json
  /tmp/mc-src/msdocs/creator/Commands/commands/*.md
  /tmp/mc-src/wiki/*.wiki
"""

from __future__ import annotations

import json
import re
from collections import defaultdict
from html import unescape
from pathlib import Path

MC_SRC = Path("/tmp/mc-src")
OUT = Path("/workspace/docs/minecraft-commands")


def walk_usages(node: dict, tokens: list[str], out: list[dict]) -> None:
    children = node.get("children") or {}
    executable = bool(node.get("executable"))
    redirect = node.get("redirect")
    if executable:
        out.append(
            {
                "tokens": tokens[:],
                "usage": " ".join(tokens),
                "redirect": redirect,
            }
        )
    if redirect is not None and not children:
        out.append(
            {
                "tokens": tokens[:],
                "usage": " ".join(tokens) + " -> " + "/".join(redirect),
                "redirect": redirect,
            }
        )
        return
    for name, child in children.items():
        kind = child.get("type")
        if kind == "literal":
            nxt = name
        elif kind == "argument":
            parser = child.get("parser", "unknown")
            nxt = f"<{name}:{parser}>"
        else:
            nxt = name
        walk_usages(child, tokens + [nxt], out)
    if not children and not executable and redirect is None and tokens:
        out.append({"tokens": tokens[:], "usage": " ".join(tokens), "redirect": None})


def collect_parsers(node: dict, into: dict[str, dict]) -> None:
    parser = node.get("parser")
    if parser:
        entry = into.setdefault(parser, {"count": 0, "properties": [], "names": []})
        entry["count"] += 1
        props = node.get("properties")
        if props and props not in entry["properties"]:
            entry["properties"].append(props)
    for name, child in (node.get("children") or {}).items():
        if child.get("type") == "argument":
            collect_parsers(child, into)
            into[child["parser"]]["names"] = sorted(
                set(into[child["parser"]].get("names", []) + [name])
            )[:40]
        else:
            collect_parsers(child, into)


def extract_ms_syntax(md: str) -> list[str]:
    usages = []
    for m in re.finditer(r"^`(/[^`]+)`\s*$", md, re.M):
        usages.append(unescape(m.group(1).strip()))
    # also indented code fences that start with /
    for m in re.finditer(r"```[^\n]*\n(/[^\n]+)", md):
        usages.append(m.group(1).strip())
    # dedupe preserve order
    seen = set()
    out = []
    for u in usages:
        if u not in seen:
            seen.add(u)
            out.append(u)
    return out


def extract_wiki_syntax(wiki: str) -> dict[str, list[str]]:
    result = {"java": [], "bedrock": []}
    m = re.search(r"==\s*Syntax\s*==\s*(.*?)(?:\n==\s|\Z)", wiki, re.S)
    if not m:
        return result
    section = m.group(1)
    je = re.search(
        r"\*\s*'''Java Edition'''\s*(.*?)(?:\*\s*'''Bedrock Edition'''|\n===\s|\Z)",
        section,
        re.S,
    )
    be = re.search(
        r"\*\s*'''Bedrock Edition'''\s*(.*?)(?:\n===\s|\Z)",
        section,
        re.S,
    )
    code_re = re.compile(r"<code>(.*?)</code>", re.S)

    def codes(block: str) -> list[str]:
        items = []
        for c in code_re.findall(block or ""):
            t = re.sub(r"<[^>]+>", "", c)
            t = unescape(t).replace("&lt;", "<").replace("&gt;", ">")
            t = re.sub(r"\s+", " ", t).strip()
            if t:
                items.append(t)
        return items

    if je:
        result["java"] = codes(je.group(1))
    if be:
        result["bedrock"] = codes(be.group(1))
    return result


def java_extract() -> dict:
    versions = json.loads((MC_SRC / "mcmeta/versions.json").read_text())
    tree = json.loads((MC_SRC / "mcmeta/commands.json").read_text())
    latest = versions[0]
    stables = [v for v in versions if v.get("stable")]
    commands = {}
    for name, node in tree["children"].items():
        usages: list[dict] = []
        walk_usages(node, [name], usages)
        commands[name] = {
            "name": name,
            "permissions": node.get("permissions"),
            "usages": usages,
        }
    parsers: dict[str, dict] = {}
    collect_parsers(tree, parsers)
    execute = tree["children"]["execute"]
    execute_usages: list[dict] = []
    walk_usages(execute, ["execute"], execute_usages)
    return {
        "source": {
            "kind": "vanilla-brigadier-tree",
            "via": "https://github.com/misode/mcmeta/blob/summary/commands/data.json",
            "game_id": latest["id"],
            "game_name": latest["name"],
            "stable": latest.get("stable"),
            "latest_stable": stables[0]["id"] if stables else None,
            "data_version": latest.get("data_version"),
            "protocol_version": latest.get("protocol_version"),
        },
        "root_commands": sorted(commands),
        "commands": commands,
        "parsers": parsers,
        "execute_usages": execute_usages,
        "tree": tree,
    }


def bedrock_extract() -> dict:
    cmd_dir = MC_SRC / "msdocs/creator/Commands/commands"
    wiki_dir = MC_SRC / "wiki"
    commands = {}
    for md_path in sorted(cmd_dir.glob("*.md")):
        name = md_path.stem
        text = md_path.read_text(encoding="utf-8", errors="replace")
        title = None
        m = re.search(r"^title:\s*(.+)$", text, re.M)
        if m:
            title = m.group(1).strip()
        perm = None
        cheats = None
        pm = re.search(r"<th>Permission Level</th>\s*<td>(.*?)</td>", text, re.S)
        if pm:
            perm = re.sub(r"\s+", " ", pm.group(1)).strip()
        cm = re.search(r"<th>Requires Cheats\?</th>\s*<td>(.*?)</td>", text, re.S)
        if cm:
            cheats = re.sub(r"\s+", " ", cm.group(1)).strip()
        usages = extract_ms_syntax(text)
        wiki_file = wiki_dir / f"Commands_{name}.wiki"
        wiki_syntax = {"java": [], "bedrock": []}
        if wiki_file.exists():
            wiki_syntax = extract_wiki_syntax(wiki_file.read_text(encoding="utf-8", errors="replace"))

        def keep_usage(s: str, cmd: str) -> bool:
            if "[[" in s or "'''" in s:
                return False
            compact = s.lstrip("/")
            return compact.startswith(cmd) or s.startswith("/")

        wiki_syntax["bedrock"] = [s for s in wiki_syntax["bedrock"] if keep_usage(s, name)]
        wiki_syntax["java"] = [s for s in wiki_syntax["java"] if keep_usage(s, name)]
        commands[name] = {
            "name": name,
            "title": title,
            "permission": perm,
            "requires_cheats": cheats,
            "official_usages": usages,
            "wiki_bedrock": wiki_syntax["bedrock"],
            "wiki_java": wiki_syntax["java"],
        }
    return {
        "source": {
            "kind": "mojang-creator-docs",
            "via": "https://github.com/MicrosoftDocs/minecraft-creator/tree/main/creator/Commands/commands",
            "generator": "@minecraft/api-docs-generator",
            "learn": "https://learn.microsoft.com/minecraft/creator/commands/commands",
        },
        "root_commands": sorted(commands),
        "commands": commands,
    }


def wiki_cross_index() -> dict:
    wiki_dir = MC_SRC / "wiki"
    out = {}
    for path in sorted(wiki_dir.glob("Commands_*.wiki")):
        name = path.stem.removeprefix("Commands_")
        syntax = extract_wiki_syntax(path.read_text(encoding="utf-8", errors="replace"))
        if syntax["java"] or syntax["bedrock"]:
            out[name] = syntax
    return out


def write_java_usages_md(java: dict) -> str:
    lines = [
        "# Java Edition command usages (generated)",
        "",
        f"Game: **{java['source']['game_name']}** (`{java['source']['game_id']}`).",
        f"Tree: vanilla brigadier dump via [misode/mcmeta summary]({java['source']['via']}).",
        f"Latest stable at extraction time: `{java['source']['latest_stable']}`.",
        "",
        "Redirect `-> execute` means the `/execute` chain continues (another subcommand is required).",
        "This file is generated; do not edit by hand.",
        "",
        f"Root commands: **{len(java['root_commands'])}**.",
        "",
    ]
    for name in java["root_commands"]:
        cmd = java["commands"][name]
        lines.append(f"## `{name}`")
        lines.append("")
        for u in cmd["usages"]:
            lines.append(f"- `{u['usage']}`")
        lines.append("")
    return "\n".join(lines) + "\n"


def write_bedrock_usages_md(be: dict) -> str:
    lines = [
        "# Bedrock Edition command usages (generated)",
        "",
        "Official signatures from Mojang's `@minecraft/api-docs-generator` markdown",
        f"([MicrosoftDocs/minecraft-creator]({be['source']['via']})),",
        "cross-checked against Minecraft Wiki Syntax sections where a page exists.",
        "",
        "This file is generated; do not edit by hand.",
        "",
        f"Root commands: **{len(be['root_commands'])}**.",
        "",
    ]
    for name in be["root_commands"]:
        cmd = be["commands"][name]
        lines.append(f"## `{name}`")
        lines.append("")
        if cmd.get("permission"):
            lines.append(f"- Permission: {cmd['permission']}")
        if cmd.get("requires_cheats"):
            lines.append(f"- Requires cheats: {cmd['requires_cheats']}")
        lines.append("")
        lines.append("### Official")
        lines.append("")
        for u in cmd["official_usages"]:
            lines.append(f"- `{u}`")
        if cmd["wiki_bedrock"]:
            lines.append("")
            lines.append("### Wiki (Bedrock Syntax section)")
            lines.append("")
            for u in cmd["wiki_bedrock"]:
                lines.append(f"- `{u}`")
        lines.append("")
    return "\n".join(lines) + "\n"


def write_matrix_md(java: dict, be: dict, wiki: dict) -> str:
    je = set(java["root_commands"])
    be_cmds = set(be["root_commands"])
    # aliases
    aliases = {
        "xp": "experience",
        "w": "msg",
        "tm": "teammsg",
        "tp": "teleport",
        "tell": "msg",
        "wb": "worldbuilder",
        "daylock": "alwaysday",
    }
    all_names = sorted(je | be_cmds | set(wiki))
    lines = [
        "# Java vs Bedrock command presence matrix",
        "",
        "Generated from the Java brigadier tree and official Bedrock Creator command pages.",
        "Wiki Syntax sections are used as a third check when available.",
        "",
        "| Command | Java tree | Bedrock official | Wiki JE syntax | Wiki BE syntax |",
        "| --- | --- | --- | --- | --- |",
    ]
    for name in all_names:
        j = "yes" if name in je else ""
        b = "yes" if name in be_cmds else ""
        wj = "yes" if wiki.get(name, {}).get("java") else ""
        wb = "yes" if wiki.get(name, {}).get("bedrock") else ""
        alias = aliases.get(name)
        note = f" (alias of `{alias}`)" if alias and alias in je else ""
        lines.append(f"| `{name}`{note} | {j} | {b} | {wj} | {wb} |")
    lines.append("")
    only_je = sorted(je - be_cmds)
    only_be = sorted(be_cmds - je)
    lines.append(f"Java-only ({len(only_je)}): " + ", ".join(f"`{c}`" for c in only_je))
    lines.append("")
    lines.append(f"Bedrock-only ({len(only_be)}): " + ", ".join(f"`{c}`" for c in only_be))
    lines.append("")
    return "\n".join(lines) + "\n"


def main() -> None:
    java = java_extract()
    be = bedrock_extract()
    wiki = wiki_cross_index()

    java_dir = OUT / "java/data"
    be_dir = OUT / "bedrock/data"
    java_dir.mkdir(parents=True, exist_ok=True)
    be_dir.mkdir(parents=True, exist_ok=True)

    tree = java.pop("tree")
    (java_dir / "brigadier-tree.json").write_text(json.dumps(tree, indent=2) + "\n")
    (java_dir / "command-index.json").write_text(
        json.dumps(
            {
                "source": java["source"],
                "root_commands": java["root_commands"],
                "parsers": java["parsers"],
                "commands": {
                    k: {
                        "permissions": v["permissions"],
                        "usages": [u["usage"] for u in v["usages"]],
                    }
                    for k, v in java["commands"].items()
                },
            },
            indent=2,
        )
        + "\n"
    )
    (java_dir / "execute-usages.json").write_text(
        json.dumps({"source": java["source"], "usages": java["execute_usages"]}, indent=2)
        + "\n"
    )
    (OUT / "java/commands/generated-usages.md").write_text(write_java_usages_md(java))

    (be_dir / "command-index.json").write_text(
        json.dumps(
            {
                "source": be["source"],
                "root_commands": be["root_commands"],
                "commands": be["commands"],
            },
            indent=2,
        )
        + "\n"
    )
    (OUT / "bedrock/commands/generated-syntax.md").write_text(write_bedrock_usages_md(be))
    (OUT / "crosswalk/command-matrix.md").write_text(write_matrix_md(java, be, wiki))
    (OUT / "crosswalk/wiki-syntax.json").write_text(json.dumps(wiki, indent=2) + "\n")

    print("java commands", len(java["root_commands"]))
    print("bedrock commands", len(be["root_commands"]))
    print("wiki syntax pages", len(wiki))
    print("java parsers", len(java["parsers"]))


if __name__ == "__main__":
    main()
