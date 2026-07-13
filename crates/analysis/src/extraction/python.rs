//! Python fact extractor (functions and classes).

use std::path::Path;

use kode_acquisition::Language;

use crate::extraction::extractor::Extractor;
use crate::extraction::model::{Entity, EntityId, Evidence, FunctionFact, StructFact, Visibility};
use crate::extraction::result::ExtractionOutcome;
use crate::parsing::syntax::SyntaxTree;

/// Extracts functions and classes from Python syntax trees.
#[derive(Debug)]
pub struct PythonExtractor;

impl Extractor for PythonExtractor {
    fn language(&self) -> Language {
        Language::Python
    }

    fn extract(&self, tree: &SyntaxTree) -> ExtractionOutcome {
        let backend = tree.backend();
        let source = tree.source();
        let path = tree.relative_path();
        let root = backend.tree.root_node();
        let mut entities = Vec::new();
        walk(root, source, path, &mut entities);
        ExtractionOutcome::Success(entities)
    }
}

fn walk(node: tree_sitter::Node, source: &str, path: &Path, entities: &mut Vec<Entity>) {
    match node.kind() {
        "function_definition" => {
            if let Some(e) = extract_function(node, source, path) {
                entities.push(Entity::Function(e));
            }
        }
        "class_definition" => {
            if let Some(e) = extract_class(node, source, path) {
                entities.push(Entity::Struct(e));
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, source, path, entities);
    }
}

fn extract_function(node: tree_sitter::Node, source: &str, path: &Path) -> Option<FunctionFact> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source);
    if name.is_empty() {
        return None;
    }
    let evidence = evidence_for(node, source, path, "function_definition");
    let id = EntityId::from_location(
        &Language::Python,
        "function",
        path,
        &name,
        evidence.byte_range().start,
    );
    Some(FunctionFact::new(
        id,
        name,
        Visibility::Public,
        None,
        Vec::new(),
        false,
        false,
        false,
        None,
        None,
        None,
        evidence,
    ))
}

fn extract_class(node: tree_sitter::Node, source: &str, path: &Path) -> Option<StructFact> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source);
    if name.is_empty() {
        return None;
    }
    let evidence = evidence_for(node, source, path, "class_definition");
    let id = EntityId::from_location(
        &Language::Python,
        "struct",
        path,
        &name,
        evidence.byte_range().start,
    );
    Some(StructFact::new(
        id,
        name,
        Vec::new(),
        Visibility::Public,
        Vec::new(),
        evidence,
    ))
}

fn evidence_for(node: tree_sitter::Node, source: &str, path: &Path, kind: &str) -> Evidence {
    let start = node.start_position();
    let end = node.end_position();
    let _ = source;
    Evidence::new(
        path.to_path_buf(),
        kind,
        node.start_byte()..node.end_byte(),
        start.row + 1,
        start.column,
        end.row + 1,
        end.column,
        Language::Python,
    )
}

fn node_text(node: tree_sitter::Node, source: &str) -> String {
    source
        .get(node.start_byte()..node.end_byte())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    use kode_acquisition::{FileMetadata, RepositoryFile};

    use crate::parsing::{ParseOutcome, Parser, PythonParser};

    #[test]
    fn extract_python_function_and_class() {
        let source = r#"
def hello(x):
    return x

class Foo:
    pass
"#;
        let file = RepositoryFile::new(
            PathBuf::from("mod.py"),
            FileMetadata::new(source.len() as u64, None),
            Some(Language::Python),
        );
        let outcome = PythonParser.parse(Arc::from(source), &file);
        let tree = match outcome {
            ParseOutcome::Success(t) | ParseOutcome::Recovered(t) => t,
            other => panic!("parse failed: {other:?}"),
        };
        let extracted = PythonExtractor.extract(&tree);
        let entities = match extracted {
            ExtractionOutcome::Success(e) => e,
            other => panic!("extract failed: {other:?}"),
        };
        let names: Vec<_> = entities
            .iter()
            .map(|e| match e {
                Entity::Function(f) => f.name().to_string(),
                Entity::Struct(s) => s.name().to_string(),
                _ => String::new(),
            })
            .collect();
        assert!(names.contains(&"hello".to_string()));
        assert!(names.contains(&"Foo".to_string()));
    }
}
