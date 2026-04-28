use crate::web_tools::WebToolError;
use std::net::IpAddr;
use url::Url;

pub(crate) fn normalize_fetch_url(raw_url: &str) -> Result<Url, WebToolError> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return Err(WebToolError::InvalidArguments("url is empty".to_string()));
    }

    let parsed = Url::parse(trimmed)
        .map_err(|err| WebToolError::InvalidArguments(format!("url is invalid: {err}")))?;
    validate_http_url(&parsed)?;
    Ok(convert_github_blob_url(parsed))
}

pub(crate) fn validate_http_url(url: &Url) -> Result<(), WebToolError> {
    match url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(WebToolError::UnsafeUrl(format!(
                "unsupported scheme {scheme}"
            )));
        }
    }

    let Some(host) = url.host_str() else {
        return Err(WebToolError::UnsafeUrl("missing host".to_string()));
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if is_blocked_host_name(&host) {
        return Err(WebToolError::UnsafeUrl(format!("blocked host {host}")));
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        validate_public_ip(ip)?;
    }
    Ok(())
}

pub(crate) async fn validate_resolved_http_url(url: &Url) -> Result<(), WebToolError> {
    validate_http_url(url)?;

    let Some(host) = url.host_str() else {
        return Err(WebToolError::UnsafeUrl("missing host".to_string()));
    };
    if host.parse::<IpAddr>().is_ok() {
        return Ok(());
    }
    let port = url
        .port_or_known_default()
        .ok_or_else(|| WebToolError::UnsafeUrl("URL does not have a known port".to_string()))?;

    let mut resolved_any = false;
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|err| WebToolError::UnsafeUrl(format!("host resolution failed: {err}")))?;
    for addr in addrs {
        resolved_any = true;
        validate_public_ip(addr.ip())?;
    }

    if resolved_any {
        Ok(())
    } else {
        Err(WebToolError::UnsafeUrl(format!(
            "host {host} did not resolve"
        )))
    }
}

pub(crate) fn sanitized_url_for_event(url: &Url) -> String {
    let mut sanitized = url.clone();
    sanitized.set_query(None);
    sanitized.set_fragment(None);
    sanitized.to_string()
}

pub(crate) fn convert_github_blob_url(url: Url) -> Url {
    if url.host_str() != Some("github.com") {
        return url;
    }

    let Some(segments) = url.path_segments() else {
        return url;
    };
    let segments = segments.collect::<Vec<_>>();
    if segments.len() < 5 || segments.get(2) != Some(&"blob") {
        return url;
    }

    let owner = segments[0];
    let repo = segments[1];
    let branch = segments[3];
    let path = segments[4..].join("/");
    Url::parse(&format!(
        "https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{path}"
    ))
    .unwrap_or(url)
}

fn validate_public_ip(ip: IpAddr) -> Result<(), WebToolError> {
    let blocked = match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.is_unspecified()
                || ip.octets()[0] == 0
                || ip.octets()[0] >= 224
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || matches!(ip.segments()[0] & 0xfe00, 0xfc00 | 0xfe80)
        }
    };

    if blocked {
        Err(WebToolError::UnsafeUrl(format!("blocked IP {ip}")))
    } else {
        Ok(())
    }
}

fn is_blocked_host_name(host: &str) -> bool {
    host == "localhost"
        || host.ends_with(".localhost")
        || host == "metadata.google.internal"
        || host == "169.254.169.254"
        || host == "host.docker.internal"
        || host.ends_with(".internal")
        || host.ends_with(".local")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_blob_url_is_converted_to_raw_url() {
        let url =
            normalize_fetch_url("https://github.com/openai/codex/blob/main/codex-rs/README.md")
                .expect("normalize");

        assert_eq!(
            url.as_str(),
            "https://raw.githubusercontent.com/openai/codex/main/codex-rs/README.md"
        );
    }

    #[test]
    fn private_urls_are_rejected() {
        for url in [
            "http://127.0.0.1/",
            "http://10.0.0.1/",
            "http://169.254.169.254/latest/meta-data/",
            "file:///etc/passwd",
            "https://metadata.google.internal/computeMetadata/v1/",
        ] {
            assert!(normalize_fetch_url(url).is_err(), "{url} should be blocked");
        }
    }
}
