#!/usr/bin/env python3
"""Recurring SDLC gates for gabcoyne/timberbot.

Fails when the tree tracks a proprietary DLL or a high-confidence secret,
when the root LICENSE is not GPL-3.0, or when skill lint, Rust tests, or
`mkdocs build --strict` fail.

The upstream Python package stays MIT. This script locks the root LICENSE
and rust/Cargo.toml to GPL-3.0. It does not replace independent verification:
offline evals, staged drills, and a PASS from a worker who did not write the
code are still required. See docs/sdlc.md.
"""

from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

SECRET_BASENAMES = {
    ".env",
    ".env.local",
    ".env.production",
    "credentials.json",
    "id_dsa",
    "id_ed25519",
    "id_rsa",
}

SKIP_CONTENT_SUFFIXES = {
    ".dll",
    ".exe",
    ".gif",
    ".ico",
    ".jpeg",
    ".jpg",
    ".pdf",
    ".pdb",
    ".png",
    ".webp",
    ".woff",
    ".woff2",
    ".zip",
}

def _pem_header(kind: str) -> str:
    # Split so this file does not contain a header the scanner would flag.
    return "-----BEGIN " + kind + "-----"


SECRET_PATTERNS: tuple[tuple[str, str], ...] = (
    (_pem_header("PRIVATE KEY"), "private key"),
    (_pem_header("RSA PRIVATE KEY"), "private key"),
    (_pem_header("OPENSSH PRIVATE KEY"), "private key"),
    (_pem_header("EC PRIVATE KEY"), "private key"),
    (_pem_header("DSA PRIVATE KEY"), "private key"),
)

MAX_CONTENT_BYTES = 2_000_000


def repo_root() -> Path:
    return REPO_ROOT


def tracked_files(root: Path) -> list[str]:
    result = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=root,
        check=True,
        capture_output=True,
    )
    raw = result.stdout.decode("utf-8", "surrogateescape")
    return [path for path in raw.split("\0") if path]


def dll_errors(paths: list[str]) -> list[str]:
    errors: list[str] = []
    for path in paths:
        name = path.replace("\\", "/").rsplit("/", 1)[-1]
        if name.lower().endswith(".dll"):
            errors.append(
                f"tracked DLL (Timberborn and Unity game DLLs stay untracked): {path}"
            )
    return errors


def secret_path_errors(paths: list[str]) -> list[str]:
    errors: list[str] = []
    for path in paths:
        name = path.replace("\\", "/").rsplit("/", 1)[-1]
        lower = name.lower()
        if lower in SECRET_BASENAMES or lower.endswith(".pem") or lower.endswith(".p12"):
            errors.append(f"tracked secret file: {path}")
    return errors


def _placeholder(value: str) -> bool:
    folded = value.lower()
    markers = (
        "replace",
        "example",
        "changeme",
        "placeholder",
        "your-token",
        "your_token",
        "your token",
        "<token>",
        "${",
    )
    return any(marker in folded for marker in markers)


def secret_content_errors(path: str, text: str) -> list[str]:
    errors: list[str] = []
    for needle, label in SECRET_PATTERNS:
        if needle in text:
            errors.append(f"{path}: {label}")
    errors.extend(_token_errors(path, text))
    errors.extend(_auth_token_errors(path, text))
    return errors


def _token_errors(path: str, text: str) -> list[str]:
    errors: list[str] = []
    for prefix, label in (
        ("ghp_", "GitHub token"),
        ("gho_", "GitHub token"),
        ("github_pat_", "GitHub token"),
    ):
        start = 0
        while True:
            index = text.find(prefix, start)
            if index == -1:
                break
            end = index + len(prefix)
            while end < len(text) and (text[end].isalnum() or text[end] == "_"):
                end += 1
            if end - index >= len(prefix) + 20:
                errors.append(f"{path}: {label}")
                break
            start = end
    errors.extend(_aws_key_errors(path, text))
    errors.extend(_slack_token_errors(path, text))
    return errors


def _aws_key_errors(path: str, text: str) -> list[str]:
    start = 0
    while True:
        index = text.find("AKIA", start)
        if index == -1:
            return []
        chunk = text[index : index + 20]
        if len(chunk) == 20 and all(char.isdigit() or char.isupper() for char in chunk):
            return [f"{path}: AWS access key id"]
        start = index + 4


def _slack_token_errors(path: str, text: str) -> list[str]:
    start = 0
    while True:
        index = text.find("xox", start)
        if index == -1:
            return []
        kind = text[index : index + 5]
        if kind in {"xoxb-", "xoxp-", "xoxa-", "xoxr-", "xoxs-"}:
            end = index + 5
            while end < len(text) and (text[end].isalnum() or text[end] == "-"):
                end += 1
            if end - index >= 16:
                return [f"{path}: Slack token"]
        start = index + 3


def _auth_value_is_secret(path: str, value: str) -> bool:
    """Flag shipped settings and high-entropy tokens, not test fixtures.

    `timberbot/src/settings.json` ships in the mod. Any non-empty authToken
    there is a secret. Other files use short fixture strings such as
    `shared-secret` and `bot-mcp-token`; those stay, and a long hex or mixed
    token does not.
    """
    stripped = value.strip()
    if not stripped or _placeholder(stripped):
        return False
    normalized = path.replace("\\", "/")
    if normalized == "timberbot/src/settings.json":
        return True
    if len(stripped) >= 32 and all(char in "0123456789abcdefABCDEF" for char in stripped):
        return True
    classes = (
        any(char.islower() for char in stripped),
        any(char.isupper() for char in stripped),
        any(char.isdigit() for char in stripped),
    )
    return len(stripped) >= 32 and sum(classes) >= 3 and " " not in stripped


def _auth_token_errors(path: str, text: str) -> list[str]:
    errors: list[str] = []
    for key in ('"authToken"', "auth_token"):
        start = 0
        while True:
            index = text.find(key, start)
            if index == -1:
                break
            value = _quoted_assignment(text, index + len(key))
            if value is not None and _auth_value_is_secret(path, value):
                errors.append(f"{path}: non-placeholder auth token")
                return errors
            start = index + len(key)
    return errors


def _quoted_assignment(text: str, index: int) -> str | None:
    length = len(text)
    while index < length and text[index] in " \t":
        index += 1
    if index >= length or text[index] not in ":=":
        return None
    index += 1
    while index < length and text[index] in " \t":
        index += 1
    if index >= length or text[index] not in "\"'":
        return None
    quote = text[index]
    index += 1
    end = text.find(quote, index)
    if end == -1:
        return None
    return text[index:end]


def gpl_errors(license_text: str, cargo_text: str) -> list[str]:
    errors: list[str] = []
    head = license_text[:400]
    if "GNU GENERAL PUBLIC LICENSE" not in head or "Version 3" not in head:
        errors.append("LICENSE is not the GNU General Public License version 3")
    if 'license = "GPL-3.0-or-later"' not in cargo_text:
        errors.append('rust/Cargo.toml must keep license = "GPL-3.0-or-later"')
    return errors


def static_errors(root: Path) -> list[str]:
    paths = tracked_files(root)
    errors = dll_errors(paths) + secret_path_errors(paths)
    errors.extend(
        gpl_errors(
            (root / "LICENSE").read_text(encoding="utf-8"),
            (root / "rust" / "Cargo.toml").read_text(encoding="utf-8"),
        )
    )
    for path in paths:
        suffix = Path(path).suffix.lower()
        if suffix in SKIP_CONTENT_SUFFIXES:
            continue
        file_path = root / path
        try:
            size = file_path.stat().st_size
        except OSError as exc:
            errors.append(f"{path}: {exc}")
            continue
        if size > MAX_CONTENT_BYTES:
            continue
        data = file_path.read_bytes()
        if b"\0" in data[:8192]:
            continue
        text = data.decode("utf-8", "replace")
        errors.extend(secret_content_errors(path, text))
    return errors


def run_step(name: str, args: list[str], cwd: Path) -> str | None:
    print(f"== {name}: {' '.join(args)}", flush=True)
    try:
        result = subprocess.run(args, cwd=cwd)
    except FileNotFoundError:
        return f"{name} failed ({args[0]} not found)"
    if result.returncode != 0:
        return f"{name} failed (exit {result.returncode})"
    print(f"PASS {name}")
    return None


def run_unit_tests() -> bool:
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(GateTests)
    result = unittest.TextTestRunner(verbosity=1).run(suite)
    return result.wasSuccessful()


def main() -> int:
    if not run_unit_tests():
        print("FAIL gate classifier tests", file=sys.stderr)
        return 1
    print("PASS gate classifier tests")

    errors = static_errors(REPO_ROOT)
    if errors:
        print("FAIL static gates:", file=sys.stderr)
        for error in errors:
            print(f"  {error}", file=sys.stderr)
        return 1
    print("PASS proprietary DLLs, secrets, GPL-3.0")

    steps = [
        ("skills lint", ["python3", "scripts/lint_skills.py"], REPO_ROOT),
        (
            "mkdocs build --strict",
            ["python3", "-m", "mkdocs", "build", "--strict"],
            REPO_ROOT,
        ),
        (
            "cargo test",
            ["cargo", "test", "--manifest-path", "rust/Cargo.toml", "--workspace"],
            REPO_ROOT,
        ),
    ]
    failed: list[str] = []
    for name, args, cwd in steps:
        error = run_step(name, args, cwd)
        if error:
            failed.append(error)
    if failed:
        print("FAIL " + "; ".join(failed), file=sys.stderr)
        return 1
    print("PASS sdlc gates")
    return 0


class GateTests(unittest.TestCase):
    def test_dll_paths(self) -> None:
        self.assertEqual(dll_errors(["docs/sdlc.md", "scripts/lint_skills.py"]), [])
        errors = dll_errors(
            [
                "timberbot/src/Timberborn.Automation.dll",
                "UnityEngine.CoreModule.dll",
                "dist/Timberbot.dll",
            ]
        )
        self.assertEqual(len(errors), 3)

    def test_secret_paths(self) -> None:
        self.assertEqual(secret_path_errors(["timberbot/src/settings.json"]), [])
        self.assertEqual(len(secret_path_errors([".env", "keys/dev.pem"])), 2)

    def test_auth_token_placeholders(self) -> None:
        empty = secret_content_errors("settings.json", '{ "authToken": "" }\n')
        placeholder = secret_content_errors(
            "docs/connect-and-play.md",
            '{ "authToken": "REPLACE_WITH_64_HEX_RANDOM_SECRET" }\n',
        )
        short = secret_content_errors("test.cs", 'authToken: "s3cret"\n')
        self.assertEqual(empty, [])
        self.assertEqual(placeholder, [])
        self.assertEqual(short, [])

    def test_real_secrets(self) -> None:
        token = "ghp_" + ("a" * 36)
        self.assertTrue(secret_content_errors("notes.md", f"token = {token}\n"))
        self.assertTrue(secret_content_errors("k.txt", _pem_header("RSA PRIVATE KEY") + "\n"))
        self.assertTrue(secret_content_errors("k.txt", "AKIA" + "IOSFODNN7EXAMPLE\n"))
        self.assertTrue(secret_content_errors("k.txt", "xoxb-" + "1234567890-abcdef\n"))
        committed = secret_content_errors(
            "timberbot/src/settings.json",
            '{ "authToken": "correct-horse-battery" }\n',
        )
        self.assertEqual(
            committed,
            ["timberbot/src/settings.json: non-placeholder auth token"],
        )
        self.assertEqual(
            secret_content_errors("user_config.py", 'auth_token = "shared-secret"\n'),
            [],
        )
        self.assertEqual(
            secret_content_errors("loop.rs", 'auth_token: "bot-mcp-token".to_string(),\n'),
            [],
        )
        hex_token = "ab" * 32
        self.assertTrue(
            secret_content_errors("notes.md", f'"authToken": "{hex_token}"\n')
        )

    def test_gpl_text(self) -> None:
        license_text = "GNU GENERAL PUBLIC LICENSE\nVersion 3, 29 June 2007\n"
        cargo = '[workspace.package]\nlicense = "GPL-3.0-or-later"\n'
        self.assertEqual(gpl_errors(license_text, cargo), [])
        self.assertTrue(gpl_errors("MIT License\n", cargo))
        self.assertTrue(gpl_errors(license_text, 'license = "MIT"\n'))

    def test_repo_declares_gpl(self) -> None:
        root = repo_root()
        errors = gpl_errors(
            (root / "LICENSE").read_text(encoding="utf-8"),
            (root / "rust" / "Cargo.toml").read_text(encoding="utf-8"),
        )
        self.assertEqual(errors, [])

    def test_tracked_tree_has_no_dlls_or_secret_files(self) -> None:
        paths = tracked_files(repo_root())
        self.assertEqual(dll_errors(paths), [])
        self.assertEqual(secret_path_errors(paths), [])

    def test_this_file_does_not_trip_the_secret_scan(self) -> None:
        text = Path(__file__).read_text(encoding="utf-8")
        self.assertEqual(secret_content_errors("scripts/check_sdlc_gates.py", text), [])


if __name__ == "__main__":
    sys.exit(main())
