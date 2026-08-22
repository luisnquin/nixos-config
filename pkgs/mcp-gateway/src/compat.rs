//! Protocol compatibility between downstream MCP clients and one canonical upstream server.
//!
//! The gateway runs a single upstream process per configured server and per scope key. Clients
//! that speak different revisions of the MCP protocol share that process instead of each getting
//! their own pool. Every downstream client is still negotiated normally: it receives the revision
//! it asked for, while the gateway itself always talks one canonical revision upstream.
//!
//! # Supported revisions
//!
//! Only [`SUPPORTED_VERSIONS`] are accepted. Anything else fails closed with an actionable error
//! instead of silently starting a second pool. Adding a revision means extending this module with
//! the translations that revision needs; there is no generic fallback on purpose.
//!
//! # Compatibility profile
//!
//! The configured servers expose tools, prompts and resources. The method allowlist in [`classify`]
//! encodes exactly that surface. Methods outside it are rejected loudly rather than forwarded,
//! because their cross-revision semantics have not been verified here. In particular the gateway
//! advertises no client capabilities upstream, so `sampling/createMessage`, `elicitation/create`
//! and `roots/list` can never be honoured and are refused.
//!
//! # Covered changelog deltas (2025-03-26 -> 2025-06-18)
//!
//! - **JSON-RPC batching**: removed in 2025-06-18. Batches are refused for every client, since the
//!   canonical upstream link never carries them. See [`BATCHING_REJECTION`].
//! - **`structuredContent`**: added in 2025-06-18. Downgraded only when the same payload is also
//!   present as unstructured `content`, which the specification recommends servers provide. When it
//!   is not, the response is refused instead of dropping data.
//! - **Resource links**: the `resource_link` content block has no 2025-03-26 equivalent, so it is
//!   refused rather than approximated by an embedded resource.
//! - **Elicitation**: a 2025-06-18 server-to-client request. Refused, as above.
//! - **`title`**: purely presentational and always accompanied by `name`, so it is stripped for
//!   2025-03-26 clients.
//! - **`_meta`**: present in both revisions and defined as an open extension point, so it is passed
//!   through untouched in both directions.
//! - **Completion `context`**: added in 2025-06-18. It flows through unchanged while the canonical
//!   revision is at least as new as the client. When the canonical revision is older, the request
//!   is refused instead of being stripped, because dropping it silently changes completion results.
//! - **Initialization lifecycle**: the gateway performs one upstream handshake and answers every
//!   downstream handshake from the cached result, rewritten to that client's revision.
//!
//! # Downlevel upstreams (2024-11-05)
//!
//! An upstream server may answer the canonical handshake with an older revision it prefers.
//! Revisions known to [`Version::parse_downlevel`] are accepted for that link only: an older
//! server never emits fields its revision does not define, so results reach every client
//! untouched, while client requests are checked against the negotiated revision instead of the
//! canonical one. 2024-11-05 is never valid as a client or canonical revision, because the
//! 2025-03-26 deltas (audio content, tool annotations) have no downgrade translations here.

use serde_json::Value;

/// Protocol revisions this gateway knows how to translate between.
pub const SUPPORTED_VERSIONS: &[&str] = &["2025-03-26", "2025-06-18"];

/// Revision the gateway speaks upstream unless configured otherwise.
pub const DEFAULT_CANONICAL_VERSION: &str = "2025-06-18";

const BATCHING_REJECTION: &str =
    "JSON-RPC batching was removed in MCP 2025-06-18 and the gateway never batches upstream";
const SAMPLING_REJECTION: &str = "the gateway advertises no sampling capability upstream, so sampling/createMessage cannot be served";
const ELICITATION_REJECTION: &str = "the gateway advertises no elicitation capability upstream, so elicitation/create cannot be served";
const ROOTS_REJECTION: &str =
    "the gateway advertises no roots capability upstream, so roots/list cannot be served";
const OUTSIDE_PROFILE: &str = "method is outside the gateway compatibility profile (lifecycle, ping, tools, prompts, resources, completion, logging)";

/// An MCP protocol revision the gateway can serve.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Version {
    /// MCP revision 2024-11-05, accepted only when an upstream server negotiates down to it.
    V20241105,
    /// MCP revision 2025-03-26.
    V20250326,
    /// MCP revision 2025-06-18.
    V20250618,
}

impl Version {
    /// Parses a wire protocol revision.
    ///
    /// # Errors
    ///
    /// Returns [`CompatError::UnsupportedVersion`] for any revision this module cannot translate.
    /// Callers must fail closed rather than fall back to a per-revision pool.
    pub fn parse(raw: &str) -> Result<Self, CompatError> {
        match raw {
            "2025-03-26" => Ok(Self::V20250326),
            "2025-06-18" => Ok(Self::V20250618),
            _ => Err(CompatError::UnsupportedVersion {
                requested: raw.to_owned(),
                supported: SUPPORTED_VERSIONS.join(", "),
            }),
        }
    }

    /// Parses a revision negotiated by an upstream server, which may be older than any revision
    /// served downstream. Returns [`None`] for revisions with no translations in this module.
    #[must_use]
    pub fn parse_downlevel(raw: &str) -> Option<Self> {
        match raw {
            "2024-11-05" => Some(Self::V20241105),
            _ => Self::parse(raw).ok(),
        }
    }

    /// Returns the wire representation of this revision.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::V20241105 => "2024-11-05",
            Self::V20250326 => "2025-03-26",
            Self::V20250618 => "2025-06-18",
        }
    }
}

/// How the gateway treats an incoming downstream method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodClass {
    /// Terminated by the gateway itself and never forwarded upstream.
    Lifecycle,
    /// Forwarded to the shared upstream process.
    Forwarded,
    /// Refused, carrying the reason shown to the client and the daemon log.
    Unsupported(&'static str),
}

/// Classifies a downstream method against the documented compatibility profile.
#[must_use]
pub fn classify(method: &str) -> MethodClass {
    match method {
        "initialize" | "notifications/initialized" | "notifications/cancelled" => {
            MethodClass::Lifecycle
        }
        "ping"
        | "tools/list"
        | "tools/call"
        | "prompts/list"
        | "prompts/get"
        | "resources/list"
        | "resources/templates/list"
        | "resources/read"
        | "resources/subscribe"
        | "resources/unsubscribe"
        | "completion/complete"
        | "logging/setLevel"
        | "notifications/progress" => MethodClass::Forwarded,
        "sampling/createMessage" => MethodClass::Unsupported(SAMPLING_REJECTION),
        "elicitation/create" => MethodClass::Unsupported(ELICITATION_REJECTION),
        "roots/list" => MethodClass::Unsupported(ROOTS_REJECTION),
        _ => MethodClass::Unsupported(OUTSIDE_PROFILE),
    }
}

/// Reason a JSON-RPC batch is always refused.
#[must_use]
pub fn batching_rejection() -> &'static str {
    BATCHING_REJECTION
}

/// One field the gateway rewrote while crossing a revision boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Note {
    /// Field that was rewritten.
    pub field: &'static str,
    /// JSON pointer-ish location of the rewrite, for the daemon log.
    pub path: String,
    /// Why the rewrite is faithful.
    pub detail: &'static str,
}

impl Note {
    fn new(field: &'static str, path: impl Into<String>, detail: &'static str) -> Self {
        Self {
            field,
            path: path.into(),
            detail,
        }
    }
}

/// Rewrites a downstream request so the canonical upstream revision can accept it.
///
/// Requests only need work when the client is newer than the canonical revision, which happens
/// when the canonical revision is pinned backwards. Semantics that cannot survive the crossing are
/// refused rather than stripped.
///
/// # Errors
///
/// Returns [`CompatError::UnsupportedRequestSemantics`] when the request carries a field the older
/// canonical revision does not define.
pub fn adapt_request(
    method: &str,
    client: Version,
    canonical: Version,
    request: &Value,
) -> Result<(), CompatError> {
    if client <= canonical {
        return Ok(());
    }
    if method == "completion/complete" && request.pointer("/params/context").is_some() {
        return Err(CompatError::UnsupportedRequestSemantics {
            method: method.to_owned(),
            field: "context",
            canonical: canonical.as_str(),
        });
    }
    Ok(())
}

/// Rewrites an upstream result for a downstream client on an older revision.
///
/// Returns the rewrites performed so the daemon can log each one. Older upstream revisions never
/// emit newer fields, so serving a newer client from an older upstream needs no rewriting.
///
/// # Errors
///
/// Returns a [`CompatError`] when the result carries semantics with no faithful representation in
/// the client's revision. The caller must surface the error instead of forwarding a lossy result.
pub fn downgrade_result(
    method: &str,
    upstream: Version,
    client: Version,
    result: &mut Value,
) -> Result<Vec<Note>, CompatError> {
    let mut notes = Vec::new();
    if upstream <= client || upstream != Version::V20250618 {
        return Ok(notes);
    }

    match method {
        "initialize" => {
            strip_title(
                result.get_mut("serverInfo"),
                "result.serverInfo",
                &mut notes,
            );
        }
        "tools/list" => {
            for (index, tool) in entries(result, "tools") {
                let path = format!("result.tools[{index}]");
                strip_title(Some(tool), &path, &mut notes);
                if tool
                    .as_object_mut()
                    .is_some_and(|object| object.remove("outputSchema").is_some())
                {
                    notes.push(Note::new(
                        "outputSchema",
                        format!("{path}.outputSchema"),
                        "output schemas describe structuredContent, which this revision cannot carry",
                    ));
                }
            }
        }
        "prompts/list" => {
            for (index, prompt) in entries(result, "prompts") {
                let path = format!("result.prompts[{index}]");
                strip_title(Some(prompt), &path, &mut notes);
                for (argument_index, argument) in entries(prompt, "arguments") {
                    strip_title(
                        Some(argument),
                        format!("{path}.arguments[{argument_index}]"),
                        &mut notes,
                    );
                }
            }
        }
        "prompts/get" => {
            for (index, message) in entries(result, "messages") {
                let path = format!("result.messages[{index}].content");
                if let Some(content) = message.get_mut("content") {
                    downgrade_content_block(content, &path, &mut notes)?;
                }
            }
        }
        "resources/list" => {
            for (index, resource) in entries(result, "resources") {
                strip_title(
                    Some(resource),
                    format!("result.resources[{index}]"),
                    &mut notes,
                );
            }
        }
        "resources/templates/list" => {
            for (index, template) in entries(result, "resourceTemplates") {
                strip_title(
                    Some(template),
                    format!("result.resourceTemplates[{index}]"),
                    &mut notes,
                );
            }
        }
        "resources/read" => {
            for (index, contents) in entries(result, "contents") {
                strip_title(
                    Some(contents),
                    format!("result.contents[{index}]"),
                    &mut notes,
                );
            }
        }
        "tools/call" => downgrade_tool_result(result, &mut notes)?,
        _ => {}
    }

    Ok(notes)
}

fn downgrade_tool_result(result: &mut Value, notes: &mut Vec<Note>) -> Result<(), CompatError> {
    let content_blocks = result
        .get("content")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

    if result.get("structuredContent").is_some() {
        if content_blocks == 0 {
            return Err(CompatError::UnrepresentableStructuredContent);
        }
        if let Some(object) = result.as_object_mut() {
            object.remove("structuredContent");
        }
        notes.push(Note::new(
            "structuredContent",
            "result.structuredContent",
            "the same payload remains available as unstructured content blocks",
        ));
    }

    for index in 0..content_blocks {
        let path = format!("result.content[{index}]");
        let Some(block) = result
            .get_mut("content")
            .and_then(Value::as_array_mut)
            .and_then(|blocks| blocks.get_mut(index))
        else {
            continue;
        };
        downgrade_content_block(block, &path, notes)?;
    }
    Ok(())
}

fn downgrade_content_block(
    block: &mut Value,
    path: &str,
    notes: &mut Vec<Note>,
) -> Result<(), CompatError> {
    if block.get("type").and_then(Value::as_str) == Some("resource_link") {
        return Err(CompatError::UnrepresentableResourceLink {
            path: path.to_owned(),
        });
    }
    strip_title(block.get_mut("resource"), format!("{path}.resource"), notes);
    Ok(())
}

fn entries<'a>(parent: &'a mut Value, key: &str) -> Vec<(usize, &'a mut Value)> {
    parent
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .map(|items| items.iter_mut().enumerate().collect())
        .unwrap_or_default()
}

fn strip_title(target: Option<&mut Value>, path: impl Into<String>, notes: &mut Vec<Note>) {
    let Some(object) = target.and_then(Value::as_object_mut) else {
        return;
    };
    if object.remove("title").is_some() {
        notes.push(Note::new(
            "title",
            format!("{}.title", path.into()),
            "title is presentational and the mandatory name field is unchanged",
        ));
    }
}

/// A protocol contract the gateway refuses to serve.
#[derive(Debug, thiserror::Error)]
pub enum CompatError {
    #[error(
        "unsupported MCP protocol version {requested}; this gateway serves {supported}. \
         Add the revision and its translations to pkgs/mcp-gateway/src/compat.rs, \
         then rebuild the gateway"
    )]
    UnsupportedVersion {
        /// Revision the peer asked for.
        requested: String,
        /// Revisions this gateway can serve.
        supported: String,
    },
    #[error(
        "upstream MCP server negotiated {negotiated} but the gateway canonical revision is \
         {canonical}; set services.mcp-gateway.protocolVersion to a revision this server accepts"
    )]
    UpstreamVersionMismatch {
        /// Revision the upstream server answered with.
        negotiated: String,
        /// Revision the gateway requested.
        canonical: &'static str,
    },
    #[error(
        "{method} carries {field}, which MCP {canonical} does not define; \
         raise services.mcp-gateway.protocolVersion so the upstream link understands it"
    )]
    UnsupportedRequestSemantics {
        /// Method that carried the unsupported field.
        method: String,
        /// Field with no representation in the canonical revision.
        field: &'static str,
        /// Canonical revision in force.
        canonical: &'static str,
    },
    #[error(
        "tool result carries structuredContent with no unstructured content to fall back on, \
         which MCP 2025-03-26 cannot represent; upgrade this client to MCP 2025-06-18"
    )]
    UnrepresentableStructuredContent,
    #[error(
        "{path} is a resource_link, which MCP 2025-03-26 does not define; \
         upgrade this client to MCP 2025-06-18"
    )]
    UnrepresentableResourceLink {
        /// Location of the offending content block.
        path: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unknown_versions_fail_closed_with_an_actionable_message() {
        let error = Version::parse("2099-01-01").expect_err("unknown revision must be refused");
        let message = error.to_string();
        assert!(message.contains("2099-01-01"));
        assert!(message.contains("compat.rs"));
    }

    #[test]
    fn observed_versions_parse() {
        assert_eq!(Version::parse("2025-03-26").unwrap(), Version::V20250326);
        assert_eq!(Version::parse("2025-06-18").unwrap(), Version::V20250618);
    }

    #[test]
    fn the_downlevel_revision_is_upstream_only() {
        assert_eq!(
            Version::parse_downlevel("2024-11-05"),
            Some(Version::V20241105)
        );
        assert!(Version::parse("2024-11-05").is_err());
        assert!(Version::V20241105 < Version::V20250326);
    }

    #[test]
    fn downlevel_upstream_results_are_never_rewritten() {
        let mut result = json!({"tools": [{"name": "a"}]});
        let notes = downgrade_result(
            "tools/list",
            Version::V20241105,
            Version::V20250618,
            &mut result,
        )
        .unwrap();
        assert!(notes.is_empty());
    }

    #[test]
    fn profile_rejects_capabilities_the_gateway_never_advertises() {
        assert!(matches!(
            classify("elicitation/create"),
            MethodClass::Unsupported(_)
        ));
        assert!(matches!(
            classify("sampling/createMessage"),
            MethodClass::Unsupported(_)
        ));
        assert!(matches!(classify("tools/call"), MethodClass::Forwarded));
        assert!(matches!(classify("initialize"), MethodClass::Lifecycle));
    }

    #[test]
    fn titles_are_stripped_for_the_older_revision() {
        let mut result = json!({"tools": [{"name": "a", "title": "A", "outputSchema": {}}]});
        let notes = downgrade_result(
            "tools/list",
            Version::V20250618,
            Version::V20250326,
            &mut result,
        )
        .unwrap();
        assert_eq!(result["tools"][0].get("title"), None);
        assert_eq!(result["tools"][0].get("outputSchema"), None);
        assert_eq!(result["tools"][0]["name"], json!("a"));
        assert_eq!(notes.len(), 2);
    }

    #[test]
    fn same_revision_is_never_rewritten() {
        let mut result = json!({"tools": [{"name": "a", "title": "A"}]});
        let notes = downgrade_result(
            "tools/list",
            Version::V20250618,
            Version::V20250618,
            &mut result,
        )
        .unwrap();
        assert!(notes.is_empty());
        assert_eq!(result["tools"][0]["title"], json!("A"));
    }

    #[test]
    fn structured_content_downgrades_only_with_a_textual_mirror() {
        let mut mirrored = json!({
            "content": [{"type": "text", "text": "{\"ok\":true}"}],
            "structuredContent": {"ok": true},
        });
        let notes = downgrade_result(
            "tools/call",
            Version::V20250618,
            Version::V20250326,
            &mut mirrored,
        )
        .unwrap();
        assert_eq!(mirrored.get("structuredContent"), None);
        assert_eq!(mirrored["content"][0]["text"], json!("{\"ok\":true}"));
        assert_eq!(notes.len(), 1);

        let mut bare = json!({"content": [], "structuredContent": {"ok": true}});
        let error = downgrade_result(
            "tools/call",
            Version::V20250618,
            Version::V20250326,
            &mut bare,
        )
        .expect_err("structured-only results cannot be downgraded");
        assert!(matches!(
            error,
            CompatError::UnrepresentableStructuredContent
        ));
    }

    #[test]
    fn resource_links_are_refused_rather_than_approximated() {
        let mut result = json!({
            "content": [{"type": "resource_link", "uri": "file:///x", "name": "x"}],
        });
        let error = downgrade_result(
            "tools/call",
            Version::V20250618,
            Version::V20250326,
            &mut result,
        )
        .expect_err("resource links have no 2025-03-26 equivalent");
        assert!(matches!(
            error,
            CompatError::UnrepresentableResourceLink { .. }
        ));
    }

    #[test]
    fn completion_context_is_refused_when_the_canonical_revision_is_older() {
        let request = json!({"params": {"context": {"arguments": {}}}});
        adapt_request(
            "completion/complete",
            Version::V20250618,
            Version::V20250326,
            &request,
        )
        .expect_err("context has no 2025-03-26 representation");

        adapt_request(
            "completion/complete",
            Version::V20250618,
            Version::V20250618,
            &request,
        )
        .expect("a canonical revision that defines context accepts it unchanged");
    }

    #[test]
    fn older_upstream_results_reach_newer_clients_untouched() {
        let mut result = json!({"tools": [{"name": "a"}]});
        let notes = downgrade_result(
            "tools/list",
            Version::V20250326,
            Version::V20250618,
            &mut result,
        )
        .unwrap();
        assert!(notes.is_empty());
        assert_eq!(result["tools"][0]["name"], json!("a"));
    }
}
