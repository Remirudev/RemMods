//! Top-level navigation filter for the webview. Only RemMods' own origin (and the dev server in
//! dev builds) may load; external pages must never replace the app with IPC access.

/// `true` for `about:blank`, the app origin and (when given) the dev server origin.
/// Origins are compared as scheme + host + port; anything unparsable is refused.
pub fn is_navigation_allowed(url: &str, app_origin: &str, dev_url: Option<&str>) -> bool {
    if url.eq_ignore_ascii_case("about:blank") {
        return true;
    }
    let Some(target) = Origin::parse(url) else {
        return false;
    };
    let allowed = std::iter::once(app_origin).chain(dev_url);
    allowed
        .filter_map(Origin::parse)
        .any(|origin| origin == target)
}

#[derive(Debug, PartialEq, Eq)]
struct Origin {
    scheme: String,
    host: String,
    port: Option<u16>,
}

impl Origin {
    fn parse(url: &str) -> Option<Self> {
        let (scheme, rest) = url.split_once("://")?;
        let scheme = scheme.to_ascii_lowercase();
        if scheme.is_empty() || !scheme.chars().all(is_scheme_char) {
            return None;
        }
        let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
        if authority.contains(['@', '\\']) {
            return None;
        }
        let (host, port) = split_host_port(authority)?;
        if host.is_empty() {
            return None;
        }
        let port = port.or_else(|| default_port(&scheme));
        Some(Self {
            host: host.to_ascii_lowercase(),
            port,
            scheme,
        })
    }
}

fn is_scheme_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')
}

fn split_host_port(authority: &str) -> Option<(&str, Option<u16>)> {
    let (host, port) = if let Some(bracketed) = authority.strip_prefix('[') {
        let (inner, after) = bracketed.split_once(']')?;
        let port = match after {
            "" => None,
            _ => Some(after.strip_prefix(':')?),
        };
        (inner, port)
    } else {
        match authority.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        }
    };
    let port = match port {
        None | Some("") => None,
        Some(digits) => Some(digits.parse::<u16>().ok()?),
    };
    Some((host, port))
}

fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        _ => None,
    }
}
