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
| WG-1 | Restart Manager holder names | Folder-busy and in-use messages name no holder (PLAN §4 Retry) | no | open |
| WG-2 | Folder-busy wait | The wait (up to 30 s) shows only the pending toggle; no toast with Cancel (PLAN §4 Retry) | yes | open |
| WG-3 | Watcher overflow rescan | notify 8.2.1 emits no rescan on a ReadDirectoryChangesW overflow; focus and post-command rescans cover it until notify 9 (PLAN §4 Watcher) | no | open |
| WG-4 | Toggle while TEKKEN 8 runs | The DELETE-access probe (InUse without a prompt) is verified in CI only, not on a real PC (PLAN §11) | no | open |
| WG-5 | Recycle Bin mover | Trash always reports `TrashUnavailable(NotSupported)`, so removal on Windows is "Delete permanently" (second confirmation) or Cancel; PLAN §4's `IFileOperation` mover arrives in P3b | yes (for 1.0) | open |
| WG-6 | Profile export path | The Windows save dialog's overwrite prompt is assumed; it is verified on Linux/portal only | no | open |
| WG-7 | Explorer drag and drop, IFileDialog multi-select | The archive drop path and the multi-file picker are CI-compiled only; verified on Linux (Plasma portal, Dolphin) | no | open |
| WG-8 | Graphics fallback | Nothing by design: WebView2 has no WebKitGTK renderer variables, so Settings → Graphics and the safe-graphics start are Linux-only; `platform_info().features.graphicsFallback` is false, `graphics_get` reports `supported: false` and `graphics_set_mode`/`graphics_ack_step` return `unsupported` (PLAN §1 table: "none") | no | closed (P5, by design) |
| WG-9 | Self-update (NSIS passive) | CI installs the NSIS setup silently and smoke-tests it, but never runs an update from one release to the next; the first real passive update is a Windows checklist step | no | open |
| WG-10 | Code signing | The installer is unsigned: SmartScreen warns ("More info", then "Run anyway") and Smart App Control blocks it with no per-app exception; each release goes to Microsoft's file submission; signing waits for SignPath (PLAN §10) | no | open |
