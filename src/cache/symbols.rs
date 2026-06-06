use tree_sitter::{Language, Parser, Query, QueryCursor};

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
}

const RUST_QUERY: &str = include_str!("queries/rust.scm");
const PYTHON_QUERY: &str = include_str!("queries/python.scm");
const TYPESCRIPT_QUERY: &str = include_str!("queries/typescript.scm");
const JAVASCRIPT_QUERY: &str = include_str!("queries/javascript.scm");
const GO_QUERY: &str = include_str!("queries/go.scm");
const JAVA_QUERY: &str = include_str!("queries/java.scm");
const C_QUERY: &str = include_str!("queries/c.scm");
const CPP_QUERY: &str = include_str!("queries/cpp.scm");
const RUBY_QUERY: &str = include_str!("queries/ruby.scm");
const CSHARP_QUERY: &str = include_str!("queries/csharp.scm");

pub fn is_supported(lang: &str) -> bool {
    matches!(
        lang,
        "rust" | "python" | "typescript" | "javascript"
            | "go" | "java" | "c" | "cpp" | "ruby" | "csharp"
    )
}

fn language_for(lang: &str) -> Option<(Language, &'static str)> {
    match lang {
        "rust" => Some((tree_sitter_rust::LANGUAGE.into(), RUST_QUERY)),
        "python" => Some((tree_sitter_python::LANGUAGE.into(), PYTHON_QUERY)),
        "typescript" => Some((
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            TYPESCRIPT_QUERY,
        )),
        "javascript" => Some((tree_sitter_javascript::LANGUAGE.into(), JAVASCRIPT_QUERY)),
        "go" => Some((tree_sitter_go::LANGUAGE.into(), GO_QUERY)),
        "java" => Some((tree_sitter_java::LANGUAGE.into(), JAVA_QUERY)),
        "c" => Some((tree_sitter_c::LANGUAGE.into(), C_QUERY)),
        "cpp" => Some((tree_sitter_cpp::LANGUAGE.into(), CPP_QUERY)),
        "ruby" => Some((tree_sitter_ruby::LANGUAGE.into(), RUBY_QUERY)),
        "csharp" => Some((tree_sitter_c_sharp::LANGUAGE.into(), CSHARP_QUERY)),
        _ => None,
    }
}

pub fn extract(lang: &str, source: &str) -> Result<Vec<Symbol>, String> {
    let Some((ts_lang, query_src)) = language_for(lang) else {
        return Ok(Vec::new());
    };

    let mut parser = Parser::new();
    parser
        .set_language(&ts_lang)
        .map_err(|e| format!("set_language: {e}"))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "parse returned None".to_string())?;

    let query = Query::new(&ts_lang, query_src).map_err(|e| format!("query compile: {e:?}"))?;

    let capture_names = query.capture_names();
    let bytes = source.as_bytes();
    let mut cursor = QueryCursor::new();
    let mut out = Vec::new();

    let mut matches = cursor.matches(&query, tree.root_node(), bytes);
    while let Some(m) = matches.next() {
        for cap in m.captures {
            let kind = capture_names
                .get(cap.index as usize)
                .copied()
                .unwrap_or("symbol");
            let node = cap.node;
            let Ok(name) = node.utf8_text(bytes) else {
                continue;
            };
            let name = name.trim().trim_matches('"').trim_matches('\'').to_string();
            if name.is_empty() {
                continue;
            }
            out.push(Symbol {
                name,
                kind: kind.to_string(),
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
            });
        }
    }

    Ok(out)
}
