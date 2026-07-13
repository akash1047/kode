//! Integration tests for repository discovery.
//!
//! Tests cover empty repositories, single packages, Cargo workspaces,
//! language detection, `.gitignore` filtering, deterministic ordering,
//! and deeply nested directory structures.

#![allow(unused_crate_dependencies)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use kode_acquisition::{discover, Repository, WorkspaceKind};

fn dir_structure(root: &Path) {
    fs::write(root.join("README.md"), "# Test\n").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn hello() {}\n").unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
}

#[test]
fn empty_repository() {
    let dir = tempfile::TempDir::new().unwrap();
    let repo = Repository::new(dir.path()).unwrap();
    let snapshot = discover(&repo).unwrap();

    assert!(snapshot.files().is_empty());
    assert!(snapshot.directories().is_empty());
    assert!(snapshot.manifests().is_empty());
    assert!(snapshot.languages().is_empty());
    assert_eq!(snapshot.workspace().kind(), &WorkspaceKind::None);
}

#[test]
fn simple_repository() {
    let dir = tempfile::TempDir::new().unwrap();
    dir_structure(dir.path());
    let repo = Repository::new(dir.path()).unwrap();
    let snapshot = discover(&repo).unwrap();

    assert_eq!(snapshot.files().len(), 4);
    assert_eq!(snapshot.directories().len(), 1);
    assert_eq!(snapshot.manifests().len(), 1);

    let file_names: BTreeSet<_> = snapshot
        .files()
        .iter()
        .map(|f| f.relative_path().to_path_buf())
        .collect();
    assert!(file_names.contains(Path::new("Cargo.toml")));
    assert!(file_names.contains(Path::new("src/lib.rs")));
    assert!(file_names.contains(Path::new("src/main.rs")));
    assert!(file_names.contains(Path::new("README.md")));

    let dir_names: BTreeSet<_> = snapshot
        .directories()
        .iter()
        .map(|d| d.relative_path().to_path_buf())
        .collect();
    assert!(dir_names.contains(Path::new("src")));
}

#[test]
fn single_package_workspace() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"mypkg\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let repo = Repository::new(dir.path()).unwrap();
    let snapshot = discover(&repo).unwrap();

    assert_eq!(
        snapshot.workspace().kind(),
        &WorkspaceKind::SinglePackage {
            manifest: PathBuf::from("Cargo.toml")
        }
    );
}

#[test]
fn cargo_workspace_detected() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n",
    )
    .unwrap();

    fs::create_dir_all(root.join("crates/alpha/src")).unwrap();
    fs::write(
        root.join("crates/alpha/Cargo.toml"),
        "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    fs::create_dir_all(root.join("crates/beta/src")).unwrap();
    fs::write(
        root.join("crates/beta/Cargo.toml"),
        "[package]\nname = \"beta\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let repo = Repository::new(root).unwrap();
    let snapshot = discover(&repo).unwrap();

    assert!(
        matches!(
            snapshot.workspace().kind(),
            WorkspaceKind::CargoWorkspace { .. }
        ),
        "expected CargoWorkspace, got {:?}",
        snapshot.workspace().kind()
    );

    let manifest_count = snapshot
        .manifests()
        .iter()
        .filter(|m| m.relative_path().ends_with("Cargo.toml"))
        .count();
    assert_eq!(manifest_count, 3);
}

#[test]
fn language_inventory_populated() {
    let dir = tempfile::TempDir::new().unwrap();
    dir_structure(dir.path());
    let repo = Repository::new(dir.path()).unwrap();
    let snapshot = discover(&repo).unwrap();

    let lang_names: BTreeSet<_> = snapshot.languages().iter().map(|l| l.to_string()).collect();
    assert!(lang_names.contains("Rust"));
    assert!(lang_names.contains("Markdown"));
    assert!(lang_names.contains("TOML"));
}

#[test]
fn gitignore_respected() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    fs::write(root.join(".gitignore"), "secret.txt\n").unwrap();
    fs::write(root.join("visible.txt"), "i am visible\n").unwrap();
    fs::write(root.join("secret.txt"), "i am hidden\n").unwrap();

    // `ignore` crate requires a .git directory to enable gitignore processing
    fs::create_dir_all(root.join(".git")).unwrap();

    let repo = Repository::new(root).unwrap();
    let snapshot = discover(&repo).unwrap();

    let file_names: BTreeSet<_> = snapshot
        .files()
        .iter()
        .map(|f| f.relative_path().to_path_buf())
        .collect();

    assert!(
        file_names.contains(Path::new("visible.txt")),
        "visible.txt should be included, got: {file_names:?}"
    );
    assert!(
        !file_names.contains(Path::new("secret.txt")),
        "secret.txt should be excluded by .gitignore"
    );
}

#[test]
fn kodeignore_excludes_patterns() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    fs::write(root.join(".kodeignore"), "hidden.txt\n").unwrap();
    fs::write(root.join("visible.txt"), "ok\n").unwrap();
    fs::write(root.join("hidden.txt"), "nope\n").unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();

    let repo = Repository::new(root).unwrap();
    let snapshot = discover(&repo).unwrap();
    let file_names: BTreeSet<_> = snapshot
        .files()
        .iter()
        .map(|f| f.relative_path().to_path_buf())
        .collect();

    assert!(file_names.contains(Path::new("visible.txt")));
    assert!(
        !file_names.contains(Path::new("hidden.txt")),
        "hidden.txt should be excluded by .kodeignore, got: {file_names:?}"
    );
}

#[test]
fn kodeignore_overrides_gitignore_conflict() {
    // When both exist, custom ignore (.kodeignore) has higher precedence.
    // .gitignore ignores keepme.txt; .kodeignore uses !keepme.txt to re-include it.
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    fs::write(root.join(".gitignore"), "keepme.txt\n").unwrap();
    fs::write(root.join(".kodeignore"), "!keepme.txt\n").unwrap();
    fs::write(root.join("keepme.txt"), "kept\n").unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();

    let repo = Repository::new(root).unwrap();
    let snapshot = discover(&repo).unwrap();
    let file_names: BTreeSet<_> = snapshot
        .files()
        .iter()
        .map(|f| f.relative_path().to_path_buf())
        .collect();

    assert!(
        file_names.contains(Path::new("keepme.txt")),
        "kodeignore negation should win over gitignore exclude, got: {file_names:?}"
    );
}

#[test]
fn nested_directories() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    fs::create_dir_all(root.join("a/b/c")).unwrap();
    fs::create_dir_all(root.join("a/d")).unwrap();
    fs::write(root.join("a/b/c/file.rs"), "").unwrap();
    fs::write(root.join("a/d/file.py"), "").unwrap();

    let repo = Repository::new(root).unwrap();
    let snapshot = discover(&repo).unwrap();

    let dir_names: BTreeSet<_> = snapshot
        .directories()
        .iter()
        .map(|d| d.relative_path().to_path_buf())
        .collect();
    assert!(dir_names.contains(Path::new("a")));
    assert!(dir_names.contains(Path::new("a/b")));
    assert!(dir_names.contains(Path::new("a/b/c")));
    assert!(dir_names.contains(Path::new("a/d")));

    let file_names: BTreeSet<_> = snapshot
        .files()
        .iter()
        .map(|f| f.relative_path().to_path_buf())
        .collect();
    assert!(file_names.contains(Path::new("a/b/c/file.rs")));
    assert!(file_names.contains(Path::new("a/d/file.py")));
}

#[test]
fn deterministic_ordering() {
    let dir = tempfile::TempDir::new().unwrap();
    dir_structure(dir.path());
    let repo = Repository::new(dir.path()).unwrap();

    let snapshot1 = discover(&repo).unwrap();
    let snapshot2 = discover(&repo).unwrap();

    let paths1: Vec<_> = snapshot1
        .files()
        .iter()
        .map(|f| f.relative_path().to_path_buf())
        .collect();
    let paths2: Vec<_> = snapshot2
        .files()
        .iter()
        .map(|f| f.relative_path().to_path_buf())
        .collect();

    assert_eq!(paths1, paths2, "file ordering must be deterministic");

    let dirs1: Vec<_> = snapshot1
        .directories()
        .iter()
        .map(|d| d.relative_path().to_path_buf())
        .collect();
    let dirs2: Vec<_> = snapshot2
        .directories()
        .iter()
        .map(|d| d.relative_path().to_path_buf())
        .collect();

    assert_eq!(dirs1, dirs2, "directory ordering must be deterministic");
}

#[test]
fn deep_nesting() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    let mut p = root.join("deep");
    for i in 0..50 {
        p = p.join(format!("nested{i}"));
    }
    fs::create_dir_all(&p).unwrap();
    fs::write(p.join("leaf.txt"), "leaf\n").unwrap();

    let repo = Repository::new(root).unwrap();
    let snapshot = discover(&repo).unwrap();

    assert!(
        !snapshot.files().is_empty(),
        "should find deeply nested file"
    );
    assert!(
        snapshot
            .files()
            .iter()
            .any(|f| f.relative_path().ends_with("leaf.txt")),
        "should include leaf.txt"
    );
}

#[test]
fn repository_with_only_docs() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("README.md"), "# Docs\n").unwrap();
    fs::write(root.join("docs/guide.md"), "# Guide\n").unwrap();

    let repo = Repository::new(root).unwrap();
    let snapshot = discover(&repo).unwrap();

    assert!(!snapshot.files().is_empty());
    assert!(
        snapshot
            .files()
            .iter()
            .all(|f| f.language().map_or(true, |l| l.to_string() == "Markdown")),
        "all files should be Markdown"
    );
}
