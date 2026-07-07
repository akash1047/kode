//! Integration tests for the `kode-graph` crate.
#![allow(unused_crate_dependencies)]

use std::path::{Path, PathBuf};

use kode_acquisition::Language;
use kode_analysis::extraction::{
    ConstantFact, Entity, EntityId, EnumFact, Evidence, ExportFact, FunctionFact, ImplBlockFact,
    ImportFact, ModuleFact, RepositoryFacts, StaticFact, StructFact, TraitFact, TypeAliasFact,
    Visibility,
};
use kode_graph::{
    GraphBuilder, GraphEvidence, GraphNodeId, Node, NodeKind, RelationshipKind, RepositoryContext,
    StructuralEvidence, StructuralNodeId, StructuralNodeKind,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn evidence(path: &str, start: usize) -> Evidence {
    Evidence::new(
        PathBuf::from(path),
        "test",
        start..start + 5,
        1,
        1,
        1,
        6,
        Language::Rust,
    )
}

fn entity_id(path: &str, kind: &str, name: &str, offset: usize) -> EntityId {
    EntityId::from_location(&Language::Rust, kind, Path::new(path), name, offset)
}

fn graph_entity_id(path: &str, kind: &str, name: &str, offset: usize) -> GraphNodeId {
    GraphNodeId::Entity(EntityId::from_location(
        &Language::Rust,
        kind,
        Path::new(path),
        name,
        offset,
    ))
}

fn graph_file_id(path: &str) -> GraphNodeId {
    GraphNodeId::Structural(StructuralNodeId::from_parts(
        StructuralNodeKind::File,
        Path::new(path),
        "",
    ))
}

fn build_facts(entities: Vec<Entity>) -> RepositoryFacts {
    RepositoryFacts::from_entities(entities, Vec::new())
}

// ---------------------------------------------------------------------------
// Constructor invariant tests
// ---------------------------------------------------------------------------

#[test]
fn structural_node_constructor_enforces_evidence_type() {
    let node = Node::structural(
        StructuralNodeId::from_parts(StructuralNodeKind::Repository, Path::new(""), "repo"),
        NodeKind::Repository,
        "test",
        kode_graph::NodeMetadata::new(None, None),
        StructuralEvidence::Repository {
            root: PathBuf::from("/repo"),
        },
    );
    assert!(matches!(node.id(), GraphNodeId::Structural(_)));
    assert!(matches!(node.evidence(), GraphEvidence::Structural(_)));
}

#[test]
fn entity_node_constructor_enforces_evidence_type() {
    let ev = evidence("src/lib.rs", 0);
    let node = Node::entity(
        entity_id("src/lib.rs", "function", "foo", 0),
        NodeKind::Function,
        "foo",
        kode_graph::NodeMetadata::new(None, None),
        ev,
    );
    assert!(matches!(node.id(), GraphNodeId::Entity(_)));
    assert!(matches!(node.evidence(), GraphEvidence::Source(_)));
}

// ---------------------------------------------------------------------------
// Graph structure tests
// ---------------------------------------------------------------------------

#[test]
fn empty_facts_produces_graph_with_structural_nodes() {
    let facts = RepositoryFacts::empty();
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    // Repository + Workspace = 2 structural nodes
    assert_eq!(graph.node_count(), 2, "empty facts: expected 2 nodes");

    // 1 relationship: Repository → Workspace (contains)
    assert_eq!(
        graph.relationship_count(),
        1,
        "empty facts: expected 1 relationship"
    );

    // Verify structural node kinds
    let repos: Vec<_> = graph.nodes_by_kind(NodeKind::Repository).collect();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name(), "repository");

    let workspaces: Vec<_> = graph.nodes_by_kind(NodeKind::Workspace).collect();
    assert_eq!(workspaces.len(), 1);
    assert_eq!(workspaces[0].name(), "default");
}

#[test]
fn single_function_becomes_one_entity_node() {
    let ev = evidence("src/lib.rs", 0);
    let func = FunctionFact::new(
        entity_id("src/lib.rs", "function", "hello", 0),
        "hello",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev,
    );

    let facts = build_facts(vec![Entity::Function(func)]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    // 3 nodes: Repository + Workspace + File + Function = 4
    assert_eq!(graph.node_count(), 4);

    // 3 relationships:
    // Repository → Workspace (contains) = 1
    // Workspace → File (contains) = 1
    // File → Function (declares) = 1
    // Total = 3
    assert_eq!(graph.relationship_count(), 3);

    // Verify function node exists
    let func_graph_id = graph_entity_id("src/lib.rs", "function", "hello", 0);
    let func_node = graph.node_by_id(&func_graph_id).unwrap();
    assert_eq!(func_node.kind(), NodeKind::Function);
    assert_eq!(func_node.name(), "hello");

    // Verify file node exists
    let file_nodes: Vec<_> = graph.nodes_by_kind(NodeKind::File).collect();
    assert_eq!(file_nodes.len(), 1);
    assert!(file_nodes[0].name().contains("src/lib.rs"));

    // Verify declares relationship
    let file_graph_id = graph_file_id("src/lib.rs");
    let declares: Vec<_> = graph.outgoing(&file_graph_id).collect();
    assert_eq!(declares.len(), 1);
    assert_eq!(declares[0].kind(), RelationshipKind::Declares);
    assert_eq!(declares[0].target(), &func_graph_id);
}

#[test]
fn every_entity_kind_becomes_a_node() {
    let ev = |path: &str, offset: usize| evidence(path, offset);

    let module = ModuleFact::new(
        entity_id("lib.rs", "module", "mymod", 0),
        "mymod",
        Visibility::Private,
        ev("lib.rs", 0),
    );
    let function = FunctionFact::new(
        entity_id("lib.rs", "function", "foo", 10),
        "foo",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev("lib.rs", 10),
    );
    let struct_fact = StructFact::new(
        entity_id("lib.rs", "struct", "Bar", 20),
        "Bar",
        Vec::new(),
        Visibility::Public,
        Vec::new(),
        ev("lib.rs", 20),
    );
    let enum_fact = EnumFact::new(
        entity_id("lib.rs", "enum", "Baz", 30),
        "Baz",
        Vec::new(),
        Visibility::Public,
        ev("lib.rs", 30),
    );
    let trait_fact = TraitFact::new(
        entity_id("lib.rs", "trait", "Qux", 40),
        "Qux",
        Visibility::Public,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        ev("lib.rs", 40),
    );
    let impl_fact = ImplBlockFact::new(
        entity_id("lib.rs", "impl", "Bar", 50),
        "Bar",
        None,
        Vec::new(),
        ev("lib.rs", 50),
    );
    let type_alias = TypeAliasFact::new(
        entity_id("lib.rs", "type_alias", "MyType", 60),
        "MyType",
        Some("u32".to_string()),
        ev("lib.rs", 60),
    );
    let constant = ConstantFact::new(
        entity_id("lib.rs", "constant", "MAX", 70),
        "MAX",
        Some("usize".to_string()),
        Some("100".to_string()),
        ev("lib.rs", 70),
    );
    let static_fact = StaticFact::new(
        entity_id("lib.rs", "static", "GLOBAL", 80),
        "GLOBAL",
        Some("u32".to_string()),
        false,
        ev("lib.rs", 80),
    );
    let import = ImportFact::new(
        entity_id("lib.rs", "import", "std::collections", 90),
        "std::collections",
        None,
        false,
        ev("lib.rs", 90),
    );
    let export = ExportFact::new(
        entity_id("lib.rs", "export", "foo", 100),
        "foo",
        None,
        false,
        ev("lib.rs", 100),
    );

    let facts = build_facts(vec![
        Entity::Module(module),
        Entity::Function(function),
        Entity::Struct(struct_fact),
        Entity::Enum(enum_fact),
        Entity::Trait(trait_fact),
        Entity::ImplBlock(impl_fact),
        Entity::TypeAlias(type_alias),
        Entity::Constant(constant),
        Entity::Static(static_fact),
        Entity::Import(import),
        Entity::Export(export),
    ]);
    let ctx = RepositoryContext::from_facts(&facts);

    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    // 2 structural (repo, workspace) + 1 file + 11 entity = 14 nodes
    assert_eq!(graph.node_count(), 14);

    // Verify each entity kind is present
    assert_eq!(
        graph.nodes_by_kind(NodeKind::Module).count(),
        1,
        "expected 1 module"
    );
    assert_eq!(
        graph.nodes_by_kind(NodeKind::Function).count(),
        1,
        "expected 1 function"
    );
    assert_eq!(
        graph.nodes_by_kind(NodeKind::Struct).count(),
        1,
        "expected 1 struct"
    );
    assert_eq!(
        graph.nodes_by_kind(NodeKind::Enum).count(),
        1,
        "expected 1 enum"
    );
    assert_eq!(
        graph.nodes_by_kind(NodeKind::Trait).count(),
        1,
        "expected 1 trait"
    );
    assert_eq!(
        graph.nodes_by_kind(NodeKind::ImplBlock).count(),
        1,
        "expected 1 impl block"
    );
    assert_eq!(
        graph.nodes_by_kind(NodeKind::TypeAlias).count(),
        1,
        "expected 1 type alias"
    );
    assert_eq!(
        graph.nodes_by_kind(NodeKind::Constant).count(),
        1,
        "expected 1 constant"
    );
    assert_eq!(
        graph.nodes_by_kind(NodeKind::Static).count(),
        1,
        "expected 1 static"
    );
    assert_eq!(
        graph.nodes_by_kind(NodeKind::Import).count(),
        1,
        "expected 1 import"
    );
    assert_eq!(
        graph.nodes_by_kind(NodeKind::Export).count(),
        1,
        "expected 1 export"
    );
}

// ---------------------------------------------------------------------------
// Determinism tests
// ---------------------------------------------------------------------------

#[test]
fn deterministic_across_runs() {
    let ev = |path: &str, offset: usize| evidence(path, offset);

    let f1 = FunctionFact::new(
        entity_id("a.rs", "function", "bbb", 0),
        "bbb",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev("a.rs", 0),
    );
    let f2 = FunctionFact::new(
        entity_id("b.rs", "function", "aaa", 0),
        "aaa",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev("b.rs", 0),
    );

    let entities = vec![Entity::Function(f1), Entity::Function(f2)];
    let facts1 = build_facts(entities.clone());
    let facts2 = build_facts(entities);

    let ctx1 = RepositoryContext::from_facts(&facts1);
    let ctx2 = RepositoryContext::from_facts(&facts2);
    let graph1 = GraphBuilder::build(&facts1, &ctx1).unwrap();
    let graph2 = GraphBuilder::build(&facts2, &ctx2).unwrap();

    // Check node order
    let names1: Vec<&str> = graph1.nodes().iter().map(|n| n.name()).collect();
    let names2: Vec<&str> = graph2.nodes().iter().map(|n| n.name()).collect();
    assert_eq!(names1, names2, "node ordering must be deterministic");

    // Check relationship order
    let rel_targets1: Vec<_> = graph1.relationships().iter().map(|r| *r.target()).collect();
    let rel_targets2: Vec<_> = graph2.relationships().iter().map(|r| *r.target()).collect();
    assert_eq!(
        rel_targets1, rel_targets2,
        "relationship ordering must be deterministic"
    );
}

// ---------------------------------------------------------------------------
// Evidence preservation tests
// ---------------------------------------------------------------------------

#[test]
fn every_node_preserves_evidence() {
    let ev = evidence("src/main.rs", 42);
    let func = FunctionFact::new(
        entity_id("src/main.rs", "function", "foo", 42),
        "foo",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev,
    );

    let facts = build_facts(vec![Entity::Function(func)]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    for node in graph.nodes() {
        match node.evidence() {
            GraphEvidence::Source(ev) => {
                assert!(
                    !ev.source_file().as_os_str().is_empty(),
                    "every source-evidenced node must have a source file"
                );
            }
            GraphEvidence::Structural(_) => {
                // Structural evidence does not have source files; this is valid.
            }
        }
    }
}

#[test]
fn entity_evidence_preserved_through_graph() {
    let ev = evidence("src/lib.rs", 0);
    let func = FunctionFact::new(
        entity_id("src/lib.rs", "function", "preserved", 0),
        "preserved",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev.clone(),
    );

    let facts = build_facts(vec![Entity::Function(func)]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    let node_id = graph_entity_id("src/lib.rs", "function", "preserved", 0);
    let node = graph.node_by_id(&node_id).unwrap();
    match node.evidence() {
        GraphEvidence::Source(ev) => {
            assert_eq!(ev.source_file(), Path::new("src/lib.rs"));
            assert_eq!(ev.start_line(), 1);
        }
        GraphEvidence::Structural(_) => {
            panic!("entity node must have source evidence");
        }
    }
}

#[test]
fn structural_nodes_have_structural_evidence() {
    let facts = RepositoryFacts::empty();
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    for node in graph.nodes() {
        assert!(
            matches!(node.evidence(), GraphEvidence::Structural(_)),
            "structural node '{}' must have structural evidence, got {:?}",
            node.name(),
            node.evidence()
        );
    }
}

#[test]
fn structural_nodes_have_structural_identity() {
    let facts = RepositoryFacts::empty();
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    for node in graph.nodes() {
        assert!(
            matches!(node.id(), GraphNodeId::Structural(_)),
            "structural node '{}' must have structural identity",
            node.name()
        );
    }
}

// ---------------------------------------------------------------------------
// Structural node tests
// ---------------------------------------------------------------------------

#[test]
fn repository_graph_always_contains_required_roots() {
    let facts = RepositoryFacts::empty();
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    let repo_count = graph.nodes_by_kind(NodeKind::Repository).count();
    let ws_count = graph.nodes_by_kind(NodeKind::Workspace).count();

    assert_eq!(repo_count, 1, "must always have a repository node");
    assert_eq!(ws_count, 1, "must always have a workspace node");
}

#[test]
fn file_nodes_created_for_every_unique_source_file() {
    let ev1 = evidence("src/a.rs", 0);
    let ev2 = evidence("src/b.rs", 0);

    let f1 = FunctionFact::new(
        entity_id("src/a.rs", "function", "f1", 0),
        "f1",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev1,
    );
    let f2 = FunctionFact::new(
        entity_id("src/b.rs", "function", "f2", 0),
        "f2",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev2,
    );

    let facts = build_facts(vec![Entity::Function(f1), Entity::Function(f2)]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    let file_nodes: Vec<_> = graph.nodes_by_kind(NodeKind::File).collect();
    assert_eq!(file_nodes.len(), 2);
}

// ---------------------------------------------------------------------------
// Relationship tests
// ---------------------------------------------------------------------------

#[test]
fn trait_defines_method_relationship() {
    let method_ev = evidence("src/lib.rs", 0);
    let method = FunctionFact::new(
        entity_id("src/lib.rs", "function", "do_thing", 0),
        "do_thing",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        method_ev,
    );

    let method_id = *method.id();
    let method_graph_id = GraphNodeId::Entity(method_id);
    let trait_ev = evidence("src/lib.rs", 50);
    let trait_fact = TraitFact::new(
        entity_id("src/lib.rs", "trait", "MyTrait", 50),
        "MyTrait",
        Visibility::Public,
        vec![method_id],
        Vec::new(),
        Vec::new(),
        trait_ev,
    );

    let facts = build_facts(vec![Entity::Trait(trait_fact), Entity::Function(method)]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    let trait_graph_id = graph_entity_id("src/lib.rs", "trait", "MyTrait", 50);
    let defines: Vec<_> = graph.outgoing(&trait_graph_id).collect();

    let method_defines: Vec<_> = defines
        .iter()
        .filter(|r| r.kind() == RelationshipKind::Defines)
        .collect();

    assert_eq!(method_defines.len(), 1);
    assert_eq!(method_defines[0].target(), &method_graph_id);
}

#[test]
fn impl_block_defines_method_relationship() {
    let method_ev = evidence("src/lib.rs", 0);
    let method = FunctionFact::new(
        entity_id("src/lib.rs", "function", "new", 0),
        "new",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        method_ev,
    );

    let method_id = *method.id();
    let method_graph_id = GraphNodeId::Entity(method_id);
    let impl_ev = evidence("src/lib.rs", 30);
    let impl_fact = ImplBlockFact::new(
        entity_id("src/lib.rs", "impl", "Foo", 30),
        "Foo",
        None,
        vec![method_id],
        impl_ev,
    );

    let facts = build_facts(vec![Entity::ImplBlock(impl_fact), Entity::Function(method)]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    let impl_graph_id = graph_entity_id("src/lib.rs", "impl", "Foo", 30);
    let defines: Vec<_> = graph.outgoing(&impl_graph_id).collect();
    let method_defines: Vec<_> = defines
        .iter()
        .filter(|r| r.kind() == RelationshipKind::Defines)
        .collect();

    assert_eq!(method_defines.len(), 1);
    assert_eq!(method_defines[0].target(), &method_graph_id);
}

// ---------------------------------------------------------------------------
// Validation tests
// ---------------------------------------------------------------------------

#[test]
fn duplicate_node_ids_rejected() {
    let ev = evidence("same.rs", 0);
    let f1 = FunctionFact::new(
        entity_id("same.rs", "function", "collide", 0),
        "collide",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev.clone(),
    );
    let f2 = FunctionFact::new(
        entity_id("same.rs", "function", "collide", 0),
        "collide",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev,
    );

    let facts = build_facts(vec![Entity::Function(f1), Entity::Function(f2)]);
    let ctx = RepositoryContext::from_facts(&facts);
    let result = GraphBuilder::build(&facts, &ctx);
    assert!(result.is_err(), "duplicate node IDs must be rejected");
}

// ---------------------------------------------------------------------------
// Traversal tests
// ---------------------------------------------------------------------------

#[test]
fn outgoing_returns_relationships_from_node() {
    let ev = evidence("src/lib.rs", 0);
    let func = FunctionFact::new(
        entity_id("src/lib.rs", "function", "check_outgoing", 0),
        "check_outgoing",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev,
    );

    let facts = build_facts(vec![Entity::Function(func)]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    // Repository node should have 1 outgoing (→ Workspace)
    let repo_id = GraphNodeId::Structural(StructuralNodeId::from_parts(
        StructuralNodeKind::Repository,
        Path::new(""),
        "repo",
    ));
    let outgoing: Vec<_> = graph.outgoing(&repo_id).collect();
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].kind(), RelationshipKind::Contains);
}

#[test]
fn incoming_returns_relationships_to_node() {
    let ev = evidence("src/lib.rs", 0);
    let func = FunctionFact::new(
        entity_id("src/lib.rs", "function", "check_incoming", 0),
        "check_incoming",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev,
    );

    let facts = build_facts(vec![Entity::Function(func)]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    // Function node should have 1 incoming (File declares Function)
    let func_graph_id = graph_entity_id("src/lib.rs", "function", "check_incoming", 0);
    let incoming: Vec<_> = graph.incoming(&func_graph_id).collect();
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].kind(), RelationshipKind::Declares);

    // File node should have 1 incoming (Workspace contains File)
    let file_graph_id = graph_file_id("src/lib.rs");
    let file_incoming: Vec<_> = graph.incoming(&file_graph_id).collect();
    assert_eq!(file_incoming.len(), 1);
    assert_eq!(file_incoming[0].kind(), RelationshipKind::Contains);
}

#[test]
fn lookup_by_id_returns_correct_node() {
    let ev = evidence("src/lib.rs", 0);
    let func = FunctionFact::new(
        entity_id("src/lib.rs", "function", "lookup_test", 0),
        "lookup_test",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev,
    );

    let facts = build_facts(vec![Entity::Function(func)]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    let func_graph_id = graph_entity_id("src/lib.rs", "function", "lookup_test", 0);
    let node = graph.node_by_id(&func_graph_id);
    assert!(node.is_some());
    assert_eq!(node.unwrap().name(), "lookup_test");

    // Non-existent ID returns None
    let fake_id = graph_entity_id("nope.rs", "function", "ghost", 0);
    assert!(graph.node_by_id(&fake_id).is_none());
}

#[test]
fn unknown_node_id_returns_empty_traversal() {
    let facts = RepositoryFacts::empty();
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    let fake_id = graph_entity_id("nonexistent.rs", "function", "nobody", 0);
    assert_eq!(graph.outgoing(&fake_id).count(), 0);
    assert_eq!(graph.incoming(&fake_id).count(), 0);
}

// ---------------------------------------------------------------------------
// Edge case tests
// ---------------------------------------------------------------------------

#[test]
fn single_file_multiple_entities() {
    let ev = |offset: usize| evidence("src/lib.rs", offset);

    let f1 = FunctionFact::new(
        entity_id("src/lib.rs", "function", "a", 0),
        "a",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev(0),
    );
    let f2 = FunctionFact::new(
        entity_id("src/lib.rs", "function", "b", 10),
        "b",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev(10),
    );
    let s = StructFact::new(
        entity_id("src/lib.rs", "struct", "S", 20),
        "S",
        Vec::new(),
        Visibility::Public,
        Vec::new(),
        ev(20),
    );

    let facts = build_facts(vec![
        Entity::Function(f1),
        Entity::Function(f2),
        Entity::Struct(s),
    ]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    // 2 structural + 1 file + 3 entity = 6 nodes
    assert_eq!(graph.node_count(), 6);

    // 1 (repo → ws) + 1 (ws → file) + 3 (file → entities) = 5 relationships
    assert_eq!(graph.relationship_count(), 5);

    // File node should have 3 outgoing declares relationships
    let file_graph_id = graph_file_id("src/lib.rs");
    let declares: Vec<_> = graph
        .outgoing(&file_graph_id)
        .filter(|r| r.kind() == RelationshipKind::Declares)
        .collect();
    assert_eq!(declares.len(), 3);
}

#[test]
fn multiple_files_isolated() {
    let ev1 = evidence("a.rs", 0);
    let ev2 = evidence("b.rs", 0);

    let f1 = FunctionFact::new(
        entity_id("a.rs", "function", "in_a", 0),
        "in_a",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev1,
    );
    let f2 = FunctionFact::new(
        entity_id("b.rs", "function", "in_b", 0),
        "in_b",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev2,
    );

    let facts = build_facts(vec![Entity::Function(f1), Entity::Function(f2)]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    let file_nodes: Vec<_> = graph.nodes_by_kind(NodeKind::File).collect();
    assert_eq!(file_nodes.len(), 2);

    // Each file has exactly 1 declares relationship
    for file_node in &file_nodes {
        let file_id = file_node.id();
        let outgoing: Vec<_> = graph.outgoing(file_id).collect();
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].kind(), RelationshipKind::Declares);
    }
}

#[test]
fn nodes_sorted_by_graph_node_id() {
    let ev = |offset: usize| evidence("src/lib.rs", offset);

    // Create entities with names that sort differently from IDs
    let f_a = FunctionFact::new(
        entity_id("src/lib.rs", "function", "a", 100),
        "a",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev(100),
    );
    let f_b = FunctionFact::new(
        entity_id("src/lib.rs", "function", "b", 0),
        "b",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev(0),
    );

    let facts = build_facts(vec![Entity::Function(f_a), Entity::Function(f_b)]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    let nodes = graph.nodes();
    for pair in nodes.windows(2) {
        assert!(
            pair[0].id() <= pair[1].id(),
            "nodes must be sorted by GraphNodeId"
        );
    }
}

#[test]
fn relationship_contains_repo_to_workspace() {
    let facts = RepositoryFacts::empty();
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    let repo_id = GraphNodeId::Structural(StructuralNodeId::from_parts(
        StructuralNodeKind::Repository,
        Path::new(""),
        "repo",
    ));
    let ws_id = GraphNodeId::Structural(StructuralNodeId::from_parts(
        StructuralNodeKind::Workspace,
        Path::new(""),
        "default",
    ));

    let contains: Vec<_> = graph
        .relationships()
        .iter()
        .filter(|r| r.kind() == RelationshipKind::Contains)
        .collect();

    assert_eq!(contains.len(), 1);
    assert_eq!(contains[0].source(), &repo_id);
    assert_eq!(contains[0].target(), &ws_id);
}

#[test]
fn metadata_preserved_for_function_node() {
    let ev = evidence("src/lib.rs", 0);
    let func = FunctionFact::new(
        entity_id("src/lib.rs", "function", "meta_test", 0),
        "meta_test",
        Visibility::Crate,
        Some("fn meta_test()".to_string()),
        Vec::new(),
        true,
        false,
        false,
        Some("This is a doc comment".to_string()),
        None,
        None,
        ev,
    );

    let facts = build_facts(vec![Entity::Function(func)]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    let func_graph_id = graph_entity_id("src/lib.rs", "function", "meta_test", 0);
    let node = graph.node_by_id(&func_graph_id).unwrap();

    assert_eq!(node.metadata().visibility, Some(Visibility::Crate));
    assert_eq!(
        node.metadata().documentation.as_deref(),
        Some("This is a doc comment")
    );
}

// ---------------------------------------------------------------------------
// Structural identity tests
// ---------------------------------------------------------------------------

#[test]
fn structural_ids_cannot_collide_with_entity_ids() {
    // Structural IDs use a separate hash namespace from Entity IDs.
    // Verify that Structural("file", "src/lib.rs", "") produces a different
    // ID than Entity would from an equivalent from_location call.
    let structural =
        StructuralNodeId::from_parts(StructuralNodeKind::File, Path::new("src/lib.rs"), "");
    let entity = EntityId::from_location(&Language::Rust, "file", Path::new("src/lib.rs"), "", 0);

    assert_ne!(
        structural.as_u64(),
        entity.as_u64(),
        "structural and entity IDs must not collide"
    );
}

// ---------------------------------------------------------------------------
// Structural identity stability tests
// ---------------------------------------------------------------------------

#[test]
fn structural_node_ids_are_deterministic() {
    let id1 = StructuralNodeId::from_parts(StructuralNodeKind::Repository, Path::new(""), "repo");
    let id2 = StructuralNodeId::from_parts(StructuralNodeKind::Repository, Path::new(""), "repo");
    assert_eq!(id1, id2, "structural node IDs must be deterministic");
}

#[test]
fn different_structural_nodes_have_different_ids() {
    let repo = StructuralNodeId::from_parts(StructuralNodeKind::Repository, Path::new(""), "repo");
    let ws = StructuralNodeId::from_parts(StructuralNodeKind::Workspace, Path::new(""), "default");
    assert_ne!(
        repo, ws,
        "different structural nodes must have different IDs"
    );
}

// ---------------------------------------------------------------------------
// Validator tests
// ---------------------------------------------------------------------------

#[test]
fn structural_roots_validated() {
    let facts = RepositoryFacts::empty();
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();
    // Valid graph: exactly one repo and one workspace
    assert_eq!(graph.nodes_by_kind(NodeKind::Repository).count(), 1);
    assert_eq!(graph.nodes_by_kind(NodeKind::Workspace).count(), 1);
}

// ---------------------------------------------------------------------------
// Regression tests
// ---------------------------------------------------------------------------

#[test]
fn empty_repository_produces_only_structural_graph() {
    let facts = RepositoryFacts::empty();
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    // Only structural nodes (repository, workspace)
    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.relationship_count(), 1);

    // All nodes are structural
    for node in graph.nodes() {
        assert!(node.kind().is_structural());
    }

    // All evidence is structural
    for node in graph.nodes() {
        assert!(matches!(node.evidence(), GraphEvidence::Structural(_)));
    }
}

#[test]
fn no_synthetic_rust_evidence_for_structural_nodes() {
    let facts = RepositoryFacts::empty();
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    for node in graph.nodes() {
        match node.evidence() {
            GraphEvidence::Source(ev) => {
                panic!(
                    "structural node '{}' has parser evidence: {:?}",
                    node.name(),
                    ev
                );
            }
            GraphEvidence::Structural(_) => {
                // Expected
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Regression: identity hashing
// ---------------------------------------------------------------------------

#[test]
fn structural_identity_deterministic_across_construction() {
    let id1 = StructuralNodeId::from_parts(StructuralNodeKind::Repository, Path::new(""), "repo");
    let id2 = StructuralNodeId::from_parts(StructuralNodeKind::Repository, Path::new(""), "repo");
    assert_eq!(
        id1, id2,
        "identical inputs must produce identical structural IDs"
    );
}

#[test]
fn structural_identity_differs_for_different_kinds() {
    let repo = StructuralNodeId::from_parts(StructuralNodeKind::Repository, Path::new(""), "repo");
    let file = StructuralNodeId::from_parts(StructuralNodeKind::File, Path::new(""), "repo");
    assert_ne!(repo, file, "different kinds must produce different IDs");
}

#[test]
fn structural_identity_differs_for_different_paths() {
    let a = StructuralNodeId::from_parts(StructuralNodeKind::File, Path::new("src/a.rs"), "");
    let b = StructuralNodeId::from_parts(StructuralNodeKind::File, Path::new("src/b.rs"), "");
    assert_ne!(a, b, "different paths must produce different IDs");
}

#[test]
fn structural_identity_differs_for_different_names() {
    let a = StructuralNodeId::from_parts(StructuralNodeKind::Workspace, Path::new(""), "alpha");
    let b = StructuralNodeId::from_parts(StructuralNodeKind::Workspace, Path::new(""), "beta");
    assert_ne!(a, b, "different names must produce different IDs");
}

#[test]
fn structural_and_entity_namespaces_remain_disjoint() {
    // Structural IDs are prefixed with "structural"; Entity IDs are not.
    // Even with identical (kind, path, name, offset) inputs, the IDs
    // must never collide.
    let structural =
        StructuralNodeId::from_parts(StructuralNodeKind::File, Path::new("src/lib.rs"), "");
    let entity = EntityId::from_location(&Language::Rust, "file", Path::new("src/lib.rs"), "", 0);

    assert_ne!(
        structural.as_u64(),
        entity.as_u64(),
        "structural and entity namespaces must remain disjoint"
    );
}

// ---------------------------------------------------------------------------
// Regression: structural evidence
// ---------------------------------------------------------------------------

#[test]
fn structural_evidence_contains_repository_root() {
    let facts = build_facts(vec![]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    let repo: Vec<_> = graph.nodes_by_kind(NodeKind::Repository).collect();
    assert_eq!(repo.len(), 1);

    match repo[0].evidence() {
        GraphEvidence::Structural(StructuralEvidence::Repository { root }) => {
            assert!(
                !root.as_os_str().is_empty(),
                "repository root must not be empty"
            );
        }
        _ => panic!("repository node must have Repository structural evidence"),
    }
}

#[test]
fn structural_evidence_contains_file_path() {
    let ev = evidence("src/lib.rs", 0);
    let func = FunctionFact::new(
        entity_id("src/lib.rs", "function", "ev_file", 0),
        "ev_file",
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        ev,
    );

    let facts = build_facts(vec![Entity::Function(func)]);
    let ctx = RepositoryContext::from_facts(&facts);
    let graph = GraphBuilder::build(&facts, &ctx).unwrap();

    let files: Vec<_> = graph.nodes_by_kind(NodeKind::File).collect();
    assert_eq!(files.len(), 1);

    match files[0].evidence() {
        GraphEvidence::Structural(StructuralEvidence::File { path }) => {
            assert_eq!(path, &std::path::PathBuf::from("src/lib.rs"));
        }
        _ => panic!("file node must have File structural evidence"),
    }
}

#[test]
fn structural_evidence_description_is_derived() {
    let sev = StructuralEvidence::Repository {
        root: PathBuf::from("/repo"),
    };
    assert_eq!(sev.description(), "repository root: /repo");

    let sev = StructuralEvidence::Workspace {
        name: "default".to_string(),
    };
    assert_eq!(sev.description(), "workspace: default");

    let sev = StructuralEvidence::File {
        path: std::path::PathBuf::from("src/main.rs"),
    };
    assert_eq!(sev.description(), "file: src/main.rs");
}
