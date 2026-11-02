#!/usr/bin/env python3
"""Renders the AUR PKGBUILD and .SRCINFO of remmods-bin from a release .deb (SPEC-P5 §7.2, §7.3).

Usage: python3 -I scripts/render-pkgbuild.py --deb F --version X.Y.Z --out DIR (--local | --release)
       python3 -I scripts/render-pkgbuild.py --self-test

Checks that the deb is named RemMods_<version>_amd64.deb, computes its sha256 and renders
packaging/aur/PKGBUILD.in (@PKGVER@, @SOURCE@, @SHA256@) into DIR/PKGBUILD, plus DIR/.SRCINFO in
`makepkg --printsrcinfo` order. --release points the source at the GitHub release download;
--local copies the deb into DIR and uses its bare file name (for a local `makepkg`). Stdlib only;
writes only into DIR.
"""

import argparse
import hashlib
import re
import shutil
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
TEMPLATE = REPO / "packaging" / "aur" / "PKGBUILD.in"
VERSION_RE = re.compile(r"(0|[1-9]\d{0,8})\.(0|[1-9]\d{0,8})\.(0|[1-9]\d{0,8})", re.ASCII)
PLACEHOLDER_RE = re.compile(r"@[A-Z0-9_]+@")
RELEASE_SOURCE = (
    '"RemMods_${pkgver}_amd64.deb::'
    'https://github.com/Remirudev/RemMods/releases/download/v${pkgver}/RemMods_${pkgver}_amd64.deb"'
)
LOCAL_SOURCE = '"RemMods_${pkgver}_amd64.deb"'

# makepkg's srcinfo.sh field order (pacman 7): pkgbase section, then the pkgname section.
GLOBAL_SINGLE = ("pkgdesc", "pkgver", "pkgrel", "epoch", "url", "install", "changelog")
GLOBAL_MULTI = (
    "arch", "groups", "license", "checkdepends", "makedepends", "depends", "optdepends", "provides",
    "conflicts", "replaces", "noextract", "options", "backup", "source", "validpgpkeys",
    "cksums", "md5sums", "sha1sums", "sha224sums", "sha256sums", "sha384sums", "sha512sums", "b2sums",
)
ARCH_MULTI = (
    "source", "provides", "conflicts", "depends", "replaces", "optdepends", "makedepends",
    "checkdepends", "cksums", "md5sums", "sha1sums", "sha224sums", "sha256sums", "sha384sums",
    "sha512sums", "b2sums",
)


class RenderError(Exception):
    pass


def deb_name(version):
    return f"RemMods_{version}_amd64.deb"


def check_inputs(deb, version):
    if not VERSION_RE.fullmatch(version):
        raise RenderError(f"version {version!r} is not X.Y.Z")
    if deb.name != deb_name(version):
        raise RenderError(f"deb is named {deb.name!r}, expected {deb_name(version)!r}")
    if not deb.is_file():
        raise RenderError(f"no such file: {deb}")


def sha256_of(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def render_pkgbuild(template, version, sha256, local):
    text = (
        template.replace("@PKGVER@", version)
        .replace("@SOURCE@", LOCAL_SOURCE if local else RELEASE_SOURCE)
        .replace("@SHA256@", sha256)
    )
    left = PLACEHOLDER_RE.findall(text)
    if left:
        raise RenderError(f"template placeholders left: {', '.join(sorted(set(left)))}")
    return text


def shell_words(value, variables):
    """Splits the right-hand side of a PKGBUILD assignment (quoted words only, ${var} expanded)."""
    words = []
    for match in re.finditer(r"'([^']*)'|\"([^\"]*)\"|([^\s'\"]+)", value):
        single, double, bare = match.groups()
        if single is not None:
            words.append(single)
        else:
            word = double if double is not None else bare
            words.append(re.sub(r"\$\{(\w+)\}|\$(\w+)", lambda m: variables[m.group(1) or m.group(2)], word))
    return words


def parse_assignments(pkgbuild):
    """Top-level `name=value` / `name=(...)` lines of the rendered template (one line each)."""
    variables, fields = {}, {}
    for line in pkgbuild.splitlines():
        match = re.fullmatch(r"([a-z0-9_]+)=(\(.*\)|.*)", line)
        if not match:
            continue
        name, value = match.groups()
        is_array = value.startswith("(")
        words = shell_words(value[1:-1] if is_array else value, variables)
        if not is_array:
            if len(words) != 1:
                raise RenderError(f"{name}: expected one word, got {words!r}")
            variables[name] = words[0]
        fields[name] = words
    return fields


def render_srcinfo(pkgbuild):
    fields = parse_assignments(pkgbuild)
    pkgname = fields["pkgname"][0]
    lines = [f"pkgbase = {pkgname}"]
    for name in GLOBAL_SINGLE + GLOBAL_MULTI:
        lines += [f"\t{name} = {value}" for value in fields.get(name, [])]
    for arch in fields.get("arch", []):
        if arch == "any":
            continue
        for name in ARCH_MULTI:
            lines += [f"\t{name}_{arch} = {value}" for value in fields.get(f"{name}_{arch}", [])]
    lines += ["", f"pkgname = {pkgname}"]
    return "\n".join(lines) + "\n"


def render(deb, version, out, local, template_text=None):
    check_inputs(deb, version)
    template = template_text if template_text is not None else TEMPLATE.read_text(encoding="utf-8")
    sha256 = sha256_of(deb)
    pkgbuild = render_pkgbuild(template, version, sha256, local)
    srcinfo = render_srcinfo(pkgbuild)
    out.mkdir(parents=True, exist_ok=True)
    if local and deb.resolve() != (out / deb.name).resolve():
        shutil.copyfile(deb, out / deb.name)
    (out / "PKGBUILD").write_bytes(pkgbuild.encode("utf-8"))
    (out / ".SRCINFO").write_bytes(srcinfo.encode("utf-8"))
    return pkgbuild, srcinfo, sha256


SELF_TEST_DEB = b"!<arch>\nremmods render-pkgbuild self-test\n"

GOLDEN_SRCINFO = """pkgbase = remmods-bin
\tpkgdesc = Mod manager for TEKKEN 8
\tpkgver = 1.2.3
\tpkgrel = 1
\turl = https://github.com/Remirudev/RemMods
\tarch = x86_64
\tlicense = LicenseRef-proprietary
\tdepends = glibc
\tdepends = libgcc
\tdepends = webkit2gtk-4.1
\tdepends = gtk3
\tdepends = libsoup3
\tdepends = openssl
\tdepends = glib2
\tdepends = cairo
\tdepends = gdk-pixbuf2
\tdepends = dbus
\tdepends = wayland
\tdepends = hicolor-icon-theme
\tdepends = xdg-utils
\tprovides = remmods
\tconflicts = remmods
\toptions = !strip
\toptions = !debug
\tsource = {source}
\tsha256sums = {sha}

pkgname = remmods-bin
"""


def golden_pkgbuild(source, sha):
    return f"""# Maintainer: remirudev
# Rendered by scripts/render-pkgbuild.py from packaging/aur/PKGBUILD.in; edit the template.
pkgname=remmods-bin
pkgver=1.2.3
pkgrel=1
pkgdesc='Mod manager for TEKKEN 8'
arch=('x86_64')
url='https://github.com/Remirudev/RemMods'
license=('LicenseRef-proprietary')
depends=('glibc' 'libgcc' 'webkit2gtk-4.1' 'gtk3' 'libsoup3' 'openssl' 'glib2' 'cairo' 'gdk-pixbuf2' 'dbus' 'wayland' 'hicolor-icon-theme' 'xdg-utils')
provides=('remmods')
conflicts=('remmods')
options=('!strip' '!debug')
source=({source})
sha256sums=('{sha}')

package() {{
  bsdtar -xf data.tar.gz -C "$pkgdir"
  install -Dm644 "$pkgdir/usr/share/doc/remmods/THIRD-PARTY-NOTICES" "$pkgdir/usr/share/licenses/$pkgname/THIRD-PARTY-NOTICES"
  install -Dm644 /dev/stdin "$pkgdir/usr/lib/remmods/install-source" <<< 'install-source=aur'
}}
"""


def expect_refusal(label, action, needle):
    try:
        action()
    except RenderError as error:
        if needle not in str(error):
            raise AssertionError(f"{label}: refused with {error!r}, expected {needle!r}") from None
        return
    raise AssertionError(f"{label}: not refused")


def self_test():
    sha = hashlib.sha256(SELF_TEST_DEB).hexdigest()
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        deb = root / "RemMods_1.2.3_amd64.deb"
        deb.write_bytes(SELF_TEST_DEB)

        release_dir = root / "release"
        pkgbuild, srcinfo, got_sha = render(deb, "1.2.3", release_dir, local=False)
        assert got_sha == sha, "sha256"
        assert pkgbuild == golden_pkgbuild(RELEASE_SOURCE, sha), "release PKGBUILD differs from the golden text"
        release_url = (
            "RemMods_1.2.3_amd64.deb::"
            "https://github.com/Remirudev/RemMods/releases/download/v1.2.3/RemMods_1.2.3_amd64.deb"
        )
        assert srcinfo == GOLDEN_SRCINFO.format(source=release_url, sha=sha), "release .SRCINFO differs"
        assert not (release_dir / deb.name).exists(), "--release must not copy the deb"

        local_dir = root / "local"
        pkgbuild, srcinfo, _ = render(deb, "1.2.3", local_dir, local=True)
        assert pkgbuild == golden_pkgbuild(LOCAL_SOURCE, sha), "local PKGBUILD differs from the golden text"
        assert srcinfo == GOLDEN_SRCINFO.format(source="RemMods_1.2.3_amd64.deb", sha=sha), "local .SRCINFO differs"
        assert (local_dir / deb.name).read_bytes() == SELF_TEST_DEB, "--local copies the deb"
        assert (local_dir / ".SRCINFO").read_bytes() == srcinfo.encode(), ".SRCINFO written"

        wrong = root / "remmods_1.2.3_amd64.deb"
        wrong.write_bytes(SELF_TEST_DEB)
        expect_refusal("wrong deb name", lambda: render(wrong, "1.2.3", root / "x", local=False), "expected")
        expect_refusal("version mismatch", lambda: render(deb, "1.2.4", root / "x", local=False), "expected")
        expect_refusal("bad version", lambda: render(deb, "v1.2.3", root / "x", local=False), "not X.Y.Z")
        missing = root / "missing" / "RemMods_1.2.3_amd64.deb"
        expect_refusal("missing file", lambda: render(missing, "1.2.3", root / "x", local=False), "no such file")
        expect_refusal(
            "placeholder left",
            lambda: render(deb, "1.2.3", root / "x", local=False, template_text="pkgver=@PKGVER@\nx=@OTHER@\n"),
            "@OTHER@",
        )
        assert not (root / "x").exists(), "a refused render writes nothing"
    print("render-pkgbuild: self-test passed")
    return 0


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--deb", type=Path)
    parser.add_argument("--version")
    parser.add_argument("--out", type=Path)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--local", action="store_true")
    mode.add_argument("--release", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return self_test()
    if not (args.deb and args.version and args.out and (args.local or args.release)):
        parser.error("--deb, --version, --out and one of --local/--release are required")
    try:
        _, _, sha256 = render(args.deb, args.version, args.out, local=args.local)
    except (RenderError, OSError) as error:
        print(f"render-pkgbuild: {error}", file=sys.stderr)
        return 1
    print(f"render-pkgbuild: wrote {args.out / 'PKGBUILD'} and .SRCINFO (sha256 {sha256})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
