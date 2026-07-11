<!--
PR template (Forgejo). Same contract as .github/pull_request_template.md.
Agents and humans must fill every section. Delete sections that genuinely
do not apply, but explain why.
-->

## Summary

<!-- 1–3 sentences. What changed and why. Link the issue if there is one. -->

## Scope

<!-- Bullet list of files/areas touched. Flag anything outside the obvious scope. -->

## Risk

<!-- low / medium / high — and a one-line reason. Note any sensitive paths touched. -->

## Test plan

<!--
How you verified this works. Include exact commands and expected output.
For UI changes, attach screenshots or a short clip.
-->

- [ ] Lint passes locally
- [ ] Tests pass locally
- [ ] Build / typecheck passes locally
- [ ] Manual smoke test (describe)

## Rollback

<!-- One sentence: how to revert if this breaks production. -->

## Notes for reviewer

<!-- Anything non-obvious, trade-offs considered, follow-ups deferred. -->

## Review path

<!--
This repo uses the WECO review topology (see templates/docs/weco-mapping.md).
- Path-based reviewers will be auto-requested via CODEOWNERS.
- Final-approval gate: vimes OR vetinari (one official approval merges).
- Direct push to the default branch is allowed only for leo.
Tick the agent that opened this PR (or the human if leo).
-->

- [ ] leo (terminal) - [ ] quirm - [ ] sancho - [ ] vetinari
- [ ] vimes - [ ] frick - [ ] frack - [ ] puck

---

<sub>Generated/edited by an agent? Tag with the `agent` label and include the agent name + run id in the notes section.</sub>
