//! The self-update allowlist, the AUR marker and version rules (SPEC-P5 §9.1 U1–U9).
#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod support;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use remmods_core::app_update::{
    AppVersion, BundleKind, InstallFacts, InstallKind, InstallPermission, InstallSource,
    MARKER_MAX_BYTES, appimage_extracted, appimage_target_ok, classify_install, install_permission,
    is_offered, marker_path, may_self_install, parse_marker, read_marker,
};

const BUNDLES: [Option<BundleKind>; 7] = [
    None,
    Some(BundleKind::AppImage),
    Some(BundleKind::Deb),
    Some(BundleKind::Rpm),
    Some(BundleKind::Nsis),
    Some(BundleKind::Msi),
    Some(BundleKind::Other),
];

const KINDS: [InstallKind; 7] = [
    InstallKind::AppImage,
    InstallKind::Nsis,
    InstallKind::Deb,
    InstallKind::Rpm,
    InstallKind::Aur,
    InstallKind::Local,
    InstallKind::Unknown,
];

const MARKERS: [Option<InstallSource>; 3] =
    [None, Some(InstallSource::Aur), Some(InstallSource::Local)];

fn expected_kind(
    bundle: Option<BundleKind>,
    marker: Option<InstallSource>,
    appimage_ok: bool,
) -> InstallKind {
    match (bundle, marker) {
        (Some(BundleKind::AppImage), _) if appimage_ok => InstallKind::AppImage,
        (Some(BundleKind::Deb), Some(InstallSource::Aur)) => InstallKind::Aur,
        (Some(BundleKind::Deb), _) => InstallKind::Deb,
        (Some(BundleKind::Rpm), _) => InstallKind::Rpm,
        (Some(BundleKind::Nsis), _) => InstallKind::Nsis,
        (Some(BundleKind::Other) | None, Some(InstallSource::Local)) => InstallKind::Local,
        _ => InstallKind::Unknown,
    }
}

fn facts(
    bundle: Option<BundleKind>,
    marker: Option<InstallSource>,
    appimage_ok: bool,
) -> InstallFacts {
    InstallFacts {
        bundle,
        marker,
        appimage_ok,
    }
}

#[test]
fn u1_classify_every_combination() {
    let mut rows = 0;
    for bundle in BUNDLES {
        for marker in MARKERS {
            for appimage_ok in [false, true] {
                assert_eq!(
                    classify_install(&facts(bundle, marker, appimage_ok)),
                    expected_kind(bundle, marker, appimage_ok),
                    "{bundle:?} {marker:?} appimage_ok={appimage_ok}"
                );
                rows += 1;
            }
        }
    }
    assert_eq!(rows, 42);
    assert_eq!(
        classify_install(&facts(Some(BundleKind::AppImage), None, false)),
        InstallKind::Unknown
    );
}

#[test]
fn u2_only_appimage_and_nsis_update_themselves() {
    let expected = [
        (InstallKind::AppImage, InstallPermission::SelfUpdate),
        (InstallKind::Nsis, InstallPermission::SelfUpdate),
        (InstallKind::Deb, InstallPermission::Notice),
        (InstallKind::Rpm, InstallPermission::Notice),
        (InstallKind::Aur, InstallPermission::Notice),
        (InstallKind::Local, InstallPermission::Notice),
        (InstallKind::Unknown, InstallPermission::Notice),
    ];
    for (kind, permission) in expected {
        assert_eq!(install_permission(kind), permission, "{kind:?}");
    }
}

#[test]
fn u3_the_aur_marker_counts_only_for_deb_bundles() {
    let aur_marker = Some(InstallSource::Aur);
    let aur = classify_install(&facts(Some(BundleKind::Deb), aur_marker, true));
    assert_eq!(aur, InstallKind::Aur);
    assert_eq!(install_permission(aur), InstallPermission::Notice);
    assert_eq!(
        classify_install(&facts(Some(BundleKind::Rpm), aur_marker, true)),
        InstallKind::Rpm
    );
    assert_eq!(
        classify_install(&facts(None, aur_marker, true)),
        InstallKind::Unknown
    );
}

#[test]
fn u3b_the_local_marker_counts_only_for_unbundled_binaries() {
    let local_marker = Some(InstallSource::Local);
    let local = classify_install(&facts(None, local_marker, true));
    assert_eq!(local, InstallKind::Local);
    assert_eq!(install_permission(local), InstallPermission::Notice);
    assert!(!may_self_install(local, None));
    assert_eq!(
        classify_install(&facts(Some(BundleKind::Other), local_marker, false)),
        InstallKind::Local
    );
    for bundle in [BundleKind::Deb, BundleKind::Rpm, BundleKind::Nsis] {
        assert_ne!(
            classify_install(&facts(Some(bundle), local_marker, true)),
            InstallKind::Local,
            "{bundle:?}"
        );
    }
    assert_eq!(
        classify_install(&facts(Some(BundleKind::AppImage), local_marker, true)),
        InstallKind::AppImage
    );
}

#[test]
fn u4_may_self_install_needs_a_matching_bundle() {
    for kind in KINDS {
        for bundle in BUNDLES {
            let expected = matches!(
                (kind, bundle),
                (InstallKind::AppImage, Some(BundleKind::AppImage))
                    | (InstallKind::Nsis, Some(BundleKind::Nsis))
            );
            assert_eq!(
                may_self_install(kind, bundle),
                expected,
                "{kind:?} {bundle:?}"
            );
        }
    }
    assert!(!may_self_install(
        InstallKind::AppImage,
        Some(BundleKind::Deb)
    ));
    assert!(!may_self_install(InstallKind::AppImage, None));
}

fn os(path: &Path) -> &OsStr {
    path.as_os_str()
}

#[test]
fn u5_appimage_target_rules() {
    let (_dir, root) = support::temp_root();
    let appimage = root.join("RemMods.AppImage");
    fs::write(&appimage, b"elf").expect("appimage");
    let mount = root.join(".mount_RemModAbC123");
    let extracted = root.join("appimage_extracted_0123456789abcdef");
    let squashfs = root.join("squashfs-root");
    for folder in [&mount, &extracted, &squashfs] {
        let bin = folder.join("usr").join("bin");
        fs::create_dir_all(&bin).expect("bin dir");
        fs::write(bin.join("remmods"), b"elf").expect("inner exe");
    }
    let exe_in = |folder: &Path| folder.join("usr").join("bin").join("remmods");
    let mount_exe = exe_in(&mount);
    let real_file = |path: &Path| path.is_file();
    let ok = |appimage: Option<&OsStr>, appdir: Option<&OsStr>, exe: Option<&Path>| {
        appimage_target_ok(appimage, appdir, exe, real_file)
    };
    let good_appimage = Some(os(&appimage));

    // $APPIMAGE itself.
    assert!(!ok(None, Some(os(&mount)), Some(&mount_exe)));
    assert!(!ok(
        Some(OsStr::new("")),
        Some(os(&mount)),
        Some(&mount_exe)
    ));
    assert!(!appimage_target_ok(
        Some(OsStr::new("RemMods.AppImage")),
        Some(os(&mount)),
        Some(&mount_exe),
        |_| true
    ));
    assert!(!ok(
        Some(os(&root.join("missing.AppImage"))),
        Some(os(&mount)),
        Some(&mount_exe)
    ));
    assert!(!ok(Some(os(&root)), Some(os(&mount)), Some(&mount_exe)));
    assert!(!ok(
        Some(os(&mount_exe)),
        Some(os(&mount)),
        Some(&mount_exe)
    ));
    // The APPDIR prefix rule alone, with a stub that calls everything a regular file.
    assert!(!appimage_target_ok(
        Some(os(&mount_exe)),
        Some(os(&mount)),
        Some(&mount_exe),
        |_| true
    ));
    assert!(!appimage_target_ok(
        Some(os(&mount)),
        Some(os(&mount)),
        Some(&mount_exe),
        |_| true
    ));

    // $APPDIR must be an absolute runtime folder and hold the running executable.
    assert!(!ok(good_appimage, None, Some(&mount_exe)));
    assert!(!ok(good_appimage, Some(OsStr::new("")), Some(&mount_exe)));
    assert!(!ok(
        good_appimage,
        Some(OsStr::new(".mount_RemModAbC123")),
        Some(&mount_exe)
    ));
    assert!(!ok(
        good_appimage,
        Some(os(&squashfs)),
        Some(&exe_in(&squashfs))
    ));
    assert!(!ok(
        good_appimage,
        Some(os(&mount)),
        Some(&exe_in(&squashfs))
    ));
    assert!(!ok(good_appimage, Some(os(&mount)), None));

    assert!(ok(good_appimage, Some(os(&mount)), Some(&mount_exe)));
    assert!(ok(
        good_appimage,
        Some(os(&extracted)),
        Some(&exe_in(&extracted))
    ));
    let sibling = root.join(".mount_RemModAbC123-other");
    assert!(appimage_target_ok(
        Some(os(&sibling)),
        Some(os(&mount)),
        Some(&mount_exe),
        |_| true
    ));
}

#[test]
fn u5b_appimage_extracted() {
    let (_dir, root) = support::temp_root();
    let extracted = root.join("appimage_extracted_0123456789abcdef");
    assert!(appimage_extracted(Some(os(&extracted))));
    assert!(!appimage_extracted(Some(os(
        &root.join(".mount_RemModAbC123")
    ))));
    assert!(!appimage_extracted(Some(os(&root.join("squashfs-root")))));
    assert!(!appimage_extracted(Some(OsStr::new(
        "appimage_extracted_0123456789abcdef"
    ))));
    assert!(!appimage_extracted(Some(OsStr::new(""))));
    assert!(!appimage_extracted(None));
}

#[test]
fn u6_parse_marker_is_exact() {
    assert_eq!(
        parse_marker(b"install-source=aur"),
        Some(InstallSource::Aur)
    );
    assert_eq!(
        parse_marker(b"install-source=aur\n"),
        Some(InstallSource::Aur)
    );
    assert_eq!(
        parse_marker(b"install-source=local"),
        Some(InstallSource::Local)
    );
    assert_eq!(
        parse_marker(b"install-source=local\n"),
        Some(InstallSource::Local)
    );
    let refused: [&[u8]; 13] = [
        b"install-source=aur\r\n",
        b"\xEF\xBB\xBFinstall-source=aur",
        b" install-source=aur",
        b"install-source=aur ",
        b"install-source=AUR",
        b"install-source=aur\n\n",
        b"install-source=deb",
        b"install-source=LOCAL",
        b"install-source=local ",
        b"install-source=local\r\n",
        b"install-source=localx",
        b"",
        b"\n",
    ];
    for bytes in refused {
        assert_eq!(parse_marker(bytes), None, "{bytes:?}");
    }
    let mut long = b"install-source=aur".to_vec();
    long.resize(257, b' ');
    assert_eq!(parse_marker(&long), None);
}

fn path_of(parts: &[&str]) -> PathBuf {
    parts
        .iter()
        .fold(PathBuf::new(), |path, part| path.join(part))
}

#[test]
fn u7_marker_path_and_read_marker() {
    let marker = |prefix: &[&str]| {
        let mut parts = prefix.to_vec();
        parts.extend(["lib", "remmods", "install-source"]);
        Some(path_of(&parts))
    };
    assert_eq!(
        marker_path(&path_of(&["/", "usr", "bin", "remmods"])),
        marker(&["/", "usr"])
    );
    assert_eq!(
        marker_path(&path_of(&["/", "opt", "x", "bin", "remmods"])),
        marker(&["/", "opt", "x"])
    );
    assert_eq!(marker_path(&path_of(&["bin", "remmods"])), marker(&[]));
    assert_eq!(marker_path(&path_of(&["/", "tmp", "remmods"])), None);
    assert_eq!(marker_path(Path::new("remmods")), None);

    let (_dir, root) = support::temp_root();
    let exe = root.join("bin").join("remmods");
    support::write_tree(&root, &[("bin/remmods", b"elf"), ("lib/remmods/", b"")]);
    let file = marker_path(&exe).expect("marker path");
    assert_eq!(read_marker(&exe), None, "missing");

    fs::write(&file, b"install-source=aur\n").expect("marker");
    assert_eq!(read_marker(&exe), Some(InstallSource::Aur));

    fs::write(&file, b"install-source=local\n").expect("local marker");
    assert_eq!(read_marker(&exe), Some(InstallSource::Local));

    let oversize = vec![b'x'; usize::try_from(MARKER_MAX_BYTES).expect("size") + 1];
    fs::write(&file, oversize).expect("oversize marker");
    assert_eq!(read_marker(&exe), None, "oversize");

    fs::remove_file(&file).expect("remove marker");
    fs::create_dir(&file).expect("marker as a folder");
    assert_eq!(read_marker(&exe), None, "directory");
}

#[test]
fn u8_version_parse() {
    for (text, expected) in [
        ("0.1.0", (0, 1, 0)),
        ("1.0.0", (1, 0, 0)),
        ("10.20.30", (10, 20, 30)),
    ] {
        let (major, minor, patch) = expected;
        assert_eq!(
            AppVersion::parse(text),
            Some(AppVersion {
                major,
                minor,
                patch
            }),
            "{text}"
        );
    }
    for text in [
        "1.0",
        "1.0.0.0",
        "v1.0.0",
        "1.0.0-rc.1",
        "1.0.0+b",
        "01.0.0",
        "",
        " 1.0.0",
        "1.0.x",
        "1234567890.0.0",
    ] {
        assert_eq!(AppVersion::parse(text), None, "{text:?}");
    }
}

#[test]
fn u9_is_offered_only_for_a_newer_release() {
    assert!(is_offered("1.0.0", "1.0.1"));
    assert!(is_offered("1.9.0", "1.10.0"));
    assert!(!is_offered("1.0.0", "1.0.0"));
    assert!(!is_offered("1.0.1", "1.0.0"));
    assert!(!is_offered("1.0.0", "1.0.1-rc.1"));
    assert!(!is_offered("1.0.0-dev", "1.0.1"));
}
