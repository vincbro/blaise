---
name: review
description: Review a change, a branch, files or the whole codebase against the code standards in code-standards/. Routes each changed file to its language's REVIEW.md (style and performance), runs the project's gate, merges the findings into one numbered list and asks which to implement. Use when asked to review, before committing, and from the commit skill. It never edits code.
---

# review

You run the whole review of a change: the project's gate, then one review per language touched, as one report with one question. You hold no language rules yourself. Each language's `REVIEW.md` does.

**This skill never edits code.** It reads, runs checks, reports, and asks. Implementing the accepted fixes is separate work, done by whoever called you (the `commit` skill) or by the person, afterwards.

## 0. Check for updates

Run `sh code-standards/scripts/check-update.sh`. `current` and `offline` mean say nothing and go on. An `update …` line becomes a one-line notice at the top of the report (how many commits behind, and whether the skills changed), and "update standards" is offered in the question in step 7. Never apply it yourself. `code-standards/UPDATE.md` says how.

## 1. Project facts

Read `STANDARDS.local.md` at the repo root if it exists. It names the project's gate command, its commit-message check, its hot paths and its overrides. Where it disagrees with `code-standards/`, it wins. If it doesn't exist, say so in the report and go on with the standards as written.

## 2. Scope

Resolve the argument as `code-standards/skills/review/scope.md` says: nothing is the working tree, `abc123` a commit, `a..b` a range, `..` or `all` the whole codebase, optionally limited by paths. State the resolved scope at the top of the report. If it is empty, say so and stop.

- **Called from `commit`:** the working tree.
- **A re-review:** only the files the caller names.

## 3. Route

List the files in scope and send each to its language:

| Files | Language review |
|---|---|
| `*.rs`, `Cargo.toml`, `Cargo.lock` | `code-standards/rust/REVIEW.md` |
| `*.ts`, `*.tsx`, `package.json`, `tsconfig.json`, `eslint.config.*` | `code-standards/typescript/REVIEW.md` |

A language's `REVIEW.md` is the whole procedure for it: which rules, how to check them, what to report. For each language that has files in scope, read its `REVIEW.md` and follow it on those files only, with the scope from step 2 and the instruction to **return findings and its rule checklist without asking** the question at its end. If the harness can run a skill or subagent per language, use it, otherwise follow the file yourself.

Files in a language with no row above go under "Not covered", as a question for the person. Never review them against another language's rules.

For a commit or a range, also review the commit messages against `code-standards/COMMITS.md` (see `scope.md`, "Commit messages").

## 4. The mechanical gate

Run the gate command from `STANDARDS.local.md` when `scope.md` says it describes the scope (see its "Running tools"), and otherwise skip it and say so. A failure is the first finding: one line per failing step, without repeating what the tools print. A change that doesn't build or pass its tests can't be committed, so say so plainly. If you can't run commands, say that and go on.

If a failure is not caused by the change (it fails the same way on a clean checkout), say so and don't count it against the change.

## 5. Merge

Combine the findings from every language into one list:

- Merge duplicates: when two reviews flag the same line for related reasons, make one finding that cites both rule ids.
- Order by how much they matter: build and test failures first, then correctness-adjacent ones (panics, errors), then hot-path performance, then flatness, naming and the rest, comments last.
- Number them `1.`, `2.`, … across all languages so the person can pick by number.

## 6. Report

Start with a one-line verdict: how many findings, and whether the change is in line with the standards. Then the numbered findings, each as:

```
src/view/layout.rs:251  S-NAMES-HONEST
  `row_ptr` isn't a pointer, it's the row being written.
  → rename to `row_cursor`
```

Then: each language's rule checklist (every rule id with its result), the hot paths the change touches, the tools' results, and "Not covered".

## 7. Ask

End by asking what to implement: **all**, **some** by number, or **none**. If step 0 found an update, add **update standards** as a separate option: it follows `code-standards/UPDATE.md` and then re-reviews, since the rules may have changed. Use the question tool if the harness has one, then stop and wait. Don't start on any fix in the same turn.
