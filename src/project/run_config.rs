use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ParsedRunConfig {
    Dockerfile {
        cmd: Option<DirectiveLine>,
        entrypoint: Option<DirectiveLine>,
        expose: Vec<DirectiveLine>,
        from: Option<DirectiveLine>,
    },
    Makefile {
        targets: Vec<DirectiveLine>,
    },
    Justfile {
        targets: Vec<DirectiveLine>,
    },
    Procfile {
        processes: Vec<DirectiveLine>,
    },
    DockerCompose {
        services: Vec<DirectiveLine>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectiveLine {
    pub value: String,
    pub line: usize,
}

pub fn detect_kind(file_name: &str) -> Option<&'static str> {
    let lower = file_name.to_ascii_lowercase();
    if lower == "dockerfile" || lower.starts_with("dockerfile.") || lower.ends_with(".dockerfile") {
        return Some("Dockerfile");
    }
    if lower == "makefile" || lower == "gnumakefile" {
        return Some("Makefile");
    }
    if lower == "justfile" || lower == ".justfile" {
        return Some("Justfile");
    }
    if lower == "procfile" {
        return Some("Procfile");
    }
    if lower == "docker-compose.yml" || lower == "docker-compose.yaml" || lower == "compose.yml" || lower == "compose.yaml" {
        return Some("DockerCompose");
    }
    None
}

pub fn parse(kind: &str, content: &str) -> Result<ParsedRunConfig, String> {
    match kind {
        "Dockerfile" => Ok(parse_dockerfile(content)),
        "Makefile" => Ok(parse_makefile(content)),
        "Justfile" => Ok(parse_justfile(content)),
        "Procfile" => Ok(parse_procfile(content)),
        "DockerCompose" => parse_compose(content),
        other => Err(format!("unsupported run-config kind: {other}")),
    }
}

fn parse_dockerfile(content: &str) -> ParsedRunConfig {
    let mut cmd = None;
    let mut entrypoint = None;
    let mut from = None;
    let mut expose = Vec::new();

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line_no = i + 1;
        let raw = lines[i];
        let stripped = strip_comment(raw).trim_start();

        let (mut full, mut consumed) = (raw.to_string(), 1);
        while full.trim_end().ends_with('\\') && i + consumed < lines.len() {
            full = format!(
                "{} {}",
                full.trim_end().trim_end_matches('\\').trim_end(),
                lines[i + consumed].trim()
            );
            consumed += 1;
        }
        let full_stripped = strip_comment(&full).trim().to_string();

        if let Some(rest) = uppercase_directive(stripped, "FROM") {
            from = Some(DirectiveLine { value: rest.trim().to_string(), line: line_no });
        } else if let Some(_) = uppercase_directive(stripped, "CMD") {
            let val = directive_value(&full_stripped, "CMD");
            cmd = Some(DirectiveLine { value: val, line: line_no });
        } else if let Some(_) = uppercase_directive(stripped, "ENTRYPOINT") {
            let val = directive_value(&full_stripped, "ENTRYPOINT");
            entrypoint = Some(DirectiveLine { value: val, line: line_no });
        } else if let Some(rest) = uppercase_directive(stripped, "EXPOSE") {
            for port in rest.split_whitespace() {
                expose.push(DirectiveLine { value: port.to_string(), line: line_no });
            }
        }

        i += consumed;
    }

    ParsedRunConfig::Dockerfile { cmd, entrypoint, expose, from }
}

fn parse_makefile(content: &str) -> ParsedRunConfig {
    let mut targets = Vec::new();
    for (idx, raw) in content.lines().enumerate() {
        if raw.starts_with('\t') || raw.starts_with(' ') {
            continue;
        }
        let line = strip_comment(raw).trim_end();
        if line.is_empty() {
            continue;
        }
        let Some(colon) = line.find(':') else { continue };
        let name = line[..colon].trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/')) {
            continue;
        }
        let after = line[colon + 1..].trim_start();
        if after.starts_with('=') {
            continue;
        }
        targets.push(DirectiveLine { value: name.to_string(), line: idx + 1 });
    }
    ParsedRunConfig::Makefile { targets }
}

fn parse_justfile(content: &str) -> ParsedRunConfig {
    let mut targets = Vec::new();
    for (idx, raw) in content.lines().enumerate() {
        if raw.starts_with(' ') || raw.starts_with('\t') {
            continue;
        }
        let line = strip_comment(raw).trim_end();
        if line.is_empty() {
            continue;
        }
        let Some(colon) = line.find(':') else { continue };
        let head = line[..colon].trim();
        let name = head.split_whitespace().next().unwrap_or("");
        if name.is_empty() {
            continue;
        }
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')) {
            continue;
        }
        targets.push(DirectiveLine { value: name.to_string(), line: idx + 1 });
    }
    ParsedRunConfig::Justfile { targets }
}

fn parse_procfile(content: &str) -> ParsedRunConfig {
    let mut processes = Vec::new();
    for (idx, raw) in content.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let Some(colon) = line.find(':') else { continue };
        let name = line[..colon].trim();
        let cmd = line[colon + 1..].trim();
        if name.is_empty() || cmd.is_empty() {
            continue;
        }
        processes.push(DirectiveLine { value: format!("{name}: {cmd}"), line: idx + 1 });
    }
    ParsedRunConfig::Procfile { processes }
}

fn parse_compose(content: &str) -> Result<ParsedRunConfig, String> {
    let yaml: serde_yaml::Value = serde_yaml::from_str(content).map_err(|e| e.to_string())?;
    let mut services = Vec::new();

    if let Some(map) = yaml.get("services").and_then(|s| s.as_mapping()) {
        for (k, _) in map {
            if let Some(name) = k.as_str() {
                let line = find_yaml_key_line(content, "services", name).unwrap_or(0);
                services.push(DirectiveLine { value: name.to_string(), line });
            }
        }
    }

    Ok(ParsedRunConfig::DockerCompose { services })
}

fn find_yaml_key_line(content: &str, parent: &str, child: &str) -> Option<usize> {
    let mut in_parent = false;
    let mut parent_indent: Option<usize> = None;
    for (idx, raw) in content.lines().enumerate() {
        let indent = raw.chars().take_while(|c| *c == ' ').count();
        let trimmed = raw.trim_start();
        if !in_parent {
            if trimmed.starts_with(&format!("{parent}:")) {
                in_parent = true;
                parent_indent = Some(indent);
            }
            continue;
        }
        if let Some(p) = parent_indent {
            if !trimmed.is_empty() && indent <= p {
                return None;
            }
            if trimmed.starts_with(&format!("{child}:")) {
                return Some(idx + 1);
            }
        }
    }
    None
}

fn strip_comment(line: &str) -> &str {
    if let Some(pos) = line.find('#') {
        &line[..pos]
    } else {
        line
    }
}

fn uppercase_directive<'a>(line: &'a str, directive: &str) -> Option<&'a str> {
    let head = line.split_whitespace().next()?;
    if head.eq_ignore_ascii_case(directive) {
        Some(line[head.len()..].trim_start())
    } else {
        None
    }
}

fn directive_value(full_stripped: &str, directive: &str) -> String {
    let head = full_stripped.split_whitespace().next().unwrap_or("");
    if head.eq_ignore_ascii_case(directive) {
        full_stripped[head.len()..].trim().to_string()
    } else {
        full_stripped.to_string()
    }
}

pub fn kind_of(_p: &Path, file_name: &str) -> Option<&'static str> {
    detect_kind(file_name)
}
