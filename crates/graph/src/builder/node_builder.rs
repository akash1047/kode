//! Entity node construction from extracted facts.
//!
//! Converts each extracted entity type into a graph [`Node`] with
//! appropriate [`GraphNodeId::Entity`] identity and [`GraphEvidence::Source`]
//! evidence.
//!
//! # Invariants
//!
//! * Entity nodes always use [`GraphNodeId::Entity`].
//! * Entity nodes always carry [`GraphEvidence::Source`].
//! * Entity evidence is passed through unchanged from Stage 3.
//! * Entity metadata is derived from extracted fact metadata.

use kode_analysis::extraction::{
    ConstantFact, EnumFact, ExportFact, FunctionFact, ImplBlockFact, ImportFact, ModuleFact,
    RepositoryFacts, StaticFact, StructFact, TraitFact, TypeAliasFact,
};

use crate::model::{Node, NodeKind, NodeMetadata};

/// Push entity nodes for all extracted facts into the node vector.
pub fn push_entity_nodes(facts: &RepositoryFacts, nodes: &mut Vec<Node>) {
    for entity in facts.modules() {
        nodes.push(module_to_node(entity));
    }
    for entity in facts.functions() {
        nodes.push(function_to_node(entity));
    }
    for entity in facts.structs() {
        nodes.push(struct_to_node(entity));
    }
    for entity in facts.enums() {
        nodes.push(enum_to_node(entity));
    }
    for entity in facts.traits() {
        nodes.push(trait_to_node(entity));
    }
    for entity in facts.impl_blocks() {
        nodes.push(impl_block_to_node(entity));
    }
    for entity in facts.type_aliases() {
        nodes.push(type_alias_to_node(entity));
    }
    for entity in facts.constants() {
        nodes.push(constant_to_node(entity));
    }
    for entity in facts.statics() {
        nodes.push(static_to_node(entity));
    }
    for entity in facts.imports() {
        nodes.push(import_to_node(entity));
    }
    for entity in facts.exports() {
        nodes.push(export_to_node(entity));
    }
}

fn module_to_node(fact: &ModuleFact) -> Node {
    Node::entity(
        *fact.id(),
        NodeKind::Module,
        fact.name(),
        NodeMetadata::new(Some(fact.visibility().clone()), None),
        fact.evidence().clone(),
    )
}

fn function_to_node(fact: &FunctionFact) -> Node {
    Node::entity(
        *fact.id(),
        NodeKind::Function,
        fact.name(),
        NodeMetadata::new(
            Some(fact.visibility().clone()),
            fact.documentation().map(|s| s.to_string()),
        ),
        fact.evidence().clone(),
    )
}

fn struct_to_node(fact: &StructFact) -> Node {
    Node::entity(
        *fact.id(),
        NodeKind::Struct,
        fact.name(),
        NodeMetadata::new(Some(fact.visibility().clone()), None),
        fact.evidence().clone(),
    )
}

fn enum_to_node(fact: &EnumFact) -> Node {
    Node::entity(
        *fact.id(),
        NodeKind::Enum,
        fact.name(),
        NodeMetadata::new(Some(fact.visibility().clone()), None),
        fact.evidence().clone(),
    )
}

fn trait_to_node(fact: &TraitFact) -> Node {
    Node::entity(
        *fact.id(),
        NodeKind::Trait,
        fact.name(),
        NodeMetadata::new(Some(fact.visibility().clone()), None),
        fact.evidence().clone(),
    )
}

fn impl_block_to_node(fact: &ImplBlockFact) -> Node {
    Node::entity(
        *fact.id(),
        NodeKind::ImplBlock,
        fact.target_type(),
        NodeMetadata::new(None, None),
        fact.evidence().clone(),
    )
}

fn type_alias_to_node(fact: &TypeAliasFact) -> Node {
    Node::entity(
        *fact.id(),
        NodeKind::TypeAlias,
        fact.name(),
        NodeMetadata::new(None, None),
        fact.evidence().clone(),
    )
}

fn constant_to_node(fact: &ConstantFact) -> Node {
    Node::entity(
        *fact.id(),
        NodeKind::Constant,
        fact.name(),
        NodeMetadata::new(None, None),
        fact.evidence().clone(),
    )
}

fn static_to_node(fact: &StaticFact) -> Node {
    Node::entity(
        *fact.id(),
        NodeKind::Static,
        fact.name(),
        NodeMetadata::new(None, None),
        fact.evidence().clone(),
    )
}

fn import_to_node(fact: &ImportFact) -> Node {
    Node::entity(
        *fact.id(),
        NodeKind::Import,
        fact.path(),
        NodeMetadata::new(None, None),
        fact.evidence().clone(),
    )
}

fn export_to_node(fact: &ExportFact) -> Node {
    Node::entity(
        *fact.id(),
        NodeKind::Export,
        fact.path(),
        NodeMetadata::new(None, None),
        fact.evidence().clone(),
    )
}
