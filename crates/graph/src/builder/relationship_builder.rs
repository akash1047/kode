//! Relationship construction between graph nodes.
//!
//! Derives deterministic relationships from extracted facts and structural
//! hierarchy information.
//!
//! # Relationship Derivation
//!
//! | Relationship | Source | Target | Evidence |
//! |---|---|---|---|
//! | Contains | Repository | Workspace | Structural |
//! | Contains | Workspace | File | Structural |
//! | Declares | File | Entity | Source (entity's) |
//! | Defines | Trait / ImplBlock | Function | Source (parent's) |
//!
//! # Invariants
//!
//! * Relationships only reference nodes that exist in the graph (verified
//!   by the caller during index construction).
//! * Relationship ordering is deterministic (sorted by source, target, kind).
//! * All entity-sourced relationships carry the entity's original evidence.
//! * Structural IDs are received via [`StructuralLookup`] — no independent
//!   hashing of structural identities occurs here.

use std::collections::BTreeMap;
use std::path::PathBuf;

use kode_analysis::extraction::RepositoryFacts;

use crate::model::{
    GraphNodeId, Relationship, RelationshipKind, RelationshipMetadata, StructuralEvidence,
};

use super::context::RepositoryContext;
use super::structural::StructuralLookup;

/// Flat reference to an entity during relationship derivation.
struct EntityRef {
    id: GraphNodeId,
    source_file: PathBuf,
    evidence: kode_analysis::extraction::Evidence,
}

/// Build all relationships for the graph.
///
/// Accepts pre-computed structural node identities via [`StructuralLookup`]
/// — no structural [`StructuralNodeId`] hashing occurs here.
pub fn build_relationships(
    facts: &RepositoryFacts,
    node_by_id: &BTreeMap<GraphNodeId, usize>,
    ctx: &RepositoryContext,
    structural: &StructuralLookup,
) -> Vec<Relationship> {
    let mut relationships = Vec::new();

    let repo_id = structural.repo_id;
    let workspace_id = structural.workspace_id;

    // Repository → Workspace (contains)
    relationships.push(Relationship::structural(
        repo_id,
        workspace_id,
        RelationshipKind::Contains,
        RelationshipMetadata::new(),
        StructuralEvidence::Repository {
            root: ctx.root_path().to_path_buf(),
        },
    ));

    // Workspace → File (contains)
    for file_path in ctx.source_files() {
        let file_id = structural.file_by_path.get(file_path).expect(
            "file path must have a corresponding structural node in the lookup",
        );
        relationships.push(Relationship::structural(
            workspace_id,
            *file_id,
            RelationshipKind::Contains,
            RelationshipMetadata::new(),
            StructuralEvidence::Workspace {
                name: ctx.workspace_name().to_string(),
            },
        ));
    }

    // File → Entity (declares)
    for entity_ref in entity_refs(facts) {
        let file_id = structural.file_by_path.get(&entity_ref.source_file).expect(
            "entity source file must have a corresponding structural file node",
        );
        relationships.push(Relationship::with_source(
            *file_id,
            entity_ref.id,
            RelationshipKind::Declares,
            RelationshipMetadata::new(),
            entity_ref.evidence.clone(),
        ));
    }

    // Trait → Function (defines)
    for trait_fact in facts.traits() {
        for method_id in trait_fact.methods() {
            let method_id = GraphNodeId::Entity(*method_id);
            if node_by_id.contains_key(&method_id) {
                relationships.push(Relationship::with_source(
                    GraphNodeId::Entity(*trait_fact.id()),
                    method_id,
                    RelationshipKind::Defines,
                    RelationshipMetadata::new(),
                    trait_fact.evidence().clone(),
                ));
            }
        }
    }

    // ImplBlock → Function (defines)
    for impl_fact in facts.impl_blocks() {
        for method_id in impl_fact.methods() {
            let method_id = GraphNodeId::Entity(*method_id);
            if node_by_id.contains_key(&method_id) {
                relationships.push(Relationship::with_source(
                    GraphNodeId::Entity(*impl_fact.id()),
                    method_id,
                    RelationshipKind::Defines,
                    RelationshipMetadata::new(),
                    impl_fact.evidence().clone(),
                ));
            }
        }
    }

    // Sort deterministically
    relationships.sort_by(|a, b| {
        a.source()
            .cmp(b.source())
            .then_with(|| a.target().cmp(b.target()))
            .then_with(|| a.kind().cmp(&b.kind()))
    });

    relationships
}

fn entity_refs(facts: &RepositoryFacts) -> Vec<EntityRef> {
    let mut refs = Vec::new();

    macro_rules! push_refs {
        ($accessor:ident) => {
            for entity in facts.$accessor() {
                refs.push(EntityRef {
                    id: GraphNodeId::Entity(*entity.id()),
                    source_file: entity.evidence().source_file().to_path_buf(),
                    evidence: entity.evidence().clone(),
                });
            }
        };
    }

    push_refs!(modules);
    push_refs!(functions);
    push_refs!(structs);
    push_refs!(enums);
    push_refs!(traits);
    push_refs!(impl_blocks);
    push_refs!(type_aliases);
    push_refs!(constants);
    push_refs!(statics);
    push_refs!(imports);
    push_refs!(exports);

    refs.sort_by_key(|a| a.id);
    refs
}
