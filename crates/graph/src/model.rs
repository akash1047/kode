//! Domain model for the knowledge graph.
//!
//! Defines [`Node`], [`Relationship`], [`NodeKind`], [`RelationshipKind`],
//! and the immutable [`KnowledgeGraph`] that owns them.
//!
//! # Identity Model
//!
//! The graph distinguishes between two kinds of identity:
//!
//! * [`GraphNodeId::Entity`] — wraps an extracted entity [`EntityId`]
//! * [`GraphNodeId::Structural`] — identifies a synthetic graph node
//!   (repository, workspace, file) that has no corresponding extracted entity
//!
//! Structural and entity identities live in separate namespaces and cannot
//! collide.
//!
//! # Evidence Model
//!
//! Every graph element is backed by evidence:
//!
//! * [`GraphEvidence::Source`] — parser-produced evidence from Stage 3
//! * [`GraphEvidence::Structural`] — evidence for synthetic graph elements

use std::collections::BTreeMap;

use kode_analysis::extraction::{EntityId, Evidence, Visibility};
use kode_common::hash::Fnv1aHasher;

// ---------------------------------------------------------------------------
// Graph-level identity
// ---------------------------------------------------------------------------

/// Stable identifier for a graph node.
///
/// Two mutually exclusive variants:
///
/// * [`Structural`](GraphNodeId::Structural) — for synthetic graph nodes
///   (repository, workspace, file)
/// * [`Entity`](GraphNodeId::Entity) — wraps an extracted entity's
///   [`EntityId`]
///
/// # Ordering
///
/// Structural IDs sort before Entity IDs. Within each variant, ordering
/// follows the inner type's [`Ord`] implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GraphNodeId {
    Structural(StructuralNodeId),
    Entity(EntityId),
}

impl GraphNodeId {
    /// Return the inner u64 value regardless of variant.
    pub fn as_u64(&self) -> u64 {
        match self {
            GraphNodeId::Structural(id) => id.0,
            GraphNodeId::Entity(id) => id.as_u64(),
        }
    }
}

impl std::fmt::Display for GraphNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphNodeId::Structural(id) => write!(f, "structural:{:016x}", id.0),
            GraphNodeId::Entity(id) => write!(f, "entity:{id}"),
        }
    }
}

impl std::str::FromStr for GraphNodeId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(hex) = s.strip_prefix("structural:") {
            let v = u64::from_str_radix(hex, 16)
                .map_err(|_| format!("invalid structural node ID: {s}"))?;
            Ok(GraphNodeId::Structural(StructuralNodeId::from_u64(v)))
        } else if let Some(hex) = s.strip_prefix("entity:") {
            let v =
                u64::from_str_radix(hex, 16).map_err(|_| format!("invalid entity node ID: {s}"))?;
            Ok(GraphNodeId::Entity(EntityId::from_u64(v)))
        } else {
            Err(format!("invalid graph node ID: {s}"))
        }
    }
}

/// Discriminator for synthetic structural node kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StructuralNodeKind {
    Repository,
    Workspace,
    File,
}

impl StructuralNodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            StructuralNodeKind::Repository => "repository",
            StructuralNodeKind::Workspace => "workspace",
            StructuralNodeKind::File => "file",
        }
    }
}

/// Identity for a synthetic structural graph node.
///
/// Computed from the node kind (e.g. `"repository"`, `"workspace"`, `"file"`),
/// an optional path, and an optional name. Deterministic and stable across
/// runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StructuralNodeId(u64);

impl StructuralNodeId {
    /// Create a deterministic structural node ID from its constituent parts.
    ///
    /// Uses the same hashing scheme as [`EntityId`] but within a separate
    /// namespace prefixed by the string `"structural"` to guarantee
    /// namespace separation.
    pub fn from_parts(kind: StructuralNodeKind, path: &std::path::Path, name: &str) -> Self {
        use std::hash::{Hash, Hasher};
        let mut hasher = Fnv1aHasher::new();
        "structural".hash(&mut hasher);
        kind.as_str().hash(&mut hasher);
        path.hash(&mut hasher);
        name.hash(&mut hasher);
        Self(hasher.finish())
    }

    /// Return the raw u64 hash value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Create a `StructuralNodeId` from a raw u64 value.
    ///
    /// # Safety
    ///
    /// The caller must ensure the value was produced by
    /// [`StructuralNodeId::from_parts`] or [`StructuralNodeId::as_u64`].
    /// This constructor does not recompute the hash.
    pub fn from_u64(value: u64) -> Self {
        Self(value)
    }
}

// ---------------------------------------------------------------------------
// Graph-level evidence
// ---------------------------------------------------------------------------

/// Evidence attached to a graph element.
///
/// Every node and relationship must be backed by evidence. This type
/// distinguishes between:
///
/// * [`Source`](GraphEvidence::Source) — evidence produced by a parser or
///   extractor, carrying full source location information
/// * [`Structural`](GraphEvidence::Structural) — structured evidence for
///   synthetic graph elements (see [`StructuralEvidence`])
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphEvidence {
    Source(Evidence),
    Structural(StructuralEvidence),
}

impl GraphEvidence {
    /// Human-readable description of the evidence.
    pub fn description(&self) -> String {
        match self {
            GraphEvidence::Source(ev) => ev.node_kind().to_string(),
            GraphEvidence::Structural(sev) => sev.description(),
        }
    }
}

/// Structured evidence for synthetic graph elements.
///
/// Unlike [`GraphEvidence::Source`] which carries parser-produced locations,
/// structural evidence carries the actual metadata that justifies the
/// element's existence. The human-readable description is derived from this
/// data rather than stored as a free-form string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuralEvidence {
    /// The repository root node.
    Repository {
        /// Path of the repository root.
        root: std::path::PathBuf,
    },
    /// A workspace within the repository.
    Workspace {
        /// Name of the workspace.
        name: String,
    },
    /// A source file containing extracted entities.
    File {
        /// Path of the file relative to the repository root.
        path: std::path::PathBuf,
    },
}

impl StructuralEvidence {
    /// Derive a human-readable description from the structured data.
    pub fn description(&self) -> String {
        match self {
            StructuralEvidence::Repository { root } => {
                format!("repository root: {}", root.display())
            }
            StructuralEvidence::Workspace { name } => format!("workspace: {name}"),
            StructuralEvidence::File { path } => format!("file: {}", path.display()),
        }
    }
}

// ---------------------------------------------------------------------------
// Node kind
// ---------------------------------------------------------------------------

/// Kind of a graph node.
///
/// Structural nodes (repository, workspace, file) are synthetic graph-level
/// constructs. All other variants correspond one-to-one with extracted entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeKind {
    // --- Structural nodes ---
    Repository,
    Workspace,
    File,

    // --- Extracted entity kinds ---
    Module,
    Function,
    Struct,
    Enum,
    Trait,
    ImplBlock,
    TypeAlias,
    Constant,
    Static,
    Import,
    Export,
}

impl NodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeKind::Repository => "repository",
            NodeKind::Workspace => "workspace",
            NodeKind::File => "file",
            NodeKind::Module => "module",
            NodeKind::Function => "function",
            NodeKind::Struct => "struct",
            NodeKind::Enum => "enum",
            NodeKind::Trait => "trait",
            NodeKind::ImplBlock => "impl_block",
            NodeKind::TypeAlias => "type_alias",
            NodeKind::Constant => "constant",
            NodeKind::Static => "static",
            NodeKind::Import => "import",
            NodeKind::Export => "export",
        }
    }

    /// Returns `true` if this kind is a synthetic structural node.
    pub fn is_structural(&self) -> bool {
        matches!(
            self,
            NodeKind::Repository | NodeKind::Workspace | NodeKind::File
        )
    }
}

impl std::fmt::Display for NodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for NodeKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "repository" => Ok(NodeKind::Repository),
            "workspace" => Ok(NodeKind::Workspace),
            "file" => Ok(NodeKind::File),
            "module" => Ok(NodeKind::Module),
            "function" => Ok(NodeKind::Function),
            "struct" => Ok(NodeKind::Struct),
            "enum" => Ok(NodeKind::Enum),
            "trait" => Ok(NodeKind::Trait),
            "impl_block" => Ok(NodeKind::ImplBlock),
            "type_alias" => Ok(NodeKind::TypeAlias),
            "constant" => Ok(NodeKind::Constant),
            "static" => Ok(NodeKind::Static),
            "import" => Ok(NodeKind::Import),
            "export" => Ok(NodeKind::Export),
            other => Err(format!("unknown node kind: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Relationship kind
// ---------------------------------------------------------------------------

/// Kind of a relationship between two graph nodes.
///
/// Only relationships directly derivable from extracted facts are modelled.
/// Semantic relationships (e.g. `calls`, `depends_on`) belong to later
/// analysis stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RelationshipKind {
    /// Structural containment (repository → workspace, workspace → file).
    Contains,
    /// A file declares an extracted entity.
    Declares,
    /// An entity defines a sub-entity (e.g. trait → method).
    Defines,
}

impl RelationshipKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationshipKind::Contains => "contains",
            RelationshipKind::Declares => "declares",
            RelationshipKind::Defines => "defines",
        }
    }
}

impl std::fmt::Display for RelationshipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for RelationshipKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "contains" => Ok(RelationshipKind::Contains),
            "declares" => Ok(RelationshipKind::Declares),
            "defines" => Ok(RelationshipKind::Defines),
            other => Err(format!("unknown relationship kind: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

/// Extensible metadata attached to a graph node.
///
/// Only metadata that is common across all node kinds lives here.
/// Kind-specific data (e.g. struct fields) can be added as new fields
/// without changing the public API shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeMetadata {
    /// Source-level visibility, if the entity has one.
    pub visibility: Option<Visibility>,
    /// Documentation comment attached to the source entity, if any.
    pub documentation: Option<String>,
}

impl NodeMetadata {
    pub fn new(visibility: Option<Visibility>, documentation: Option<String>) -> Self {
        Self {
            visibility,
            documentation,
        }
    }
}

/// Extensible metadata attached to a relationship.
///
/// Currently empty; reserved for future qualifiers such as label or weight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipMetadata;

impl RelationshipMetadata {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RelationshipMetadata {
    fn default() -> Self {
        Self
    }
}

// ---------------------------------------------------------------------------
// Node
// ---------------------------------------------------------------------------

/// A single node in the knowledge graph.
///
/// Every node carries a [`GraphNodeId`] (its stable identity), a [`NodeKind`]
/// discriminator, a human-readable name, optional metadata, and
/// [`GraphEvidence`] — no graph element exists without evidence.
///
/// Structural nodes (repository, workspace, file) carry
/// [`GraphNodeId::Structural`] and [`GraphEvidence::Structural`].
/// Entity nodes carry [`GraphNodeId::Entity`] and [`GraphEvidence::Source`].
#[derive(Debug, Clone)]
pub struct Node {
    id: GraphNodeId,
    kind: NodeKind,
    name: String,
    metadata: NodeMetadata,
    evidence: GraphEvidence,
}

impl Node {
    /// Create a structural node (repository, workspace, file).
    ///
    /// Guarantees at the type level that the node carries a
    /// [`GraphNodeId::Structural`] and [`GraphEvidence::Structural`].
    pub fn structural(
        id: StructuralNodeId,
        kind: NodeKind,
        name: impl Into<String>,
        metadata: NodeMetadata,
        evidence: StructuralEvidence,
    ) -> Self {
        Self {
            id: GraphNodeId::Structural(id),
            kind,
            name: name.into(),
            metadata,
            evidence: GraphEvidence::Structural(evidence),
        }
    }

    /// Create an entity node (function, struct, enum, etc.).
    ///
    /// Guarantees at the type level that the node carries a
    /// [`GraphNodeId::Entity`] and [`GraphEvidence::Source`].
    pub fn entity(
        id: kode_analysis::extraction::EntityId,
        kind: NodeKind,
        name: impl Into<String>,
        metadata: NodeMetadata,
        evidence: kode_analysis::extraction::Evidence,
    ) -> Self {
        Self {
            id: GraphNodeId::Entity(id),
            kind,
            name: name.into(),
            metadata,
            evidence: GraphEvidence::Source(evidence),
        }
    }

    pub fn id(&self) -> &GraphNodeId {
        &self.id
    }

    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn metadata(&self) -> &NodeMetadata {
        &self.metadata
    }

    pub fn evidence(&self) -> &GraphEvidence {
        &self.evidence
    }
}

// ---------------------------------------------------------------------------
// Relationship
// ---------------------------------------------------------------------------

/// A directed relationship between two graph nodes.
///
/// Every relationship carries a source and target [`GraphNodeId`], a
/// [`RelationshipKind`], optional metadata, and
/// [`GraphEvidence`] — no graph element exists without evidence.
#[derive(Debug, Clone)]
pub struct Relationship {
    source: GraphNodeId,
    target: GraphNodeId,
    kind: RelationshipKind,
    metadata: RelationshipMetadata,
    evidence: GraphEvidence,
}

impl Relationship {
    /// Create a relationship backed by structural evidence.
    pub fn structural(
        source: GraphNodeId,
        target: GraphNodeId,
        kind: RelationshipKind,
        metadata: RelationshipMetadata,
        evidence: StructuralEvidence,
    ) -> Self {
        Self {
            source,
            target,
            kind,
            metadata,
            evidence: GraphEvidence::Structural(evidence),
        }
    }

    /// Create a relationship backed by parser-produced source evidence.
    pub fn with_source(
        source: GraphNodeId,
        target: GraphNodeId,
        kind: RelationshipKind,
        metadata: RelationshipMetadata,
        evidence: kode_analysis::extraction::Evidence,
    ) -> Self {
        Self {
            source,
            target,
            kind,
            metadata,
            evidence: GraphEvidence::Source(evidence),
        }
    }

    pub fn source(&self) -> &GraphNodeId {
        &self.source
    }

    pub fn target(&self) -> &GraphNodeId {
        &self.target
    }

    pub fn kind(&self) -> RelationshipKind {
        self.kind
    }

    pub fn metadata(&self) -> &RelationshipMetadata {
        &self.metadata
    }

    pub fn evidence(&self) -> &GraphEvidence {
        &self.evidence
    }
}

// ---------------------------------------------------------------------------
// Graph version
// ---------------------------------------------------------------------------

/// Graph format version — independent of crate version or application version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphVersion {
    pub major: u16,
    pub minor: u16,
}

impl GraphVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// Current graph format version for newly constructed graphs.
    pub const CURRENT: GraphVersion = GraphVersion::new(1, 0);
}

impl std::fmt::Display for GraphVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

// ---------------------------------------------------------------------------
// KnowledgeGraph — immutable, validated in-memory graph
// ---------------------------------------------------------------------------

/// Canonical, immutable, validated knowledge graph for a repository.
///
/// # Construction
///
/// Graphs are produced exclusively by [`crate::GraphBuilder`] and are
/// immutable once constructed. Consumers receive read-only references.
///
/// # Invariants
///
/// * Every node has a unique [`GraphNodeId`].
/// * Every node carries [`GraphEvidence`].
/// * Every relationship carries [`GraphEvidence`].
/// * Relationship endpoints refer to existing node IDs.
/// * Iteration order is deterministic (sorted by [`GraphNodeId`]).
/// * Exactly one repository node exists.
/// * Exactly one workspace node exists.
/// * Entity nodes use [`GraphNodeId::Entity`] and [`GraphEvidence::Source`].
/// * Structural nodes use [`GraphNodeId::Structural`] and
///   [`GraphEvidence::Structural`].
///
/// # Lookup
///
/// [`KnowledgeGraph::node_by_id`] provides O(log n) lookup by identity.
/// [`KnowledgeGraph::nodes_by_kind`] filters by node kind.
///
/// # Traversal
///
/// [`KnowledgeGraph::outgoing`] and [`KnowledgeGraph::incoming`] return
/// relationships incident to a given node.
///
/// # Complexity
///
/// | Operation | Complexity |
/// |-----------|-----------|
/// | `node_by_id` | O(log n) |
/// | `nodes_by_kind` | O(n) |
/// | `outgoing` / `incoming` | O(1) per node (amortized) |
/// | `nodes` / `relationships` | O(1) slice view |
/// | `node_count` / `relationship_count` | O(1) |
#[derive(Debug, Clone)]
pub struct KnowledgeGraph {
    nodes: Vec<Node>,
    relationships: Vec<Relationship>,
    node_by_id: BTreeMap<GraphNodeId, usize>,
    outgoing: Vec<Vec<usize>>,
    incoming: Vec<Vec<usize>>,
}

impl KnowledgeGraph {
    #[doc(hidden)]
    pub fn new(
        nodes: Vec<Node>,
        relationships: Vec<Relationship>,
        node_by_id: BTreeMap<GraphNodeId, usize>,
        outgoing: Vec<Vec<usize>>,
        incoming: Vec<Vec<usize>>,
    ) -> Self {
        Self {
            nodes,
            relationships,
            node_by_id,
            outgoing,
            incoming,
        }
    }

    /// Construct a [`KnowledgeGraph`] from nodes and relationships,
    /// computing the index structures internally.
    ///
    /// Uses [`crate::builder::index::build_node_index`] and
    /// [`crate::builder::index::build_edge_index`] to reconstruct the
    /// lookup and traversal indexes.
    pub fn from_nodes_and_relationships(
        nodes: Vec<Node>,
        relationships: Vec<Relationship>,
    ) -> Result<Self, Vec<crate::validator::ValidationError>> {
        let node_by_id = crate::builder::index::build_node_index(&nodes)?;
        let n = nodes.len();
        let (outgoing, incoming) =
            crate::builder::index::build_edge_index(n, &relationships, &node_by_id);

        Ok(Self {
            nodes,
            relationships,
            node_by_id,
            outgoing,
            incoming,
        })
    }

    // ------------------------------------------------------------------
    // Lookup
    // ------------------------------------------------------------------

    /// Look up a node by its [`GraphNodeId`].
    ///
    /// Returns `None` if the node is not present.
    pub fn node_by_id(&self, id: &GraphNodeId) -> Option<&Node> {
        self.node_by_id.get(id).map(|&idx| &self.nodes[idx])
    }

    // ------------------------------------------------------------------
    // Iteration (deterministic order)
    // ------------------------------------------------------------------

    /// All nodes in deterministic order (sorted by [`GraphNodeId`]).
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// All relationships in deterministic order.
    pub fn relationships(&self) -> &[Relationship] {
        &self.relationships
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn relationship_count(&self) -> usize {
        self.relationships.len()
    }

    /// Iterate over all nodes with the given [`NodeKind`].
    ///
    /// Iteration follows the deterministic node ordering.
    pub fn nodes_by_kind(&self, kind: NodeKind) -> impl Iterator<Item = &Node> {
        self.nodes.iter().filter(move |n| n.kind() == kind)
    }

    // ------------------------------------------------------------------
    // Traversal
    // ------------------------------------------------------------------

    /// Relationships where the given node is the source (outgoing edges).
    ///
    /// Returns an empty iterator if the node is not in the graph.
    pub fn outgoing<'a>(&'a self, id: &'a GraphNodeId) -> impl Iterator<Item = &'a Relationship> {
        self.node_by_id.get(id).into_iter().flat_map(move |&idx| {
            let indices = &self.outgoing[idx];
            indices
                .iter()
                .map(move |&rel_idx| &self.relationships[rel_idx])
        })
    }

    /// Relationships where the given node is the target (incoming edges).
    ///
    /// Returns an empty iterator if the node is not in the graph.
    pub fn incoming<'a>(&'a self, id: &'a GraphNodeId) -> impl Iterator<Item = &'a Relationship> {
        self.node_by_id.get(id).into_iter().flat_map(move |&idx| {
            let indices = &self.incoming[idx];
            indices
                .iter()
                .map(move |&rel_idx| &self.relationships[rel_idx])
        })
    }
}
