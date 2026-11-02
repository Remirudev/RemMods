RemMods @VERSION@, a mod manager for TEKKEN 8.

## Downloads

| File | For |
|---|---|
| `RemMods_@VERSION@_amd64.AppImage` | any x86_64 Linux with glibc 2.39 or newer; Settings → Updates (self-update is new and first used with the next release) |
| `RemMods_@VERSION@_amd64.deb` | Ubuntu 24.04+, Linux Mint 22+, Debian 13+ |
| `RemMods-@VERSION@-1.x86_64.rpm` | Fedora 40+ |
| `RemMods_@VERSION@_x64-setup.exe` | Windows 10 and 11 (x64), installs for the current user; Settings → Updates (self-update is new and first used with the next release) |
| AUR `remmods-bin` | Arch Linux, CachyOS, EndeavourOS, Manjaro |

**Linux** needs glibc 2.39 or newer (Ubuntu 24.04, Debian 13, Fedora 40, current Arch-based
distributions). Older releases such as Debian 12 or Ubuntu 22.04 are not supported. The deb, rpm and
AUR packages are updated through your package manager; Settings → Updates only tells you that a new
version exists.

**Windows: beta, CI-tested only.** The Windows build is tested in CI but not yet on real PCs. The
installer is unsigned, so Microsoft Defender SmartScreen shows "Windows protected your PC" (choose
"More info", then "Run anyway"), and Smart App Control, when it is on, blocks it entirely.

## Check your download

`SHA256SUMS` lists the SHA-256 of every file of this release. On Linux, in the download folder:

```sh
sha256sum --check --ignore-missing SHA256SUMS
```

On Windows (PowerShell), compare the output with the line for the installer in `SHA256SUMS`:

```powershell
Get-FileHash .\RemMods_@VERSION@_x64-setup.exe -Algorithm SHA256
```

The `.sig` files are the update signatures the app checks before it installs an update.

More in the README: [Install](https://github.com/Remirudev/RemMods/blob/v@VERSION@/README.md#install) and
[Updates](https://github.com/Remirudev/RemMods/blob/v@VERSION@/README.md#updates).
