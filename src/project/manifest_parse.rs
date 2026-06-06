use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

const TOP_DEPS_CAP: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedManifest {
    pub ecosystem: Ecosystem,
    pub name: Option<String>,
    pub version: Option<String>,
    pub language_version: Option<String>,
    pub scripts: Vec<Script>,
    pub top_deps: Vec<String>,
    pub workspace_marker: Option<WorkspaceMarker>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ecosystem {
    Rust,
    Python,
    Node,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    pub name: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum WorkspaceMarker {
    CargoWorkspace { members: Vec<String> },
    NpmWorkspaces { packages: Vec<String> },
    UvWorkspace { members: Vec<String> },
    PoetryWorkspace { members: Vec<String> },
}

pub fn parse(path: &Path, content: &str) -> Result<ParsedManifest, String> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    match name {
        "Cargo.toml" => parse_cargo(content),
        "package.json" => parse_package_json(content),
        "pyproject.toml" => parse_pyproject(content),
        other => Err(format!("unsupported manifest: {other}")),
    }
}

pub fn is_manifest(file_name: &str) -> bool {
    matches!(file_name, "Cargo.toml" | "package.json" | "pyproject.toml")
}

fn parse_cargo(content: &str) -> Result<ParsedManifest, String> {
    let v: toml::Value = toml::from_str(content).map_err(|e| e.to_string())?;

    let pkg = v.get("package").and_then(|x| x.as_table());
    let name = pkg.and_then(|p| p.get("name")).and_then(|x| x.as_str()).map(String::from);
    let version = pkg.and_then(|p| p.get("version")).and_then(|x| x.as_str()).map(String::from);
    let language_version = pkg
        .and_then(|p| p.get("rust-version"))
        .and_then(|x| x.as_str())
        .map(String::from);

    let mut scripts = Vec::new();
    if let Some(bins) = v.get("bin").and_then(|x| x.as_array()) {
        for b in bins {
            let bin_name = b.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let bin_path = b.get("path").and_then(|x| x.as_str()).unwrap_or("").to_string();
            if !bin_name.is_empty() {
                scripts.push(Script { name: bin_name, target: bin_path });
            }
        }
    }

    let mut top_deps = Vec::new();
    if let Some(deps) = v.get("dependencies").and_then(|x| x.as_table()) {
        for (k, _) in deps.iter().take(TOP_DEPS_CAP) {
            top_deps.push(k.clone());
        }
    }

    let workspace_marker = v
        .get("workspace")
        .and_then(|x| x.as_table())
        .map(|ws| {
            let members = ws
                .get("members")
                .and_then(|m| m.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            WorkspaceMarker::CargoWorkspace { members }
        });

    Ok(ParsedManifest {
        ecosystem: Ecosystem::Rust,
        name,
        version,
        language_version,
        scripts,
        top_deps,
        workspace_marker,
    })
}

fn parse_package_json(content: &str) -> Result<ParsedManifest, String> {
    let v: Value = serde_json::from_str(content).map_err(|e| e.to_string())?;

    let name = v.get("name").and_then(|x| x.as_str()).map(String::from);
    let version = v.get("version").and_then(|x| x.as_str()).map(String::from);
    let language_version = v
        .get("engines")
        .and_then(|e| e.get("node"))
        .and_then(|x| x.as_str())
        .map(String::from);

    let mut scripts = Vec::new();
    if let Some(obj) = v.get("scripts").and_then(|x| x.as_object()) {
        for (k, val) in obj {
            if let Some(target) = val.as_str() {
                scripts.push(Script {
                    name: k.clone(),
                    target: target.to_string(),
                });
            }
        }
    }

    let mut top_deps = Vec::new();
    if let Some(deps) = v.get("dependencies").and_then(|x| x.as_object()) {
        for (k, _) in deps.iter().take(TOP_DEPS_CAP) {
            top_deps.push(k.clone());
        }
    }

    let workspace_marker = v.get("workspaces").and_then(|w| {
        let packages: Vec<String> = match w {
            Value::Array(arr) => arr.iter().filter_map(|x| x.as_str().map(String::from)).collect(),
            Value::Object(obj) => obj
                .get("packages")
                .and_then(|x| x.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            _ => return None,
        };
        Some(WorkspaceMarker::NpmWorkspaces { packages })
    });

    Ok(ParsedManifest {
        ecosystem: Ecosystem::Node,
        name,
        version,
        language_version,
        scripts,
        top_deps,
        workspace_marker,
    })
}

fn parse_pyproject(content: &str) -> Result<ParsedManifest, String> {
    let v: toml::Value = toml::from_str(content).map_err(|e| e.to_string())?;

    let project = v.get("project").and_then(|x| x.as_table());
    let poetry = v
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|x| x.as_table());

    let name = project
        .and_then(|p| p.get("name"))
        .and_then(|x| x.as_str())
        .map(String::from)
        .or_else(|| {
            poetry
                .and_then(|p| p.get("name"))
                .and_then(|x| x.as_str())
                .map(String::from)
        });
    let version = project
        .and_then(|p| p.get("version"))
        .and_then(|x| x.as_str())
        .map(String::from)
        .or_else(|| {
            poetry
                .and_then(|p| p.get("version"))
                .and_then(|x| x.as_str())
                .map(String::from)
        });
    let language_version = project
        .and_then(|p| p.get("requires-python"))
        .and_then(|x| x.as_str())
        .map(String::from);

    let mut scripts = Vec::new();
    if let Some(s) = project.and_then(|p| p.get("scripts")).and_then(|x| x.as_table()) {
        for (k, val) in s {
            if let Some(target) = val.as_str() {
                scripts.push(Script {
                    name: k.clone(),
                    target: target.to_string(),
                });
            }
        }
    }
    if let Some(s) = poetry.and_then(|p| p.get("scripts")).and_then(|x| x.as_table()) {
        for (k, val) in s {
            if let Some(target) = val.as_str() {
                scripts.push(Script {
                    name: k.clone(),
                    target: target.to_string(),
                });
            }
        }
    }

    let mut top_deps = Vec::new();
    if let Some(arr) = project.and_then(|p| p.get("dependencies")).and_then(|x| x.as_array()) {
        for d in arr.iter().take(TOP_DEPS_CAP) {
            if let Some(s) = d.as_str() {
                top_deps.push(extract_dep_name(s));
            }
        }
    } else if let Some(tbl) = poetry.and_then(|p| p.get("dependencies")).and_then(|x| x.as_table()) {
        for (k, _) in tbl.iter().take(TOP_DEPS_CAP) {
            if k != "python" {
                top_deps.push(k.clone());
            }
        }
    }

    let workspace_marker = v
        .get("tool")
        .and_then(|t| t.get("uv"))
        .and_then(|u| u.get("workspace"))
        .and_then(|x| x.as_table())
        .map(|ws| {
            let members = ws
                .get("members")
                .and_then(|m| m.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            WorkspaceMarker::UvWorkspace { members }
        });

    Ok(ParsedManifest {
        ecosystem: Ecosystem::Python,
        name,
        version,
        language_version,
        scripts,
        top_deps,
        workspace_marker,
    })
}

fn extract_dep_name(spec: &str) -> String {
    let cut = spec
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.')
        .unwrap_or(spec.len());
    spec[..cut].to_string()
}
