use remmods_core::navigation::is_navigation_allowed;

const LINUX_ORIGIN: &str = "tauri://localhost";
const WINDOWS_ORIGIN: &str = "http://tauri.localhost";
const DEV_URL: &str = "http://localhost:1430/";

#[test]
fn allows_the_app_origin_on_both_oses() {
    assert!(is_navigation_allowed(
        "tauri://localhost",
        LINUX_ORIGIN,
        None
    ));
    assert!(is_navigation_allowed(
        "tauri://localhost/",
        LINUX_ORIGIN,
        None
    ));
    assert!(is_navigation_allowed(
        "tauri://localhost/index.html?x=1#top",
        LINUX_ORIGIN,
        None
    ));
    assert!(is_navigation_allowed(
        "http://tauri.localhost/",
        WINDOWS_ORIGIN,
        None
    ));
    assert!(is_navigation_allowed(
        "HTTP://Tauri.LocalHost:80/index.html",
        WINDOWS_ORIGIN,
        None
    ));
}

#[test]
fn allows_about_blank() {
    assert!(is_navigation_allowed("about:blank", LINUX_ORIGIN, None));
}

#[test]
fn origin_of_the_other_os_is_refused() {
    assert!(!is_navigation_allowed(
        "http://tauri.localhost/",
        LINUX_ORIGIN,
        None
    ));
    assert!(!is_navigation_allowed(
        "tauri://localhost/",
        WINDOWS_ORIGIN,
        None
    ));
    assert!(!is_navigation_allowed(
        "https://tauri.localhost/",
        WINDOWS_ORIGIN,
        None
    ));
}

#[test]
fn dev_url_is_allowed_only_when_given() {
    assert!(is_navigation_allowed(
        "http://localhost:1430/",
        LINUX_ORIGIN,
        Some(DEV_URL)
    ));
    assert!(is_navigation_allowed(
        "http://localhost:1430/src/main.tsx",
        WINDOWS_ORIGIN,
        Some(DEV_URL)
    ));
    assert!(!is_navigation_allowed(
        "http://localhost:1430/",
        LINUX_ORIGIN,
        None
    ));
}

#[test]
fn other_ports_and_hosts_are_refused() {
    assert!(!is_navigation_allowed(
        "http://localhost:1431/",
        LINUX_ORIGIN,
        Some(DEV_URL)
    ));
    assert!(!is_navigation_allowed(
        "http://localhost/",
        LINUX_ORIGIN,
        Some(DEV_URL)
    ));
    assert!(!is_navigation_allowed(
        "http://127.0.0.1:1430/",
        LINUX_ORIGIN,
        Some(DEV_URL)
    ));
    assert!(!is_navigation_allowed(
        "tauri://localhost:8080/",
        LINUX_ORIGIN,
        None
    ));
    assert!(!is_navigation_allowed(
        "http://tauri.localhost.evil.com/",
        WINDOWS_ORIGIN,
        None
    ));
    assert!(!is_navigation_allowed(
        "http://tauri.localhost@evil.com/",
        WINDOWS_ORIGIN,
        None
    ));
}

#[test]
fn external_and_local_schemes_are_refused() {
    let refused = [
        "https://example.com/",
        "http://example.com",
        "file:///etc/passwd",
        "file://C:/Windows/win.ini",
        "data:text/html,<script>alert(1)</script>",
        "javascript:alert(1)",
        "blob:tauri://localhost/1234",
        "about:srcdoc",
        "",
        "not a url",
        "tauri://",
        "http://localhost:99999/",
    ];
    for url in refused {
        assert!(
            !is_navigation_allowed(url, LINUX_ORIGIN, Some(DEV_URL)),
            "{url} must be refused"
        );
        assert!(
            !is_navigation_allowed(url, WINDOWS_ORIGIN, Some(DEV_URL)),
            "{url} must be refused"
        );
    }
}

#[test]
fn ipv6_dev_hosts_compare_with_their_port() {
    let dev = Some("http://[::1]:1430/");

    assert!(is_navigation_allowed(
        "http://[::1]:1430/app",
        LINUX_ORIGIN,
        dev
    ));
    assert!(!is_navigation_allowed(
        "http://[::1]:1431/app",
        LINUX_ORIGIN,
        dev
    ));
}
