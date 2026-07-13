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
        let file_id = structural
            .file_by_path
            .get(file_path)
            .expect("file path must have a corresponding structural node in the lookup");
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
        let file_id = structural
            .file_by_path
            .get(&entity_ref.source_file)
            .expect("entity source file must have a corresponding structural file node");
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

    // Function → Function (calls) — resolve call sites by callee name.
    relationships.extend(build_call_relationships(facts, node_by_id));

    // Import edges — best-effort resolve `use` path last segment to module/struct/function.
    relationships.extend(build_import_relationships(facts, node_by_id));

    // Sort deterministically
    relationships.sort_by(|a, b| {
        a.source()
            .cmp(b.source())
            .then_with(|| a.target().cmp(b.target()))
            .then_with(|| a.kind().cmp(&b.kind()))
    });

    relationships
}

/// Resolve import facts into `Imports` edges by matching the final path segment
/// to a module / type / function name in the graph.
fn build_import_relationships(
    facts: &RepositoryFacts,
    node_by_id: &BTreeMap<GraphNodeId, usize>,
) -> Vec<Relationship> {
    use std::collections::HashMap;

    let mut by_name: HashMap<&str, Vec<GraphNodeId>> = HashMap::new();
    for f in facts.functions() {
        by_name
            .entry(f.name())
            .or_default()
            .push(GraphNodeId::Entity(*f.id()));
    }
    for m in facts.modules() {
        by_name
            .entry(m.name())
            .or_default()
            .push(GraphNodeId::Entity(*m.id()));
    }
    for s in facts.structs() {
        by_name
            .entry(s.name())
            .or_default()
            .push(GraphNodeId::Entity(*s.id()));
    }
    for t in facts.traits() {
        by_name
            .entry(t.name())
            .or_default()
            .push(GraphNodeId::Entity(*t.id()));
    }
    for e in facts.enums() {
        by_name
            .entry(e.name())
            .or_default()
            .push(GraphNodeId::Entity(*e.id()));
    }

    let mut relationships = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    for imp in facts.imports() {
        let source_id = GraphNodeId::Entity(*imp.id());
        if !node_by_id.contains_key(&source_id) {
            continue;
        }
        let path = imp.path();
        let segment = path.rsplit("::").next().unwrap_or(path);
        // Strip trailing `*` for glob imports like `foo::*`.
        let segment = segment.trim_end_matches('*').trim_end_matches(':');
        if segment.is_empty() {
            continue;
        }
        let Some(targets) = by_name.get(segment) else {
            continue;
        };
        for target in targets {
            if !node_by_id.contains_key(target) {
                continue;
            }
            if !seen.insert((source_id, *target)) {
                continue;
            }
            relationships.push(Relationship::with_source(
                source_id,
                *target,
                RelationshipKind::Imports,
                RelationshipMetadata::new(),
                imp.evidence().clone(),
            ));
        }
    }

    relationships
}

/// Resolve extracted call sites into `Calls` edges.
///
/// Resolution is name-based (best-effort):
/// 1. Prefer unique repo-wide function match.
/// 2. Else prefer same-file matches.
/// 3. Else emit edges to every candidate with that name.
///
/// Unresolved names (e.g. external crates) produce no edge.
fn build_call_relationships(
    facts: &RepositoryFacts,
    node_by_id: &BTreeMap<GraphNodeId, usize>,
) -> Vec<Relationship> {
    use std::collections::HashMap;

    // Index functions by name for resolution.
    let mut by_name: HashMap<&str, Vec<(&kode_analysis::extraction::FunctionFact, GraphNodeId)>> =
        HashMap::new();
    for f in facts.functions() {
        let id = GraphNodeId::Entity(*f.id());
        if node_by_id.contains_key(&id) {
            by_name.entry(f.name()).or_default().push((f, id));
        }
    }

    let mut relationships = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    for caller in facts.functions() {
        let caller_id = GraphNodeId::Entity(*caller.id());
        if !node_by_id.contains_key(&caller_id) {
            continue;
        }
        let caller_file = caller.evidence().source_file();

        for site in caller.calls() {
            let Some(candidates) = by_name.get(site.callee_name()) else {
                continue;
            };

            let targets: Vec<GraphNodeId> = if candidates.len() == 1 {
                vec![candidates[0].1]
            } else {
                let same_file: Vec<_> = candidates
                    .iter()
                    .filter(|(f, _)| f.evidence().source_file() == caller_file)
                    .map(|(_, id)| *id)
                    .collect();
                if !same_file.is_empty() {
                    same_file
                } else {
                    candidates.iter().map(|(_, id)| *id).collect()
                }
            };

            for target_id in targets {
                let key = (caller_id, target_id);
                if !seen.insert(key) {
                    continue; // one edge per (caller, callee) pair
                }
                relationships.push(Relationship::with_source(
                    caller_id,
                    target_id,
                    RelationshipKind::Calls,
                    RelationshipMetadata::new(),
                    site.evidence().clone(),
                ));
            }
        }
    }

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
