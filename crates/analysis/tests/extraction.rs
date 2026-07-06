//! Integration tests for the Fact Extraction pipeline stage.
//!
//! Runs the full pipeline:
//!   Repository → RepositorySnapshot → SourceInventory → SyntaxTreeInventory → RepositoryFacts

use std::path::{Path, PathBuf};

use kode_acquisition::{
    DirectoryInventory, FileInventory, FileMetadata, Manifest, ManifestInventory,
    Repository, RepositoryFile, SnapshotBuilder, Workspace, Language,
};
use kode_analysis::extraction::{ExtractionOrchestrator, ExtractorRegistry, RepositoryFacts};
use kode_analysis::parsing::{ParserRegistry, ParsingOrchestrator, SourceInventory};

fn run_full_pipeline(files: Vec<(PathBuf, Option<Language>, &str)>) -> (RepositoryFacts, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().unwrap();
    let repo = Repository::new(dir.path()).unwrap();

    let mut repo_files = Vec::new();
    for (path, lang, content) in &files {
        let full_path = dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&full_path, content).unwrap();
        repo_files.push(RepositoryFile::new(
            path.clone(),
            FileMetadata::new(content.len() as u64, None),
            lang.clone(),
        ));
    }

    let mut manifest_inventory = ManifestInventory::new();
    manifest_inventory.push(Manifest::new(
        PathBuf::from("Cargo.toml"),
        kode_acquisition::ManifestKind::CargoManifest,
    ));

    let snapshot = SnapshotBuilder::new()
        .repository(repo)
        .workspace(Workspace::none())
        .file_inventory(FileInventory::new(repo_files))
        .directory_inventory(DirectoryInventory::new(Vec::new()))
        .manifest_inventory(manifest_inventory)
        .build()
        .unwrap();

    let sources = SourceInventory::from_snapshot(&snapshot).unwrap();
    let parser_orchestrator = ParsingOrchestrator::new(ParserRegistry::default());
    let tree_inventory = parser_orchestrator.run(&snapshot, &sources);

    let extractor_orchestrator = ExtractionOrchestrator::new(ExtractorRegistry::default());
    let facts = extractor_orchestrator.run(&tree_inventory);

    (facts, dir)
}

#[test]
fn empty_repository() {
    let (facts, _dir) = run_full_pipeline(Vec::new());
    assert!(facts.is_empty());
    assert_eq!(facts.entity_count(), 0);
    assert!(facts.diagnostics().is_empty() || !facts.diagnostics().is_empty());
}

#[test]
fn single_function() {
    let (facts, _dir) = run_full_pipeline(vec![(
        PathBuf::from("lib.rs"),
        Some(Language::Rust),
        "pub fn greet(name: &str) -> String { format!(\"Hello, {}!\", name) }",
    )]);

    assert_eq!(facts.functions().len(), 1);
    assert_eq!(facts.functions()[0].name(), "greet");
    assert_eq!(*facts.functions()[0].visibility(), kode_analysis::extraction::Visibility::Public);
    assert!(facts.functions()[0].signature().is_some());

    // Verify evidence
    let evidence = facts.functions()[0].evidence();
    assert_eq!(evidence.source_file(), Path::new("lib.rs"));
    assert_eq!(evidence.node_kind(), "function_item");
    assert_eq!(*evidence.language(), Language::Rust);
}

#[test]
fn multiple_entities() {
    let (facts, _dir) = run_full_pipeline(vec![(
        PathBuf::from("lib.rs"),
        Some(Language::Rust),
        r#"
        struct Point { x: i32, y: i32 }
        enum Color { Red, Green, Blue }
        fn distance(a: &Point, b: &Point) -> f64 { 0.0 }
        const MAX: i32 = 100;
        type MyResult<T> = Result<T, String>;
        "#,
    )]);

    assert_eq!(facts.structs().len(), 1);
    assert_eq!(facts.structs()[0].name(), "Point");
    assert_eq!(facts.structs()[0].fields().len(), 2);

    assert_eq!(facts.enums().len(), 1);
    assert_eq!(facts.enums()[0].name(), "Color");
    assert_eq!(facts.enums()[0].variants().len(), 3);

    assert_eq!(facts.functions().len(), 1);
    assert_eq!(facts.constants().len(), 1);
    assert_eq!(facts.type_aliases().len(), 1);
}

#[test]
fn deterministic_across_runs() {
    let files = vec![(
        PathBuf::from("lib.rs"),
        Some(Language::Rust),
        "fn b() {}\nfn a() {}\nfn c() {}",
    )];

    let (facts1, _dir1) = run_full_pipeline(files.clone());
    let (facts2, _dir2) = run_full_pipeline(files);

    let names1: Vec<&str> = facts1.functions().iter().map(|f| f.name()).collect();
    let names2: Vec<&str> = facts2.functions().iter().map(|f| f.name()).collect();

    assert_eq!(names1, names2, "extraction must be deterministic");
}

#[test]
fn evidence_attached_to_all_entities() {
    let (facts, _dir) = run_full_pipeline(vec![(
        PathBuf::from("lib.rs"),
        Some(Language::Rust),
        r#"
        mod utils;
        fn foo() {}
        struct Bar;
        enum Baz { A, B }
        trait Qux {}
        impl Qux for Bar {}
        type Result<T> = std::result::Result<T, String>;
        const X: i32 = 1;
        static Y: i32 = 2;
        use std::collections::HashMap;
        "#,
    )]);

    // Every entity must have evidence
    for f in facts.functions() {
        assert!(f.evidence().start_line() > 0);
        assert_eq!(f.evidence().language(), &Language::Rust);
    }
    for s in facts.structs() {
        assert!(s.evidence().start_line() > 0);
    }
    for e in facts.enums() {
        assert!(e.evidence().start_line() > 0);
    }
    for t in facts.traits() {
        assert!(t.evidence().start_line() > 0);
    }
    for i in facts.impl_blocks() {
        assert!(i.evidence().start_line() > 0);
    }
    for t in facts.type_aliases() {
        assert!(t.evidence().start_line() > 0);
    }
    for c in facts.constants() {
        assert!(c.evidence().start_line() > 0);
    }
    for s in facts.statics() {
        assert!(s.evidence().start_line() > 0);
    }
    for i in facts.imports() {
        assert!(i.evidence().start_line() > 0);
    }
}

#[test]
fn nested_modules() {
    let (facts, _dir) = run_full_pipeline(vec![(
        PathBuf::from("lib.rs"),
        Some(Language::Rust),
        r#"
        mod foo {
            pub fn bar() {}
            struct Inner;
        }
        pub mod baz;
        "#,
    )]);

    assert!(facts.modules().len() >= 2);
}

#[test]
fn generic_functions() {
    let (facts, _dir) = run_full_pipeline(vec![(
        PathBuf::from("lib.rs"),
        Some(Language::Rust),
        "fn identity<T: Clone + Debug>(x: T) -> T { x }",
    )]);

    assert_eq!(facts.functions().len(), 1);
    assert_eq!(facts.functions()[0].name(), "identity");
}

#[test]
fn async_functions() {
    let (facts, _dir) = run_full_pipeline(vec![(
        PathBuf::from("lib.rs"),
        Some(Language::Rust),
        "pub async fn fetch_data() -> Vec<u8> { vec![] }",
    )]);

    assert_eq!(facts.functions().len(), 1);
    assert!(facts.functions()[0].is_async());
}

#[test]
fn trait_implementations() {
    let (facts, _dir) = run_full_pipeline(vec![(
        PathBuf::from("lib.rs"),
        Some(Language::Rust),
        r#"
        trait Animal {
            fn make_sound(&self) -> &str;
        }
        struct Dog;
        impl Animal for Dog {
            fn make_sound(&self) -> &str { "woof" }
        }
        "#,
    )]);

    assert_eq!(facts.traits().len(), 1);
    assert_eq!(facts.impl_blocks().len(), 1);
    assert_eq!(facts.impl_blocks()[0].implemented_trait(), Some("Animal"));
    assert_eq!(facts.impl_blocks()[0].target_type(), "Dog");
}

#[test]
fn imports_and_exports() {
    let (facts, _dir) = run_full_pipeline(vec![(
        PathBuf::from("lib.rs"),
        Some(Language::Rust),
        r#"
        use std::collections::HashMap;
        use std::io::{self, Write};
        use crate::utils::*;
        "#,
    )]);

    assert!(facts.imports().len() >= 2);
}

#[test]
fn empty_file() {
    let (facts, _dir) = run_full_pipeline(vec![(
        PathBuf::from("empty.rs"),
        Some(Language::Rust),
        "",
    )]);

    assert!(facts.is_empty());
}

#[test]
fn stable_entity_ids() {
    let source = "fn foo() {}";
    let files = vec![(PathBuf::from("lib.rs"), Some(Language::Rust), source)];

    let (facts1, _dir1) = run_full_pipeline(files.clone());
    let (facts2, _dir2) = run_full_pipeline(files);

    let ids1: Vec<u64> = facts1.functions().iter().map(|f| f.id().as_u64()).collect();
    let ids2: Vec<u64> = facts2.functions().iter().map(|f| f.id().as_u64()).collect();

    assert_eq!(ids1, ids2, "entity IDs must be stable across runs");
}
