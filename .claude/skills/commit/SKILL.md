---
name: commit
description: Review the working-tree diff against the code standards, apply the fixes the person accepts, split what is left into logical commits and write each message in the S-COMMITS format (`area: lowercase imperative summary`, then a body saying why). Use when asked to commit, write commit messages, split a diff into commits, or /commit. `/commit direct` skips the review. It asks before it fixes anything and before it stages or commits.
---

# commit

You turn the current changes into well-formed commits, and you make sure bad code doesn't get committed. The message format is **S-COMMITS** in `code-standards/COMMITS.md`. Read that rule first, and the project's commit-message check if `STANDARDS.local.md` names one, and follow them over this skill if they disagree.

The flow is: **review → report findings → accept → fix → report commit messages → commit.**

**Direct mode.** If the person typed `/commit direct` (or asked to commit directly), skip steps 2 and 3: no review, no fixes, no gate. Everything else stays: the split, the messages, the plan and the question before committing.

**Nothing is fixed, staged, committed, amended or pushed until the person has agreed.** Reading (`git status`, `git diff`, `git log`) is always fine.

## 1. Read the changes

Run `git status --short`, `git diff HEAD` (staged and unstaged) and `git log -5 --format='%h %s'`. Read untracked files that look like part of the work. If there is nothing to commit, say so and stop.

If the current branch is `main` (or the project's default branch), mention it in the plan and ask whether to commit there or branch first.

## 2. Review (skipped in direct mode)

Invoke the `review` skill on the uncommitted diff. It runs the project's gate, reviews each language touched, merges the findings into one numbered list and asks what to implement: all, some by number, or none. Let it ask; don't ask again.

If it finds nothing, say so and go on to step 4.

## 3. Fix what was accepted (skipped in direct mode)

Implement only the findings the person picked, following the standards while you do it: names, comments, nesting, error handling, tests. Don't change anything else, and don't touch changes you left out of the commits on purpose (such as work the person said to leave alone).

Then run `review` once more on **only the files the fixes touched**. Report anything still found, plainly, and let the person decide whether to commit anyway. Don't loop beyond this one pass.

Re-read `git diff HEAD`: the diff changed, and the plan is made from the new one.

## 4. Split into commits

Group the changes by *reason*, not by directory. One commit is one idea that could be reverted on its own.

- A rename or file move with no behavior change is its own commit, apart from the change that uses it.
- Formatting-only changes are their own commit, apart from the logic.
- Lockfile and manifest changes go with the commit that needs them.
- Tests go with the behavior they cover.
- Prefer whole files per commit. Only split a file across commits when its hunks really belong to different ideas. Then commit the files by path (`git commit -- <paths>` commits those files' contents and leaves other staged changes alone), or stage hunks with `git apply --cached` from a patch you write to the scratchpad directory, since `git add -p` is interactive and not available.
- If the changes are one idea, make one commit. Don't split for the sake of it.

Order the commits so each one leaves the tree building, with the foundation first.

## 5. Write the messages

Per S-COMMITS:

- First line is `area: summary`. The area is the module or topic. The summary is lowercase except for code and proper nouns, starts with an imperative verb, has no trailing period and is at most 72 characters (ideally 50).
- A blank line, then a body in prose that says **why**: what was wrong or missing and the idea of the fix. No list of files. Wrap at 72 characters. Leave the body out when the summary says it all.
- A change made for speed carries its benchmark numbers in the body (P-MEASURE).
- Add no attribution trailer unless the project or the person asks for one.

Write each message to a file in the scratchpad directory and check it with the project's commit-message check, if it has one. Fix it until the check passes.

## 6. Ask before committing

Show the plan: for each commit, in order, the message and the files (or hunks) it contains. Note anything you left out of every commit and why, such as a stray build artifact, a file that looks secret, or work the person said to keep out. Never include secrets or credentials.

Then ask the person what to do, with the question tool if the harness has one:

- **commit all** as planned,
- **change the plan**, which they describe, and you show the new plan before asking again,
- **messages only**: print the messages and stop without touching git.

Do not stage or commit anything before the answer.

## 7. Commit

After approval, for each commit in order: stage exactly its files, run `git commit` with the approved message, and check it succeeded. If a commit-msg hook rejects it, show the hook's output, fix the message, and ask again only if the fix changes the meaning.

Never use `--no-verify`, `--amend`, `git reset`, `git push` or `git add -A`. If something fails or the tree isn't what you expected, stop and report with `git status`. Finish by showing `git log --oneline` for the new commits.
