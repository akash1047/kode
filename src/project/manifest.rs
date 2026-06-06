use std::fs;
use std::path::Path;

use super::parse::{
    extract_kv, json_array_field, json_string_field, parse_array, unquote,
};

pub fn detect(root: &Path) -> (Option<String>, Vec<String>) {
    let cargo = root.join("Cargo.toml");
    if cargo.is_file() {
        if let Ok(s) = fs::read_to_string(&cargo) {
            return parse_cargo_toml(&s);
        }
    }
    let pkg = root.join("package.json");
    if pkg.is_file() {
        if let Ok(s) = fs::read_to_string(&pkg) {
            return parse_package_json(&s);
        }
    }
    let pyproj = root.join("pyproject.toml");
    if pyproj.is_file() {
        if let Ok(s) = fs::read_to_string(&pyproj) {
            return parse_pyproject_toml(&s);
        }
    }
    (None, Vec::new())
}

fn parse_cargo_toml(s: &str) -> (Option<String>, Vec<String>) {
    let mut in_package = false;
    let mut name = None;
    let mut authors = Vec::new();
    for raw in s.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some(v) = extract_kv(line, "name") {
            name = Some(unquote(v));
        } else if let Some(v) = extract_kv(line, "authors") {
            authors = parse_array(v);
        }
    }
    (name, authors)
}

fn parse_pyproject_toml(s: &str) -> (Option<String>, Vec<String>) {
    let mut section = String::new();
    let mut name = None;
    let mut authors = Vec::new();
    for raw in s.lines() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line.trim_matches(['[', ']']).to_string();
            continue;
        }
        let allow = section == "project" || section == "tool.poetry";
        if !allow {
            continue;
        }
        if name.is_none() {
            if let Some(v) = extract_kv(line, "name") {
                name = Some(unquote(v));
                continue;
            }
        }
        if let Some(v) = extract_kv(line, "authors") {
            for item in parse_array(v) {
                let cleaned = item.trim_start_matches('{').trim_end_matches('}').to_string();
                if let Some(rest) = cleaned.strip_prefix("name") {
                    let rest = rest.trim_start().trim_start_matches('=').trim();
                    authors.push(unquote(rest.trim_end_matches(',').trim()));
                } else {
                    authors.push(cleaned);
                }
            }
        }
    }
    (name, authors)
}

fn parse_package_json(s: &str) -> (Option<String>, Vec<String>) {
    let name = json_string_field(s, "name");
    let mut authors = Vec::new();
    if let Some(a) = json_string_field(s, "author") {
        authors.push(a);
    }
    if let Some(arr) = json_array_field(s, "contributors") {
        for v in arr {
            authors.push(v);
        }
    }
    (name, authors)
}
