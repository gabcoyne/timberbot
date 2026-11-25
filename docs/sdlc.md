# Timberbot SDLC

Applies to `gabcoyne/timberbot`. Pull requests #1–#17 are on `main`. New work branches from fresh `main`.

Role checklists for workers: `.cursor/rules/triage-issue.mdc`, `.cursor/rules/review.mdc`, `.cursor/rules/qa-evals.mdc`.

## Roles

Each role is a separate worker. One worker does not implement, review, and sign off on its own change.

1. **Triage.** Read the GitHub issue. Classify it as C# mod, Rust client or MCP, Layer 1 state, Layer 2 tactics, Layer 3 evals, Layer 4 outer loop, skills, or docs. Comment on the issue with the diagnosis and the plan, then dispatch a builder.
2. **Builder.** Branch from fresh `main`. One small pull request targeting `main`. Add a regression test, or an offline eval or staged-drill case, for the reported failure. Open the pull request with `Closes #<issue>`.
3. **Reviewer.** A different worker. Check correctness, Rust safety (no unguarded panics or unwraps on live paths), GPL-3.0, no secrets, no Timberborn or Unity DLLs, and docs and skills consistency. Approve or request changes in the pull request review.
4. **QA.** A different worker. Run `cargo test`, the offline evals, the staged drills (0 deaths, 0 contamination), and `python3 scripts/lint_skills.py`. Confirm GitHub Actions is green on the pull request.
5. **Release flag.** If the diff touches `timberbot/`, the builder says so in the pull request and the triage comment says the gaming machine must rebuild and upload `timberbot-mod-<version>.zip` with `scripts/package-mod.ps1` or `scripts/package-mod.sh`.

## Gates before merge

- Reviewer has approved.
- QA reports tests, evals, drills, skill lint, and CI green.
- A diff that touches `timberbot/` names the `timberbot-mod-<version>.zip` rebuild (`scripts/package-mod.ps1` or `scripts/package-mod.sh`).
- The pull request targets `main` and does not stack on an unmerged branch.

## Issue watch

This Project has no GitHub issue webhook. A 10-minute check lists new, reopened, and newly commented issues on `gabcoyne/timberbot`. Pull request and CI watches stay on the open pull requests. Triage starts when that check finds an issue, or when George brings one here.
