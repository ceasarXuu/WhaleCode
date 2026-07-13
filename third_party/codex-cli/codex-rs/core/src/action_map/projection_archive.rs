#![allow(dead_code)]

use codex_protocol::protocol::ActionMapSnapshotEdge;
use codex_protocol::protocol::ActionMapSnapshotMap;
use codex_protocol::protocol::ActionMapSnapshotNode;
use codex_protocol::protocol::ActionMapSnapshotNodeEvent;
use codex_protocol::protocol::ActionMapSnapshotResult;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::collections::HashSet;

const ARCHIVE_SCHEMA_VERSION: &str = "TaskSpaceProjectionArchiveV1";
const OUTPUT_REF_PREFIX: &str = "output-ref://sha256/";
pub(super) const S1_MINIMUM_NODE_COUNT: usize = 3;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct ProjectionArchivePayload {
    schema_version: String,
    pub(super) map_id: String,
    pub(super) nodes: Vec<ActionMapSnapshotNode>,
    pub(super) internal_edges: Vec<ActionMapSnapshotEdge>,
    pub(super) entry_edges: Vec<ActionMapSnapshotEdge>,
    pub(super) exit_edges: Vec<ActionMapSnapshotEdge>,
    pub(super) results: Vec<ActionMapSnapshotResult>,
    pub(super) node_events: Vec<ActionMapSnapshotNodeEvent>,
}

impl ProjectionArchivePayload {
    pub(super) fn new(
        map_id: String,
        nodes: Vec<ActionMapSnapshotNode>,
        internal_edges: Vec<ActionMapSnapshotEdge>,
        entry_edges: Vec<ActionMapSnapshotEdge>,
        exit_edges: Vec<ActionMapSnapshotEdge>,
        results: Vec<ActionMapSnapshotResult>,
        node_events: Vec<ActionMapSnapshotNodeEvent>,
    ) -> Self {
        Self {
            schema_version: ARCHIVE_SCHEMA_VERSION.to_string(),
            map_id,
            nodes,
            internal_edges,
            entry_edges,
            exit_edges,
            results,
            node_events,
        }
    }

    fn normalize(&mut self) {
        self.nodes.sort_by(|left, right| left.id.cmp(&right.id));
        self.internal_edges.sort_by(edge_order);
        self.entry_edges.sort_by(edge_order);
        self.exit_edges.sort_by(edge_order);
        self.results.sort_by(|left, right| left.id.cmp(&right.id));
        self.node_events
            .sort_by(|left, right| left.id.cmp(&right.id));
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema_version != ARCHIVE_SCHEMA_VERSION {
            return Err("projection archive schema version mismatch".to_string());
        }
        if self.map_id.trim().is_empty() {
            return Err("projection archive requires map_id".to_string());
        }
        if self.nodes.is_empty() {
            return Err("projection archive requires at least one node".to_string());
        }
        let node_ids = unique_ids(self.nodes.iter().map(|node| node.id.as_str()), "node")?;
        unique_ids(
            self.results.iter().map(|result| result.id.as_str()),
            "result",
        )?;
        unique_ids(
            self.node_events.iter().map(|event| event.id.as_str()),
            "node event",
        )?;
        validate_edges(&self.internal_edges, &node_ids, EdgeClass::Internal)?;
        validate_edges(&self.entry_edges, &node_ids, EdgeClass::Entry)?;
        validate_edges(&self.exit_edges, &node_ids, EdgeClass::Exit)?;
        for result in &self.results {
            if result.map_id != self.map_id || !node_ids.contains(result.node_id.as_str()) {
                return Err(format!(
                    "archive result {} has invalid ownership",
                    result.id
                ));
            }
        }
        for event in &self.node_events {
            if event.map_id != self.map_id || !node_ids.contains(event.node_id.as_str()) {
                return Err(format!(
                    "archive node event {} has invalid ownership",
                    event.id
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EncodedProjectionArchive {
    pub(super) archive_ref: String,
    pub(super) payload_sha256: String,
    pub(super) covered_node_ids_sha256: String,
    pub(super) bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectionArchiveCandidate {
    pub(super) encoded: EncodedProjectionArchive,
    pub(super) covered_node_ids: Vec<String>,
    pub(super) result_refs: Vec<String>,
}

pub(super) fn build_s1_projection_archive(
    map: &ActionMapSnapshotMap,
    current_node_id: Option<&str>,
    covered_node_ids: &[String],
) -> Result<ProjectionArchiveCandidate, String> {
    if covered_node_ids.len() < S1_MINIMUM_NODE_COUNT {
        return Err(format!(
            "S1 projection archive requires at least {S1_MINIMUM_NODE_COUNT} nodes"
        ));
    }
    let covered = unique_ids(covered_node_ids.iter().map(String::as_str), "covered node")?;
    let nodes = map
        .nodes
        .iter()
        .filter(|node| covered.contains(node.id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if nodes.len() != covered.len() {
        return Err("S1 projection archive references a missing canonical node".to_string());
    }
    for node in &nodes {
        if node.status != "completed"
            || node.active_lease.is_some()
            || current_node_id == Some(node.id.as_str())
            || map.leases.iter().any(|lease| lease.node_id == node.id)
        {
            return Err(format!("node {} is not eligible for S1 archive", node.id));
        }
    }
    if let Some(edge) = map
        .edges
        .iter()
        .find(|edge| covered.contains(edge.from.as_str()) || covered.contains(edge.to.as_str()))
    {
        return Err(format!(
            "node incident to edge {}->{} is not eligible for S1 archive",
            edge.from, edge.to
        ));
    }
    let results = map
        .results
        .iter()
        .filter(|result| covered.contains(result.node_id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let result_refs = results.iter().map(|result| result.id.clone()).collect();
    let node_events = map
        .node_events
        .iter()
        .filter(|event| covered.contains(event.node_id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let encoded = encode_projection_archive(ProjectionArchivePayload::new(
        map.id.clone(),
        nodes,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        results,
        node_events,
    ))?;
    let mut covered_node_ids = covered_node_ids.to_vec();
    covered_node_ids.sort();
    Ok(ProjectionArchiveCandidate {
        encoded,
        covered_node_ids,
        result_refs,
    })
}

pub(super) fn encode_projection_archive(
    mut payload: ProjectionArchivePayload,
) -> Result<EncodedProjectionArchive, String> {
    payload.normalize();
    payload.validate()?;
    let bytes = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
    let payload_sha256 = sha256_hex(&bytes);
    let covered_node_ids_sha256 = sha256_hex(
        payload
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>()
            .join("\n")
            .as_bytes(),
    );
    Ok(EncodedProjectionArchive {
        archive_ref: format!("{OUTPUT_REF_PREFIX}{payload_sha256}"),
        payload_sha256,
        covered_node_ids_sha256,
        bytes,
    })
}

pub(super) fn decode_projection_archive(
    archive_ref: &str,
    bytes: &[u8],
) -> Result<ProjectionArchivePayload, String> {
    let expected_sha256 = parse_archive_ref(archive_ref)?;
    let actual_sha256 = sha256_hex(bytes);
    if actual_sha256 != expected_sha256 {
        return Err(format!(
            "projection archive hash mismatch: expected {expected_sha256}, got {actual_sha256}"
        ));
    }
    let mut payload: ProjectionArchivePayload =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    payload.normalize();
    payload.validate()?;
    let canonical_bytes = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
    if canonical_bytes != bytes {
        return Err("projection archive payload is not canonically ordered".to_string());
    }
    Ok(payload)
}

fn parse_archive_ref(archive_ref: &str) -> Result<&str, String> {
    let sha256 = archive_ref.strip_prefix(OUTPUT_REF_PREFIX).ok_or_else(|| {
        "projection archive ref must use output-ref://sha256/<sha256>".to_string()
    })?;
    if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("projection archive ref requires 64 hexadecimal characters".to_string());
    }
    Ok(sha256)
}

fn unique_ids<'a>(
    ids: impl Iterator<Item = &'a str>,
    label: &str,
) -> Result<HashSet<&'a str>, String> {
    let mut unique = HashSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(format!("projection archive {label} id must not be empty"));
        }
        if !unique.insert(id) {
            return Err(format!(
                "projection archive contains duplicate {label} id {id}"
            ));
        }
    }
    Ok(unique)
}

#[derive(Clone, Copy)]
enum EdgeClass {
    Internal,
    Entry,
    Exit,
}

fn validate_edges(
    edges: &[ActionMapSnapshotEdge],
    node_ids: &HashSet<&str>,
    class: EdgeClass,
) -> Result<(), String> {
    let mut unique = HashSet::new();
    for edge in edges {
        if !unique.insert((edge.from.as_str(), edge.to.as_str())) {
            return Err(format!(
                "projection archive contains duplicate edge {}->{}",
                edge.from, edge.to
            ));
        }
        let from_inside = node_ids.contains(edge.from.as_str());
        let to_inside = node_ids.contains(edge.to.as_str());
        let valid = match class {
            EdgeClass::Internal => from_inside && to_inside,
            EdgeClass::Entry => !from_inside && to_inside,
            EdgeClass::Exit => from_inside && !to_inside,
        };
        if !valid {
            return Err(format!(
                "projection archive edge {}->{} has invalid boundary class",
                edge.from, edge.to
            ));
        }
    }
    Ok(())
}

fn edge_order(left: &ActionMapSnapshotEdge, right: &ActionMapSnapshotEdge) -> std::cmp::Ordering {
    left.from
        .cmp(&right.from)
        .then_with(|| left.to.cmp(&right.to))
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(index: usize) -> ActionMapSnapshotNode {
        ActionMapSnapshotNode {
            id: format!("node-{index:05}"),
            title: format!("Node {index}"),
            kind: "custom".to_string(),
            canonical_kind: "custom".to_string(),
            status: "completed".to_string(),
            context_summary: format!("Goal {index}"),
            source_refs: vec![format!("task-event-{index}")],
            active_lease: None,
            result_ids: Vec::new(),
            node_event_ids: Vec::new(),
            origin_node_id: None,
        }
    }

    fn payload(node_count: usize) -> ProjectionArchivePayload {
        ProjectionArchivePayload::new(
            "map-1".to_string(),
            (0..node_count).rev().map(node).collect(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }

    #[test]
    fn archive_codec_is_deterministic_and_reversible_at_scale() {
        for node_count in [100, 1_000, 10_000] {
            let encoded = encode_projection_archive(payload(node_count)).expect("encode archive");
            let decoded = decode_projection_archive(&encoded.archive_ref, &encoded.bytes)
                .expect("decode archive");
            let reencoded = encode_projection_archive(decoded).expect("re-encode archive");
            assert_eq!(reencoded.archive_ref, encoded.archive_ref);
            assert_eq!(reencoded.bytes, encoded.bytes);
            assert_eq!(reencoded.payload_sha256, encoded.payload_sha256);
            assert_eq!(
                reencoded.covered_node_ids_sha256,
                encoded.covered_node_ids_sha256
            );
        }
    }

    #[test]
    fn archive_codec_rejects_empty_duplicate_and_corrupt_payloads() {
        let empty = encode_projection_archive(payload(0)).expect_err("empty archive must fail");
        assert!(empty.contains("at least one node"));

        let mut duplicate = payload(2);
        duplicate.nodes[1].id = duplicate.nodes[0].id.clone();
        let duplicate = encode_projection_archive(duplicate).expect_err("duplicate must fail");
        assert!(duplicate.contains("duplicate node id"));

        let encoded = encode_projection_archive(payload(3)).expect("encode archive");
        let mut corrupt = encoded.bytes.clone();
        corrupt[0] ^= 1;
        let corrupt = decode_projection_archive(&encoded.archive_ref, &corrupt)
            .expect_err("hash mismatch must fail");
        assert!(corrupt.contains("hash mismatch"));
    }

    #[test]
    fn archive_codec_rejects_invalid_boundary_edges() {
        let mut invalid = payload(2);
        invalid.entry_edges.push(ActionMapSnapshotEdge {
            from: "node-00000".to_string(),
            to: "node-00001".to_string(),
        });
        let error = encode_projection_archive(invalid).expect_err("invalid entry edge must fail");
        assert!(error.contains("invalid boundary class"));
    }
}
