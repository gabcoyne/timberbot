#!/usr/bin/env python3
"""Lint Timberborn Grok Bot playbooks under skills/.

Checks:
1. Every skills/timberborn-*/SKILL.md has valid YAML frontmatter with
   `name` matching its directory slug and a non-empty `description`.
2. Every skill folder is indexed in skills/README.md, and every slug listed in
   skills/README.md exists on disk.
3. Every cross-reference to a `timberborn-*` slug points to an existing skill
   directory.
4. No broken relative markdown links inside skills/.
5. No skill falsely claims that the C# mod exposes a `POST /mcp` endpoint
   (explicit corrections/negations like "does *not* expose `POST /mcp`" are
   allowed).
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SKILLS_DIR = REPO_ROOT / "skills"

SKILL_SLUG_RE = re.compile(r"\btimberborn-[a-z0-9-]+\b")
MD_LINK_RE = re.compile(r"\[[^\]]*\]\(([^)]+)\)")
# Known non-skill timberborn-* identifiers (repo names, tool names, etc.).
NON_SKILL_SLUGS = {
    "timberborn-tools",
    "timberborn-modding",
    "timberborn-http",
}


def parse_frontmatter(path: Path) -> tuple[dict[str, str], str]:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        raise ValueError("missing opening '---' YAML frontmatter")
    end = text.find("\n---\n", 4)
    if end == -1:
        raise ValueError("missing closing '---' YAML frontmatter")
    raw_fm = text[4:end]
    body = text[end + 5 :]
    meta: dict[str, str] = {}
    for line in raw_fm.splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if ":" not in line:
            raise ValueError(f"invalid frontmatter line: {line!r}")
        key, value = line.split(":", 1)
        key = key.strip()
        value = value.strip()
        if (value.startswith('"') and value.endswith('"')) or (
            value.startswith("'") and value.endswith("'")
        ):
            value = value[1:-1]
        meta[key] = value
    return meta, body


def lint() -> list[str]:
    errors: list[str] = []

    if not SKILLS_DIR.is_dir():
        return [f"skills directory not found: {SKILLS_DIR}"]

    skill_dirs = sorted(
        d for d in SKILLS_DIR.iterdir() if d.is_dir() and d.name.startswith("timberborn-")
    )
    if not skill_dirs:
        return ["no skills/timberborn-* directories found"]

    known_slugs = {d.name for d in skill_dirs}

    # 1. Frontmatter check on every SKILL.md
    for d in skill_dirs:
        skill_md = d / "SKILL.md"
        if not skill_md.is_file():
            errors.append(f"{d.relative_to(REPO_ROOT)}: missing SKILL.md")
            continue
        try:
            meta, _body = parse_frontmatter(skill_md)
        except ValueError as exc:
            errors.append(f"{skill_md.relative_to(REPO_ROOT)}: {exc}")
            continue
        if meta.get("name") != d.name:
            errors.append(
                f"{skill_md.relative_to(REPO_ROOT)}: frontmatter name "
                f"{meta.get('name')!r} != directory {d.name!r}"
            )
        desc = meta.get("description", "").strip()
        if len(desc) < 20:
            errors.append(
                f"{skill_md.relative_to(REPO_ROOT)}: frontmatter description is too short or empty"
            )

    # 2. Index check against skills/README.md
    readme = SKILLS_DIR / "README.md"
    if not readme.is_file():
        errors.append("skills/README.md is missing")
    else:
        readme_text = readme.read_text(encoding="utf-8")
        indexed_slugs = {
            m.group(0)
            for m in SKILL_SLUG_RE.finditer(readme_text)
            if m.group(0) not in NON_SKILL_SLUGS
        }
        missing_from_readme = sorted(known_slugs - indexed_slugs)
        extra_in_readme = sorted(indexed_slugs - known_slugs)
        for slug in missing_from_readme:
            errors.append(f"skills/README.md: missing index entry for skill {slug!r}")
        for slug in extra_in_readme:
            errors.append(f"skills/README.md: references unknown skill {slug!r}")

    # 3, 4, 5. Cross-references, relative links, and POST /mcp guard across all skills/*.md
    all_md = sorted(SKILLS_DIR.rglob("*.md"))
    for md in all_md:
        rel = md.relative_to(REPO_ROOT)
        text = md.read_text(encoding="utf-8")

        # Cross-references (skip URLs like github.com/0xguy07/timberborn-tools)
        for lineno, line in enumerate(text.splitlines(), start=1):
            stripped_urls = re.sub(r"https?://\S+", "", line)
            for match in SKILL_SLUG_RE.finditer(stripped_urls):
                slug = match.group(0)
                if slug not in known_slugs and slug not in NON_SKILL_SLUGS:
                    errors.append(f"{rel}:{lineno}: unknown skill slug {slug!r}")

            # Guard against claiming POST /mcp exists on the mod
            if re.search(r"POST\s+/mcp\b", line, re.IGNORECASE):
                lower = re.sub(r"[*_`]", "", line).lower()
                negated = bool(
                    re.search(r"\b(not|no|never|correction)\b", lower)
                )
                if not negated:
                    errors.append(
                        f"{rel}:{lineno}: mentions 'POST /mcp' without negating/correcting it"
                    )

        # Relative markdown links
        for match in MD_LINK_RE.finditer(text):
            target = match.group(1).strip()
            if not target or target.startswith(("http://", "https://", "mailto:", "#")):
                continue
            target_path = target.split("#", 1)[0]
            if not target_path:
                continue
            resolved = (md.parent / target_path).resolve()
            if not resolved.exists():
                errors.append(f"{rel}: broken relative link ({target})")

    return errors


def main() -> int:
    errors = lint()
    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        print(f"lint_skills: {len(errors)} error(s) found", file=sys.stderr)
        return 1
    skill_count = sum(
        1 for d in SKILLS_DIR.iterdir() if d.is_dir() and d.name.startswith("timberborn-")
    )
    print(f"lint_skills: OK ({skill_count} skills validated)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
