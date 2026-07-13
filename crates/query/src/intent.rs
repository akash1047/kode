//! Deterministic query intent resolution.
//!
//! Maps user query strings to structured intents. The same string always
//! produces the same intent (prefix-based, case-sensitive prefixes).

/// Structured query intent resolved from a free-form query string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryIntent {
    /// Prefix / exact symbol name search.
    Search { query: String },
    /// Incoming `Calls` edges: who calls `name`.
    Callers { name: String },
    /// Outgoing `Calls` edges: what `name` calls.
    Callees { name: String },
    /// Reverse call BFS impact set for `name`.
    Impact {
        name: String,
        max_depth: Option<usize>,
    },
    /// Functions with zero incoming `Calls` (heuristic dead code).
    DeadCode,
    /// Strongly connected components of size > 1 on the call graph.
    Cycles,
    /// Fan-in / fan-out metrics; optional symbol name filter.
    Metrics { name: Option<String> },
}

/// Parse a query string into a [`QueryIntent`].
///
/// Supported prefixes (checked in order):
/// - `callers:` / `who calls `
/// - `callees:`
/// - `impact:` (optional `:depth` after name, e.g. `impact:foo:4`)
/// - `dead:` / `deadcode` / `dead-code`
/// - `cycles:` / `cycles`
/// - `metrics:` / `metrics` (optional name after colon)
///
/// Anything else is treated as a symbol search.
pub fn parse_intent(raw: &str) -> QueryIntent {
    let q = raw.trim();
    if q.is_empty() {
        return QueryIntent::Search {
            query: String::new(),
        };
    }

    if let Some(rest) = q.strip_prefix("callers:") {
        return QueryIntent::Callers {
            name: rest.trim().to_string(),
        };
    }
    if let Some(rest) = q.strip_prefix("who calls ") {
        return QueryIntent::Callers {
            name: rest.trim().to_string(),
        };
    }
    if let Some(rest) = q.strip_prefix("callees:") {
        return QueryIntent::Callees {
            name: rest.trim().to_string(),
        };
    }
    if let Some(rest) = q.strip_prefix("impact:") {
        let rest = rest.trim();
        // impact:name:depth or impact:name
        if let Some((name, depth_s)) = rest.rsplit_once(':') {
            if let Ok(d) = depth_s.trim().parse::<usize>() {
                return QueryIntent::Impact {
                    name: name.trim().to_string(),
                    max_depth: Some(d),
                };
            }
        }
        return QueryIntent::Impact {
            name: rest.to_string(),
            max_depth: Some(8),
        };
    }

    let dead_keys = ["dead:", "deadcode", "dead-code", "dead"];
    for key in dead_keys {
        if q.eq_ignore_ascii_case(key) || q.to_ascii_lowercase() == format!("{key}:") {
            return QueryIntent::DeadCode;
        }
        if let Some(rest) = q.strip_prefix(key) {
            if rest.is_empty() || rest == ":" || rest.trim().is_empty() {
                return QueryIntent::DeadCode;
            }
        }
    }
    if q.eq_ignore_ascii_case("dead") || q.eq_ignore_ascii_case("deadcode") {
        return QueryIntent::DeadCode;
    }

    if q.eq_ignore_ascii_case("cycles") || q.eq_ignore_ascii_case("cycles:") {
        return QueryIntent::Cycles;
    }
    if let Some(rest) = q.strip_prefix("cycles:") {
        if rest.trim().is_empty() {
            return QueryIntent::Cycles;
        }
    }

    if q.eq_ignore_ascii_case("metrics") {
        return QueryIntent::Metrics { name: None };
    }
    if let Some(rest) = q.strip_prefix("metrics:") {
        let name = rest.trim();
        return QueryIntent::Metrics {
            name: if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            },
        };
    }

    QueryIntent::Search {
        query: q.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_plain() {
        assert_eq!(
            parse_intent("hello"),
            QueryIntent::Search {
                query: "hello".into()
            }
        );
    }

    #[test]
    fn callers_prefix() {
        assert_eq!(
            parse_intent("callers:run_scan"),
            QueryIntent::Callers {
                name: "run_scan".into()
            }
        );
        assert_eq!(
            parse_intent("who calls run_scan"),
            QueryIntent::Callers {
                name: "run_scan".into()
            }
        );
    }

    #[test]
    fn callees_and_impact() {
        assert_eq!(
            parse_intent("callees:foo"),
            QueryIntent::Callees { name: "foo".into() }
        );
        assert_eq!(
            parse_intent("impact:bar"),
            QueryIntent::Impact {
                name: "bar".into(),
                max_depth: Some(8)
            }
        );
        assert_eq!(
            parse_intent("impact:bar:3"),
            QueryIntent::Impact {
                name: "bar".into(),
                max_depth: Some(3)
            }
        );
    }

    #[test]
    fn dead_cycles_metrics() {
        assert_eq!(parse_intent("dead"), QueryIntent::DeadCode);
        assert_eq!(parse_intent("dead:"), QueryIntent::DeadCode);
        assert_eq!(parse_intent("cycles"), QueryIntent::Cycles);
        assert_eq!(parse_intent("metrics"), QueryIntent::Metrics { name: None });
        assert_eq!(
            parse_intent("metrics:foo"),
            QueryIntent::Metrics {
                name: Some("foo".into())
            }
        );
    }
}
