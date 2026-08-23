//! Protocol compatibility between downstream MCP clients and one canonical upstream server.
//!
//! The gateway runs a single upstream process per configured server and per scope key. Clients
//! that speak different revisions of the MCP protocol share that process instead of each getting
//! their own pool. Every downstream client is negotiated normally, and the gateway itself always
//! talks one canonical revision upstream.
//!
//! # Negotiation is version-agnostic; translation is not
//!
//! These are separate concerns and this module keeps them separate.
//!
//! *Negotiation* needs no knowledge of what a revision contains, only how revisions order. A
//! client proposing any revision at all is answered by [`negotiate`] with the newest revision the
//! gateway can honour, and the MCP lifecycle lets the client disconnect if that is not enough for
//! it. A revision this module has never heard of is therefore served, not refused: it is answered
//! at [`NEWEST_TRANSLATABLE`] and the client speaks that instead. No client is ever rejected over
//! its protocol revision, so a new MCP release does not need a gateway release.
//!
//! *Translation* does need that knowledge, and only ever runs in one direction: an upstream result
//! crossing down to a client on an older revision. [`downgrade_result`] is a ladder of per-revision
//! steps, and the revisions it spans are bounded by [`OLDEST_TRANSLATABLE`] and
//! [`NEWEST_TRANSLATABLE`]. Adding a revision to the ladder means adding one step.
//!
//! # Why the bounds hold
//!
//! [`negotiate`] never serves a revision above [`NEWEST_TRANSLATABLE`], nor one below
//! [`OLDEST_TRANSLATABLE`] while the upstream link sits above it. [`Version::parse_canonical`]
//! refuses a canonical revision above [`NEWEST_TRANSLATABLE`] at startup, and an upstream server
//! may only negotiate at or below the canonical revision it was offered. Every crossing the ladder
//! is asked to perform therefore falls inside its bounds, and no payload can reach a client
//! untranslated.
//!
//! # Compatibility profile
//!
//! The configured servers expose tools, prompts and resources. The method allowlist in [`classify`]
//! encodes exactly that surface. Methods outside it are rejected loudly rather than forwarded,
//! because their cross-revision semantics have not been verified here. The gateway advertises no
//! client capabilities upstream, so `sampling/createMessage`, `elicitation/create` and `roots/list`
//! can never be honoured and are refused. Capabilities the gateway will not relay are stripped
//! from the handshake by [`redact_unservable_capabilities`] rather than advertised and then denied.
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
//! - **`title`**: purely presentational and always accompanied by `name`, so it is stripped.
//! - **`_meta`**: present in both revisions and defined as an open extension point, so it is passed
//!   through untouched in both directions.
//! - **Completion `context`**: added in 2025-06-18. Refused only when the upstream link itself
//!   predates it, because dropping it silently changes completion results.
//!
//! # Covered changelog deltas (2025-06-18 -> 2025-11-25)
//!
//! - **`icons`**: added in 2025-11-25 for tools, prompts, resources and resource templates. Purely
//!   presentational, like `title`, so it is stripped for older clients.
//! - **Tasks**: an experimental 2025-11-25 facility for durable requests. The gateway does not
//!   relay the `tasks/*` methods, so it strips the `tasks` capability from the handshake rather
//!   than letting a client discover a facility that would then be refused.
//! - **Authorization** (OpenID Connect discovery, incremental scope consent, OAuth Client ID
//!   Metadata Documents) and the Streamable HTTP `Origin` rule apply to the HTTP transport. The
//!   gateway speaks stdio over a unix socket, so none of it reaches the wire here.
//! - **Sampling `tools`/`toolChoice`** and the **`ElicitResult`/`EnumSchema`** and **URL
//!   elicitation** changes extend capabilities the gateway already refuses outright.
//! - **JSON Schema 2020-12 as the default dialect**, the tool-name guidance and the decoupling of
//!   request payloads from RPC method definitions change no wire shape, so nothing is translated.
//!
//! # Downlevel upstreams
//!
//! An upstream server may answer the canonical handshake with an older revision it prefers. That
//! is accepted for the link: an older server never emits fields its revision does not define, so
//! its results reach every client untouched, and client requests are checked against the
//! negotiated revision instead of the canonical one.

use std::fmt;

use serde_json::Value;

/// MCP revision 2025-03-26.
pub const V2025_03_26: Version = Version(*b"2025-03-26");
/// MCP revision 2025-06-18.
pub const V2025_06_18: Version = Version(*b"2025-06-18");
/// MCP revision 2025-11-25.
pub const V2025_11_25: Version = Version(*b"2025-11-25");

/// Oldest revision a client is served while the upstream link sits above it.
///
/// A client asking for something older is answered here instead, because the ladder in
/// [`downgrade_result`] has no step that reaches further down: the 2025-03-26 additions (audio
/// content, tool annotations) have no translation in this module.
pub const OLDEST_TRANSLATABLE: Version = V2025_03_26;

/// Newest revision the ladder in [`downgrade_result`] can translate down from.
///
/// This bounds the canonical revision rather than any client. Raising it means adding the
/// revision's deltas to the ladder.
pub const NEWEST_TRANSLATABLE: Version = V2025_11_25;

/// Revision the gateway speaks upstream unless configured otherwise.
pub const DEFAULT_CANONICAL_VERSION: &str = "2025-06-18";

const BATCHING_REJECTION: &str =
    "JSON-RPC batching was removed in MCP 2025-06-18 and the gateway never batches upstream";
const SAMPLING_REJECTION: &str = "the gateway advertises no sampling capability upstream, so sampling/createMessage cannot be served";
const ELICITATION_REJECTION: &str = "the gateway advertises no elicitation capability upstream, so elicitation/create cannot be served";
const ROOTS_REJECTION: &str =
    "the gateway advertises no roots capability upstream, so roots/list cannot be served";
const TASKS_REJECTION: &str = "the gateway does not relay MCP tasks, and strips the tasks capability from the handshake so none are offered";
const OUTSIDE_PROFILE: &str = "method is outside the gateway compatibility profile (lifecycle, ping, tools, prompts, resources, completion, logging)";

/// Server capabilities the gateway refuses to relay, stripped from every handshake it serves.
const UNSERVABLE_CAPABILITIES: &[&str] = &["tasks"];

/// An MCP protocol revision, held in its wire form.
///
/// MCP revisions are `YYYY-MM-DD` dates, so the derived byte ordering is chronological ordering.
/// Any well-formed date parses, including revisions released after this gateway was built;
/// ordering them is all [`negotiate`] needs.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Version([u8; 10]);

impl Version {
    /// Parses a wire protocol revision, accepting any well-formed `YYYY-MM-DD` date.
    ///
    /// Returns [`None`] only for a string that is not a revision at all, which cannot be ordered
    /// against anything and so cannot be negotiated.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let bytes: [u8; 10] = raw.as_bytes().try_into().ok()?;
        let shaped = bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        });
        shaped.then_some(Self(bytes))
    }

    /// Parses the canonical revision the gateway will speak upstream.
    ///
    /// # Errors
    ///
    /// Returns [`CompatError::MalformedVersion`] when the string is not a revision, and
    /// [`CompatError::UntranslatableCanonical`] when it is newer than the ladder can translate
    /// down from. The second check is what lets [`negotiate`] serve any client without ever
    /// producing a crossing the ladder cannot perform.
    pub fn parse_canonical(raw: &str) -> Result<Self, CompatError> {
        let version = Self::parse(raw).ok_or_else(|| CompatError::MalformedVersion {
            raw: raw.to_owned(),
        })?;
        if version > NEWEST_TRANSLATABLE {
            return Err(CompatError::UntranslatableCanonical {
                canonical: version.to_string(),
                newest: NEWEST_TRANSLATABLE,
            });
        }
        Ok(version)
    }

    /// Returns the wire representation of this revision.
    #[must_use]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("0000-00-00")
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Chooses the revision to serve a client, given the revision the upstream link negotiated.
///
/// A client is never refused over its revision. It is answered with the newest revision this
/// gateway can honour, which is the specification's own negotiation rule: the server answers with
/// a revision it supports, and a client that cannot live with the answer terminates the connection
/// itself.
///
/// The answer is bounded by [`NEWEST_TRANSLATABLE`] rather than by the upstream revision. Serving
/// a client a revision *newer* than the upstream link is sound, because an older upstream emits
/// only fields the newer client already understands, and the request direction is policed by
/// [`adapt_request`]. What is not sound is claiming a revision whose deltas this module has never
/// seen: nothing would police the fields such a client is entitled to send. So an unknown future
/// revision is answered at the newest revision the gateway actually implements.
#[must_use]
pub fn negotiate(client: Version, upstream: Version) -> Version {
    let served = client.min(NEWEST_TRANSLATABLE);
    if served < upstream && served < OLDEST_TRANSLATABLE {
        // Serving this client its own revision would oblige the ladder to reach below its floor.
        // Offer the floor instead and let the client decide whether it can speak it.
        return OLDEST_TRANSLATABLE.min(upstream);
    }
    served
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
        _ if method.starts_with("tasks/") => MethodClass::Unsupported(TASKS_REJECTION),
        _ => MethodClass::Unsupported(OUTSIDE_PROFILE),
    }
}

/// Reason a JSON-RPC batch is always refused.
#[must_use]
pub fn batching_rejection() -> &'static str {
    BATCHING_REJECTION
}

/// Removes capabilities the gateway will not relay from an `initialize` result.
///
/// Applied to every client on every revision. A capability the profile refuses must not be
/// advertised, or a client discovers a facility whose every call is then rejected.
pub fn redact_unservable_capabilities(result: &mut Value, notes: &mut Vec<Note>) {
    let Some(capabilities) = result
        .get_mut("capabilities")
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    for capability in UNSERVABLE_CAPABILITIES {
        if capabilities.remove(*capability).is_some() {
            notes.push(Note::new(
                capability,
                format!("result.capabilities.{capability}"),
                "the gateway does not relay this capability, so it is not advertised",
            ));
        }
    }
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

/// Rewrites a downstream request so the upstream link can accept it.
///
/// Keyed on what the upstream revision actually defines rather than on how it compares to the
/// client, so a client newer than the upstream link is only refused over a field the upstream
/// genuinely predates.
///
/// # Errors
///
/// Returns [`CompatError::UnsupportedRequestSemantics`] when the request carries a field the
/// upstream revision does not define.
pub fn adapt_request(method: &str, upstream: Version, request: &Value) -> Result<(), CompatError> {
    if method == "completion/complete"
        && upstream < V2025_06_18
        && request.pointer("/params/context").is_some()
    {
        return Err(CompatError::UnsupportedRequestSemantics {
            method: method.to_owned(),
            field: "context",
            upstream: upstream.to_string(),
        });
    }
    Ok(())
}

/// Rewrites an upstream result for a downstream client on an older revision.
///
/// Walks one step per revision the crossing spans, newest first, so each step only has to know
/// what its own revision added. Older upstream revisions never emit newer fields, so serving a
/// newer client from an older upstream needs no rewriting and returns immediately.
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
    if upstream <= client {
        return Ok(notes);
    }
    if upstream >= V2025_11_25 && client < V2025_11_25 {
        strip_2025_11_25_additions(method, result, &mut notes);
    }
    if upstream >= V2025_06_18 && client < V2025_06_18 {
        strip_2025_06_18_additions(method, result, &mut notes)?;
    }
    Ok(notes)
}

/// Strips what 2025-11-25 added, for a client that predates it.
fn strip_2025_11_25_additions(method: &str, result: &mut Value, notes: &mut Vec<Note>) {
    const DETAIL: &str = "icons are presentational and the mandatory name field is unchanged";

    let collection = match method {
        "initialize" => {
            strip_field(
                result.get_mut("serverInfo"),
                "icons",
                "result.serverInfo",
                DETAIL,
                notes,
            );
            return;
        }
        "tools/list" => "tools",
        "prompts/list" => "prompts",
        "resources/list" => "resources",
        "resources/templates/list" => "resourceTemplates",
        _ => return,
    };
    for (index, entry) in entries(result, collection) {
        strip_field(
            Some(entry),
            "icons",
            format!("result.{collection}[{index}]"),
            DETAIL,
            notes,
        );
    }
}

/// Strips what 2025-06-18 added, for a client that predates it.
fn strip_2025_06_18_additions(
    method: &str,
    result: &mut Value,
    notes: &mut Vec<Note>,
) -> Result<(), CompatError> {
    match method {
        "initialize" => {
            strip_title(result.get_mut("serverInfo"), "result.serverInfo", notes);
        }
        "tools/list" => {
            for (index, tool) in entries(result, "tools") {
                let path = format!("result.tools[{index}]");
                strip_title(Some(tool), &path, notes);
                strip_field(
                    Some(tool),
                    "outputSchema",
                    &path,
                    "output schemas describe structuredContent, which this revision cannot carry",
                    notes,
                );
            }
        }
        "prompts/list" => {
            for (index, prompt) in entries(result, "prompts") {
                let path = format!("result.prompts[{index}]");
                strip_title(Some(prompt), &path, notes);
                for (argument_index, argument) in entries(prompt, "arguments") {
                    strip_title(
                        Some(argument),
                        format!("{path}.arguments[{argument_index}]"),
                        notes,
                    );
                }
            }
        }
        "prompts/get" => {
            for (index, message) in entries(result, "messages") {
                let path = format!("result.messages[{index}].content");
                if let Some(content) = message.get_mut("content") {
                    downgrade_content_block(content, &path, notes)?;
                }
            }
        }
        "resources/list" => {
            for (index, resource) in entries(result, "resources") {
                strip_title(Some(resource), format!("result.resources[{index}]"), notes);
            }
        }
        "resources/templates/list" => {
            for (index, template) in entries(result, "resourceTemplates") {
                strip_title(
                    Some(template),
                    format!("result.resourceTemplates[{index}]"),
                    notes,
                );
            }
        }
        "resources/read" => {
            for (index, contents) in entries(result, "contents") {
                strip_title(Some(contents), format!("result.contents[{index}]"), notes);
            }
        }
        "tools/call" => downgrade_tool_result(result, notes)?,
        _ => {}
    }
    Ok(())
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
    strip_field(
        target,
        "title",
        path,
        "title is presentational and the mandatory name field is unchanged",
        notes,
    );
}

fn strip_field(
    target: Option<&mut Value>,
    field: &'static str,
    path: impl Into<String>,
    detail: &'static str,
    notes: &mut Vec<Note>,
) {
    let Some(object) = target.and_then(Value::as_object_mut) else {
        return;
    };
    if object.remove(field).is_some() {
        notes.push(Note::new(field, format!("{}.{field}", path.into()), detail));
    }
}

/// A protocol contract the gateway refuses to serve.
#[derive(Debug, thiserror::Error)]
pub enum CompatError {
    #[error("{raw} is not an MCP protocol revision; revisions are dates in YYYY-MM-DD form")]
    MalformedVersion {
        /// String that could not be read as a revision.
        raw: String,
    },
    #[error(
        "MCP {canonical} is newer than {newest}, the newest revision this gateway can translate \
         down from; lower services.mcp-gateway.protocolVersion, or add the revision's deltas to \
         the ladder in pkgs/mcp-gateway/src/compat.rs and rebuild the gateway"
    )]
    UntranslatableCanonical {
        /// Canonical revision that was configured.
        canonical: String,
        /// Newest revision the ladder covers.
        newest: Version,
    },
    #[error(
        "upstream MCP server negotiated {negotiated}, which is newer than the canonical revision \
         {canonical} the gateway proposed; the gateway cannot translate a revision it never asked \
         for"
    )]
    UpstreamVersionMismatch {
        /// Revision the upstream server answered with.
        negotiated: String,
        /// Revision the gateway requested.
        canonical: Version,
    },
    #[error(
        "{method} carries {field}, which MCP {upstream} does not define; \
         raise services.mcp-gateway.protocolVersion so the upstream link understands it"
    )]
    UnsupportedRequestSemantics {
        /// Method that carried the unsupported field.
        method: String,
        /// Field with no representation in the upstream revision.
        field: &'static str,
        /// Revision the upstream link negotiated.
        upstream: String,
    },
    #[error(
        "tool result carries structuredContent with no unstructured content to fall back on, \
         which MCP revisions before 2025-06-18 cannot represent; \
         upgrade this client to MCP 2025-06-18"
    )]
    UnrepresentableStructuredContent,
    #[error(
        "{path} is a resource_link, which MCP revisions before 2025-06-18 do not define; \
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

    fn v(raw: &str) -> Version {
        Version::parse(raw).expect("well-formed revision")
    }

    #[test]
    fn named_revisions_round_trip_through_the_parser() {
        for named in [V2025_03_26, V2025_06_18, V2025_11_25] {
            assert_eq!(Version::parse(named.as_str()), Some(named));
        }
    }

    #[test]
    fn any_well_formed_date_parses_including_unreleased_revisions() {
        assert!(Version::parse("2026-07-28").is_some());
        assert!(Version::parse("2099-01-01").is_some());
        assert!(Version::parse("draft").is_none());
        assert!(Version::parse("1.0.0").is_none());
        assert!(Version::parse("2025-6-18").is_none());
    }

    #[test]
    fn revisions_order_chronologically() {
        assert!(v("2024-11-05") < V2025_03_26);
        assert!(V2025_03_26 < V2025_06_18);
        assert!(V2025_06_18 < V2025_11_25);
        assert!(V2025_11_25 < v("2026-07-28"));
    }

    #[test]
    fn a_client_on_an_unknown_future_revision_is_negotiated_down_never_refused() {
        // The failure this design exists to remove: a client on a revision released after the
        // gateway was built is answered at the newest revision the gateway implements.
        assert_eq!(negotiate(v("2026-07-28"), V2025_06_18), NEWEST_TRANSLATABLE);
        assert_eq!(negotiate(v("2099-01-01"), V2025_03_26), NEWEST_TRANSLATABLE);
    }

    #[test]
    fn a_client_newer_than_a_downlevel_upstream_keeps_its_own_revision() {
        // An older upstream emits only fields a newer client already understands, so there is no
        // reason to downgrade the client to match it.
        assert_eq!(negotiate(V2025_11_25, V2025_06_18), V2025_11_25);
        assert_eq!(negotiate(V2025_06_18, v("2024-11-05")), V2025_06_18);
    }

    #[test]
    fn a_matching_client_is_served_its_own_revision() {
        assert_eq!(negotiate(V2025_06_18, V2025_06_18), V2025_06_18);
        assert_eq!(negotiate(V2025_03_26, V2025_06_18), V2025_03_26);
        assert_eq!(negotiate(V2025_11_25, V2025_11_25), V2025_11_25);
    }

    #[test]
    fn a_client_below_the_ladder_floor_is_offered_the_floor() {
        // 2024-11-05 lacks translations here, so it is answered at the floor rather than served a
        // payload the ladder cannot rewrite. The client disconnects itself if that is too new.
        assert_eq!(negotiate(v("2024-11-05"), V2025_06_18), OLDEST_TRANSLATABLE);
        // Unless the upstream link is itself that old, in which case nothing needs rewriting.
        assert_eq!(negotiate(v("2024-11-05"), v("2024-11-05")), v("2024-11-05"));
        assert_eq!(negotiate(v("2024-01-01"), v("2024-11-05")), v("2024-11-05"));
    }

    #[test]
    fn negotiation_never_produces_a_crossing_the_ladder_cannot_perform() {
        let candidates = [
            "2024-10-07",
            "2024-11-05",
            "2025-03-26",
            "2025-06-18",
            "2025-11-25",
            "2026-07-28",
            "2099-01-01",
        ];
        for upstream in candidates.iter().map(|raw| v(raw)) {
            if upstream > NEWEST_TRANSLATABLE {
                // Refused at startup, so it can never reach negotiation.
                assert!(Version::parse_canonical(upstream.as_str()).is_err());
                continue;
            }
            for client in candidates.iter().map(|raw| v(raw)) {
                let served = negotiate(client, upstream);
                assert!(
                    served <= NEWEST_TRANSLATABLE,
                    "{served} is a revision the ladder does not know"
                );
                assert!(
                    served >= upstream.min(OLDEST_TRANSLATABLE),
                    "{served} is below the ladder floor for upstream {upstream}"
                );
            }
        }
    }

    #[test]
    fn a_canonical_revision_past_the_ladder_is_refused_at_startup() {
        assert_eq!(Version::parse_canonical("2025-06-18").unwrap(), V2025_06_18);
        assert_eq!(Version::parse_canonical("2025-11-25").unwrap(), V2025_11_25);

        let error = Version::parse_canonical("2026-07-28")
            .expect_err("a revision past the ladder must not be canonical");
        let message = error.to_string();
        assert!(message.contains("2026-07-28"), "{message}");
        assert!(message.contains("2025-11-25"), "{message}");
        assert!(message.contains("compat.rs"), "{message}");

        let error =
            Version::parse_canonical("draft").expect_err("a non-revision must not be canonical");
        assert!(matches!(error, CompatError::MalformedVersion { .. }));
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
        assert!(matches!(classify("tasks/get"), MethodClass::Unsupported(_)));
        assert!(matches!(
            classify("tasks/result"),
            MethodClass::Unsupported(_)
        ));
        assert!(matches!(classify("tools/call"), MethodClass::Forwarded));
        assert!(matches!(classify("initialize"), MethodClass::Lifecycle));
    }

    #[test]
    fn unservable_capabilities_are_never_advertised() {
        let mut result = json!({"capabilities": {"tools": {}, "tasks": {"list": true}}});
        let mut notes = Vec::new();
        redact_unservable_capabilities(&mut result, &mut notes);
        assert_eq!(result["capabilities"].get("tasks"), None);
        assert!(result["capabilities"].get("tools").is_some());
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn titles_are_stripped_for_the_older_revision() {
        let mut result = json!({"tools": [{"name": "a", "title": "A", "outputSchema": {}}]});
        let notes = downgrade_result("tools/list", V2025_06_18, V2025_03_26, &mut result).unwrap();
        assert_eq!(result["tools"][0].get("title"), None);
        assert_eq!(result["tools"][0].get("outputSchema"), None);
        assert_eq!(result["tools"][0]["name"], json!("a"));
        assert_eq!(notes.len(), 2);
    }

    #[test]
    fn icons_are_stripped_for_a_client_predating_them() {
        let mut result = json!({
            "tools": [{"name": "a", "icons": [{"src": "data:,x"}]}],
        });
        let notes = downgrade_result("tools/list", V2025_11_25, V2025_06_18, &mut result).unwrap();
        assert_eq!(result["tools"][0].get("icons"), None);
        assert_eq!(result["tools"][0]["name"], json!("a"));
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].field, "icons");
    }

    #[test]
    fn a_crossing_spanning_two_revisions_walks_every_step() {
        let mut result = json!({
            "tools": [{
                "name": "a",
                "title": "A",
                "icons": [{"src": "data:,x"}],
                "outputSchema": {},
            }],
        });
        let notes = downgrade_result("tools/list", V2025_11_25, V2025_03_26, &mut result).unwrap();
        let tool = &result["tools"][0];
        assert_eq!(tool.get("icons"), None);
        assert_eq!(tool.get("title"), None);
        assert_eq!(tool.get("outputSchema"), None);
        assert_eq!(tool["name"], json!("a"));
        assert_eq!(notes.len(), 3);
    }

    #[test]
    fn same_revision_is_never_rewritten() {
        let mut result = json!({"tools": [{"name": "a", "title": "A", "icons": []}]});
        let notes = downgrade_result("tools/list", V2025_11_25, V2025_11_25, &mut result).unwrap();
        assert!(notes.is_empty());
        assert_eq!(result["tools"][0]["title"], json!("A"));
        assert!(result["tools"][0].get("icons").is_some());
    }

    #[test]
    fn older_upstream_results_reach_newer_clients_untouched() {
        let mut result = json!({"tools": [{"name": "a"}]});
        let notes = downgrade_result("tools/list", V2025_03_26, V2025_11_25, &mut result).unwrap();
        assert!(notes.is_empty());
        assert_eq!(result["tools"][0]["name"], json!("a"));

        let mut result = json!({"tools": [{"name": "a"}]});
        let notes =
            downgrade_result("tools/list", v("2024-11-05"), V2025_06_18, &mut result).unwrap();
        assert!(notes.is_empty());
    }

    #[test]
    fn structured_content_downgrades_only_with_a_textual_mirror() {
        let mut mirrored = json!({
            "content": [{"type": "text", "text": "{\"ok\":true}"}],
            "structuredContent": {"ok": true},
        });
        let notes =
            downgrade_result("tools/call", V2025_06_18, V2025_03_26, &mut mirrored).unwrap();
        assert_eq!(mirrored.get("structuredContent"), None);
        assert_eq!(mirrored["content"][0]["text"], json!("{\"ok\":true}"));
        assert_eq!(notes.len(), 1);

        let mut bare = json!({"content": [], "structuredContent": {"ok": true}});
        let error = downgrade_result("tools/call", V2025_06_18, V2025_03_26, &mut bare)
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
        let error = downgrade_result("tools/call", V2025_06_18, V2025_03_26, &mut result)
            .expect_err("resource links have no 2025-03-26 equivalent");
        assert!(matches!(
            error,
            CompatError::UnrepresentableResourceLink { .. }
        ));
    }

    #[test]
    fn completion_context_is_refused_only_when_the_upstream_link_predates_it() {
        let request = json!({"params": {"context": {"arguments": {}}}});

        adapt_request("completion/complete", V2025_03_26, &request)
            .expect_err("context has no 2025-03-26 representation");

        // A client newer than the upstream link must not be refused over a field that upstream
        // revision has defined since 2025-06-18.
        adapt_request("completion/complete", V2025_06_18, &request)
            .expect("an upstream revision that defines context accepts it unchanged");
        adapt_request("completion/complete", V2025_11_25, &request)
            .expect("newer upstream revisions still define context");
    }
}
