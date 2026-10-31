# Windows gaps

Linux comes first, but Windows must work at all times (docs/PLAN.md §1). A feature that lands
Linux-first still builds and passes its tests on Windows, and on Windows it either works or reports
`unsupported` through `platform_info().features`. Until its Windows half is done, it has an entry here.

- **ID**: `WG-<n>`, numbered in order and never reused. Issues are off, so PRs point here with the
  template line "Windows impact: windows-gap WG-n".
- **Release-blocking**: `yes` or `no`. A release needs zero open entries marked `yes`.
- **Status**: `open`, `in progress` or `closed (<PR or version>)`. Closed entries stay for the record.

| ID | Feature | What's missing on Windows | Release-blocking | Status |
|----|---------|---------------------------|------------------|--------|

No entries yet.
