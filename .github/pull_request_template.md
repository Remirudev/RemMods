## Summary

<!-- What changes and why. Link the plan section (docs/PLAN.md §n) if it implements one. -->

## Windows impact

<!-- Keep exactly one line. A Linux-first feature needs an entry in docs/windows-gaps.md. -->

- Windows impact: none
- Windows impact: covered by CI (`windows` and `smoke (windows)` exercise it)
- Windows impact: windows-gap WG-n

## Checks

- [ ] `npm run lint`, `npm run typecheck`, `npm test`, `npm run build`
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings`
- [ ] `cargo test --workspace --locked` (new on-disk, `safe_fs` or settings rules have tests on both OSes)
- [ ] `npm run bindings`, and `src/lib/ipc/bindings.ts` is committed if it changed
- [ ] `scripts/check-cfg.sh` and `scripts/check-native.sh`
- [ ] Linux in-game smoke test (`docs/checklists/ingame-linux.md`), if this PR ends a phase or touches mod files
