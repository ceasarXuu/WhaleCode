use super::FetchProvider;
use super::FetchRequest;
use super::WebFetchOutput;
use super::support::build_fetch_output;
use super::support::http_error;
use super::support::read_limited_body;
use crate::web_tools::WebToolError;
use crate::web_tools::safety::validate_http_url;
use crate::web_tools::safety::validate_resolved_http_url;
use async_trait::async_trait;
use reqwest::Client;
use reqwest::header::LOCATION;

pub(super) struct JinaFetchProvider;

#[async_trait]
impl FetchProvider for JinaFetchProvider {
    async fn fetch(
        &self,
        client: &Client,
        request: &FetchRequest,
    ) -> Result<WebFetchOutput, WebToolError> {
        let response = client
            .get(format!("https://r.jina.ai/{}", request.url.as_str()))
            .timeout(request.timeout)
            .header("Accept", "text/plain")
            .send()
            .await
            .map_err(|source| WebToolError::Network {
                provider: "jina",
                source,
            })?;
        let status = response.status();
        let (body, body_truncated) = read_limited_body(response, "jina", request.max_chars).await?;
        if !status.is_success() {
            return Err(http_error("jina", status, body));
        }

        Ok(build_fetch_output(
            "jina",
            &request.url,
            &request.url,
            body,
            request.max_chars,
            request.format,
            body_truncated,
        ))
    }
}

pub(super) struct DirectFetchProvider;

#[async_trait]
impl FetchProvider for DirectFetchProvider {
    async fn fetch(
        &self,
        client: &Client,
        request: &FetchRequest,
    ) -> Result<WebFetchOutput, WebToolError> {
        let mut current_url = request.url.clone();
        for redirect_count in 0..=5 {
            validate_resolved_http_url(&current_url).await?;
            let response = client
                .get(current_url.clone())
                .timeout(request.timeout)
                .send()
                .await
                .map_err(|source| WebToolError::Network {
                    provider: "direct",
                    source,
                })?;
            let status = response.status();
            if status.is_redirection() {
                if redirect_count == 5 {
                    return Err(WebToolError::UnsafeUrl(
                        "too many redirects while fetching URL".to_string(),
                    ));
                }
                let location = response
                    .headers()
                    .get(LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| {
                        WebToolError::UnsafeUrl("redirect missing Location header".to_string())
                    })?;
                current_url = current_url.join(location).map_err(|err| {
                    WebToolError::UnsafeUrl(format!("redirect Location is invalid: {err}"))
                })?;
                validate_http_url(&current_url)?;
                continue;
            }

            let final_url = current_url;
            let (body, body_truncated) =
                read_limited_body(response, "direct", request.max_chars).await?;
            if !status.is_success() {
                return Err(http_error("direct", status, body));
            }

            return Ok(build_fetch_output(
                "direct",
                &request.url,
                &final_url,
                body,
                request.max_chars,
                request.format,
                body_truncated,
            ));
        }

        Err(WebToolError::UnsafeUrl(
            "too many redirects while fetching URL".to_string(),
        ))
    }
}
