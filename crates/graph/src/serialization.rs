use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::model::*;
use crate::Visibility;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SerializationError {
    #[error("invalid node ID format: {0}")]
    InvalidNodeId(String),

    #[error("unknown node kind: {0}")]
    UnknownNodeKind(String),

    #[error("unknown relationship kind: {0}")]
    UnknownRelationshipKind(String),

    #[error("evidence type mismatch: expected {expected}, got {actual}")]
    EvidenceMismatch { expected: String, actual: String },

    #[error("serialization error: {0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// Serializable evidence for a graph element.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "evidence_type")]
pub enum EvidenceDto {
    Source(SourceEvidenceDto),
    Structural(StructuralEvidenceDto),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceEvidenceDto {
    pub source_file: String,
    pub node_kind: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum StructuralEvidenceDto {
    Repository { root: String },
    Workspace { name: String },
    File { path: String },
}

/// Serializable graph node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeDto {
    /// String form of [`GraphNodeId`] (e.g. `"structural:..."` / `"entity:..."`).
    pub id: String,
    pub kind: String,
    pub name: String,
    pub visibility: Option<String>,
    pub documentation: Option<String>,
    pub evidence: EvidenceDto,
}

/// Serializable relationship.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelationshipDto {
    pub source: String,
    pub target: String,
    pub kind: String,
    pub evidence: EvidenceDto,
}

// ---------------------------------------------------------------------------
// Converting KnowledgeGraph → DTOs
// ---------------------------------------------------------------------------

pub fn graph_to_dtos(graph: &KnowledgeGraph) -> (Vec<NodeDto>, Vec<RelationshipDto>) {
    let nodes: Vec<NodeDto> = graph.nodes().iter().map(node_to_dto).collect();
    let relationships: Vec<RelationshipDto> =
        graph.relationships().iter().map(rel_to_dto).collect();
    (nodes, relationships)
}

fn node_to_dto(node: &Node) -> NodeDto {
    let visibility = node.metadata().visibility.as_ref().map(|v| v.to_string());
    let documentation = node.metadata().documentation.clone();

    NodeDto {
        id: node.id().to_string(),
        kind: node.kind().to_string(),
        name: node.name().to_string(),
        visibility,
        documentation,
        evidence: evidence_to_dto(node.evidence()),
    }
}

fn rel_to_dto(rel: &Relationship) -> RelationshipDto {
    RelationshipDto {
        source: rel.source().to_string(),
        target: rel.target().to_string(),
        kind: rel.kind().to_string(),
        evidence: evidence_to_dto(rel.evidence()),
    }
}

fn evidence_to_dto(evidence: &GraphEvidence) -> EvidenceDto {
    match evidence {
        GraphEvidence::Source(src) => EvidenceDto::Source(SourceEvidenceDto {
            source_file: src.source_file().to_string_lossy().to_string(),
            node_kind: src.node_kind().to_string(),
            byte_start: src.byte_range().start,
            byte_end: src.byte_range().end,
            start_line: src.start_line(),
            start_column: src.start_column(),
            end_line: src.end_line(),
            end_column: src.end_column(),
            language: src.language().to_string(),
        }),
        GraphEvidence::Structural(sev) => match sev {
            StructuralEvidence::Repository { root } => {
                EvidenceDto::Structural(StructuralEvidenceDto::Repository {
                    root: root.to_string_lossy().to_string(),
                })
            }
            StructuralEvidence::Workspace { name } => {
                EvidenceDto::Structural(StructuralEvidenceDto::Workspace { name: name.clone() })
            }
            StructuralEvidence::File { path } => {
                EvidenceDto::Structural(StructuralEvidenceDto::File {
                    path: path.to_string_lossy().to_string(),
                })
            }
        },
    }
}

// ---------------------------------------------------------------------------
// Converting DTOs → KnowledgeGraph
// ---------------------------------------------------------------------------

pub fn dtos_to_graph(
    nodes: Vec<NodeDto>,
    relationships: Vec<RelationshipDto>,
) -> Result<KnowledgeGraph, SerializationError> {
    let graph_nodes: Vec<Node> = nodes
        .into_iter()
        .map(dto_to_node)
        .collect::<Result<Vec<_>, _>>()?;

    let graph_rels: Vec<Relationship> = relationships
        .into_iter()
        .map(dto_to_relationship)
        .collect::<Result<Vec<_>, _>>()?;

    KnowledgeGraph::from_nodes_and_relationships(graph_nodes, graph_rels)
        .map_err(|errors| SerializationError::Other(format!("graph validation errors: {errors:?}")))
}

fn dto_to_node(dto: NodeDto) -> Result<Node, SerializationError> {
    let id = GraphNodeId::from_str(&dto.id).map_err(SerializationError::InvalidNodeId)?;
    let kind =
        NodeKind::from_str(&dto.kind).map_err(SerializationError::UnknownNodeKind)?;
    let visibility = dto
        .visibility
        .as_deref()
        .map(Visibility::from_str)
        .transpose()
        .map_err(SerializationError::Other)?;
    let metadata = NodeMetadata::new(visibility, dto.documentation);

    match id {
        GraphNodeId::Structural(struct_id) => {
            let ev = dto_to_structural_evidence(&dto.evidence)?;
            Ok(Node::structural(struct_id, kind, dto.name, metadata, ev))
        }
        GraphNodeId::Entity(entity_id) => {
            let ev = dto_to_source_evidence(&dto.evidence)?;
            Ok(Node::entity(entity_id, kind, dto.name, metadata, ev))
        }
    }
}

fn dto_to_relationship(dto: RelationshipDto) -> Result<Relationship, SerializationError> {
    let source =
        GraphNodeId::from_str(&dto.source).map_err(SerializationError::InvalidNodeId)?;
    let target =
        GraphNodeId::from_str(&dto.target).map_err(SerializationError::InvalidNodeId)?;
    let kind = RelationshipKind::from_str(&dto.kind)
        .map_err(SerializationError::UnknownRelationshipKind)?;

    match &dto.evidence {
        EvidenceDto::Structural(_) => {
            let ev = dto_to_structural_evidence(&dto.evidence)?;
            Ok(Relationship::structural(
                source,
                target,
                kind,
                RelationshipMetadata::new(),
                ev,
            ))
        }
        EvidenceDto::Source(_) => {
            let ev = dto_to_source_evidence(&dto.evidence)?;
            Ok(Relationship::with_source(
                source,
                target,
                kind,
                RelationshipMetadata::new(),
                ev,
            ))
        }
    }
}

fn dto_to_structural_evidence(dto: &EvidenceDto) -> Result<StructuralEvidence, SerializationError> {
    match dto {
        EvidenceDto::Structural(sev) => match sev {
            StructuralEvidenceDto::Repository { root } => Ok(StructuralEvidence::Repository {
                root: PathBuf::from(root),
            }),
            StructuralEvidenceDto::Workspace { name } => {
                Ok(StructuralEvidence::Workspace { name: name.clone() })
            }
            StructuralEvidenceDto::File { path } => Ok(StructuralEvidence::File {
                path: PathBuf::from(path),
            }),
        },
        EvidenceDto::Source(_) => Err(SerializationError::EvidenceMismatch {
            expected: "structural".into(),
            actual: "source".into(),
        }),
    }
}

fn dto_to_source_evidence(
    dto: &EvidenceDto,
) -> Result<kode_analysis::extraction::Evidence, SerializationError> {
    match dto {
        EvidenceDto::Source(src) => {
            let language = kode_acquisition::Language::from_str(&src.language)
                .map_err(|e| SerializationError::Other(format!("invalid language: {e}")))?;
            Ok(kode_analysis::extraction::Evidence::new(
                PathBuf::from(&src.source_file),
                &src.node_kind,
                src.byte_start..src.byte_end,
                src.start_line,
                src.start_column,
                src.end_line,
                src.end_column,
                language,
            ))
        }
        EvidenceDto::Structural(_) => Err(SerializationError::EvidenceMismatch {
            expected: "source".into(),
            actual: "structural".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kode_acquisition::Language;
    use kode_analysis::extraction::{EntityId, Evidence, Visibility};
    use std::collections::BTreeMap;
    use std::path::Path;

    fn make_test_graph() -> KnowledgeGraph {
        let repo_id = StructuralNodeId::from_parts(
            StructuralNodeKind::Repository,
            Path::new("/test"),
            "test-repo",
        );
        let ws_id = StructuralNodeId::from_parts(
            StructuralNodeKind::Workspace,
            Path::new("/test"),
            "test-ws",
        );
        let file_id = StructuralNodeId::from_parts(
            StructuralNodeKind::File,
            Path::new("src/lib.rs"),
            "src/lib.rs",
        );

        let entity_id = EntityId::from_location(
            &Language::Rust,
            "function",
            Path::new("src/lib.rs"),
            "hello",
            42,
        );

        let repo = Node::structural(
            repo_id,
            NodeKind::Repository,
            "test-repo",
            NodeMetadata::new(None, None),
            StructuralEvidence::Repository {
                root: PathBuf::from("/test"),
            },
        );
        let ws = Node::structural(
            ws_id,
            NodeKind::Workspace,
            "test-ws",
            NodeMetadata::new(None, None),
            StructuralEvidence::Workspace {
                name: "test-ws".into(),
            },
        );
        let file = Node::structural(
            file_id,
            NodeKind::File,
            "src/lib.rs",
            NodeMetadata::new(None, None),
            StructuralEvidence::File {
                path: PathBuf::from("src/lib.rs"),
            },
        );
        let func = Node::entity(
            entity_id,
            NodeKind::Function,
            "hello",
            NodeMetadata::new(Some(Visibility::Public), Some("Docs".into())),
            Evidence::new(
                PathBuf::from("src/lib.rs"),
                "function_item",
                42..100,
                3,
                5,
                7,
                20,
                Language::Rust,
            ),
        );

        let nodes = vec![repo, ws, file, func];
        let mut node_by_id: BTreeMap<GraphNodeId, usize> = BTreeMap::new();
        for (i, n) in nodes.iter().enumerate() {
            node_by_id.insert(*n.id(), i);
        }

        let rels = vec![
            Relationship::structural(
                GraphNodeId::Structural(repo_id),
                GraphNodeId::Structural(ws_id),
                RelationshipKind::Contains,
                RelationshipMetadata::new(),
                StructuralEvidence::Workspace {
                    name: "test-ws".into(),
                },
            ),
            Relationship::structural(
                GraphNodeId::Structural(ws_id),
                GraphNodeId::Structural(file_id),
                RelationshipKind::Contains,
                RelationshipMetadata::new(),
                StructuralEvidence::File {
                    path: PathBuf::from("src/lib.rs"),
                },
            ),
            Relationship::with_source(
                GraphNodeId::Structural(file_id),
                GraphNodeId::Entity(entity_id),
                RelationshipKind::Declares,
                RelationshipMetadata::new(),
                Evidence::new(
                    PathBuf::from("src/lib.rs"),
                    "function_item",
                    42..100,
                    3,
                    5,
                    7,
                    20,
                    Language::Rust,
                ),
            ),
        ];

        let n = nodes.len();
        let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut incoming: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (ri, r) in rels.iter().enumerate() {
            if let Some(&idx) = node_by_id.get(r.source()) {
                outgoing[idx].push(ri);
            }
            if let Some(&idx) = node_by_id.get(r.target()) {
                incoming[idx].push(ri);
            }
        }

        KnowledgeGraph::new(nodes, rels, node_by_id, outgoing, incoming)
    }

    #[test]
    fn serialization_determinism() {
        let g1 = make_test_graph();
        let g2 = make_test_graph();

        let (n1, r1) = graph_to_dtos(&g1);
        let (n2, r2) = graph_to_dtos(&g2);

        assert_eq!(n1, n2, "node DTOs must be identical for same graph");
        assert_eq!(r1, r2, "relationship DTOs must be identical for same graph");
    }

    #[test]
    fn roundtrip_preserves_graph() {
        let original = make_test_graph();

        let (nodes, rels) = graph_to_dtos(&original);
        let restored = dtos_to_graph(nodes, rels).unwrap();

        assert_eq!(original.node_count(), restored.node_count());
        assert_eq!(original.relationship_count(), restored.relationship_count());

        for node in original.nodes() {
            let restored_node = restored.node_by_id(node.id()).unwrap();
            assert_eq!(node.kind(), restored_node.kind());
            assert_eq!(node.name(), restored_node.name());
            assert_eq!(node.metadata(), restored_node.metadata());
            assert_eq!(node.evidence(), restored_node.evidence());
        }

        for rel in original.relationships() {
            let found = restored.relationships().iter().find(|r| {
                r.source() == rel.source() && r.target() == rel.target() && r.kind() == rel.kind()
            });
            assert!(found.is_some(), "relationship not found after roundtrip");
            if let Some(found) = found {
                assert_eq!(rel.evidence(), found.evidence());
            }
        }
    }

    #[test]
    fn serialization_json_determinism() {
        let g = make_test_graph();
        let (nodes, rels) = graph_to_dtos(&g);

        let json1 = serde_json::to_string_pretty(&nodes).unwrap();
        let json2 = serde_json::to_string_pretty(&nodes).unwrap();
        assert_eq!(json1, json2);

        let json1 = serde_json::to_string_pretty(&rels).unwrap();
        let json2 = serde_json::to_string_pretty(&rels).unwrap();
        assert_eq!(json1, json2);
    }

    #[test]
    fn empty_graph_roundtrip() {
        let g = KnowledgeGraph::from_nodes_and_relationships(Vec::new(), Vec::new()).unwrap();
        let (nodes, rels) = graph_to_dtos(&g);
        let restored = dtos_to_graph(nodes, rels).unwrap();
        assert_eq!(restored.node_count(), 0);
        assert_eq!(restored.relationship_count(), 0);
    }

    #[test]
    fn structural_node_identity_preserved() {
        let g = make_test_graph();
        let (nodes, _) = graph_to_dtos(&g);

        let repo_dto = nodes.iter().find(|n| n.kind == "repository").unwrap();
        assert!(repo_dto.id.starts_with("structural:"));

        let parsed = GraphNodeId::from_str(&repo_dto.id).unwrap();
        assert!(matches!(parsed, GraphNodeId::Structural(_)));
    }

    #[test]
    fn entity_node_identity_preserved() {
        let g = make_test_graph();
        let (nodes, _) = graph_to_dtos(&g);

        let func_dto = nodes.iter().find(|n| n.kind == "function").unwrap();
        assert!(func_dto.id.starts_with("entity:"));

        let parsed = GraphNodeId::from_str(&func_dto.id).unwrap();
        assert!(matches!(parsed, GraphNodeId::Entity(_)));
    }
}
