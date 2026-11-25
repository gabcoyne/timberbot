# Timberbot SDLC

Applies to `gabcoyne/timberbot`. New work branches from fresh `origin/main`.

Role checklists: `.cursor/rules/triage-issue.mdc`, `.cursor/rules/owner-pr.mdc`, `.cursor/rules/review.mdc`, `.cursor/rules/qa-evals.mdc`, `.cursor/rules/merge-pr.mdc`.

Recurring mechanical gates run from `python3 scripts/check_sdlc_gates.py`. `.github/workflows/sdlc-gates.yml` runs that script on every pull request and on every push to `main`.

## Poteto-mode

When Cursor `/poteto-mode` is installed, follow the playbooks it ships. This repository does not add poteto flags.

- `opening-a-pr`: title `type(scope): subject`, body under 40 lines, pull request open and ready for review.
- `autopilot-full`: one owner per pull request, branched off trunk, unstacked. Green CI is not a verdict. Block only for a failing run at this head, a bug a user would hit, or a doc line this pull request made wrong.
- `shipping`: a verifier who did not write the code posts `PASS`, `PASS+NOTES`, or `FAIL`. Squash-merge with `gh pr merge <pr> --squash`, then read the pull request back and confirm the merge commit is on `main`.
- `babysit`: drive the pull request to merge-ready. Babysitting does not authorize a merge and does not force-push.
- `autonomous-run`: state a checkable exit condition and continue until it holds.

Issue intake for this Project stays the 10-minute poll in [Issue watch](#issue-watch). There is no GitHub issue webhook.

## Roles

Each role is a separate worker. The owner does not post the independent verdict on their own change.

1. **Triage.** Read the GitHub issue. Classify it as C# mod, Rust client or MCP, Layer 1 state, Layer 2 tactics, Layer 3 evals, Layer 4 outer loop, skills, or docs. Comment with the diagnosis and the plan, then dispatch one owner.
2. **Owner.** One pull request off fresh `main`. Open, not draft. Base ref is `main`. Add a regression test, or an offline eval or staged-drill case, for the reported failure. Use `Closes #<issue>` when the pull request fixes an issue.
3. **Verifier.** A different worker, one who did not write the code. Prove the current head and comment `PASS`, `PASS+NOTES`, or `FAIL` with that full SHA. A green author CI run, including `sdlc-gates`, is not this verdict.
4. **Merge.** After `PASS` or `PASS+NOTES` on the current head, green CI, and a clean merge into `main`, squash-merge and confirm the merge commit is on `origin/main`.

## Independent verification

On the pull request head, from the repo root:

```bash
python3 scripts/check_sdlc_gates.py
cargo run --manifest-path rust/Cargo.toml -p timberbot-drills -- offline --cases scenarios/eval-cases.json
cargo run --manifest-path rust/Cargo.toml -p timberbot-drills -- drills
```

`scripts/check_sdlc_gates.py` is the recurring gate. It fails the tree when a proprietary DLL or a high-confidence secret is tracked, when the root `LICENSE` is not GPL-3.0 or `rust/Cargo.toml` drops `GPL-3.0-or-later`, when `python3 scripts/lint_skills.py` fails, when `cargo test --workspace` fails, or when `mkdocs build --strict` fails. The upstream Python client stays MIT; the gate does not relicense it.

Offline evals pass when that command exits 0. Staged drills pass when that command exits 0 and every drill prints `deaths=0` and `contam=0`.

Comment on the pull request:

```text
Verdict: PASS
Head: <full-sha>
```

`PASS+NOTES` records non-blocking notes. `FAIL` names the failing command, the user-facing bug, or the doc line this pull request made wrong. A new head needs a new verdict.

Block only for a failing run at this head, a bug a user would hit, or a doc line this pull request made wrong. A problem already on `main` waits for a follow-up unless this pull request copies it or newly reaches it.

## Merge

Squash-merge when the latest independent verdict is `PASS` or `PASS+NOTES` for the current head, GitHub Actions (including `sdlc-gates`) is green on that head, and the pull request is open against `main` and mergeable.

```bash
gh pr merge <number> --squash
gh pr view <number> --json state,mergedAt,baseRefName,mergeCommit
git fetch origin main
git merge-base --is-ancestor <merge-commit> origin/main
```

`state` is `MERGED`, `baseRefName` is `main`, and the merge commit is contained in `origin/main`. Never force-push `main`. Never commit secrets or Timberborn or Unity DLLs.

## Release flag

A diff that touches `timberbot/` flags a `timberbot-mod-<version>.zip` GitHub Release for the gaming machine. The owner says so in the pull request. The gaming machine builds that zip with `scripts/package-mod.sh` or `scripts/package-mod.ps1`. CI does not build the mod. The game DLLs are proprietary and stay off this repository.

## Issue watch

This Project has no GitHub issue webhook. A 10-minute check lists new, reopened, and newly commented issues on `gabcoyne/timberbot`. Pull request and CI watches stay on the open pull requests. Triage starts when that check finds an issue, or when George brings one here.
