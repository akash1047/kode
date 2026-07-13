//! Preloaded, Send+Sync symbol index for agent tools.
//!
//! Avoids holding `QueryEngine` (storage backends are not Sync) inside the
//! agent so turns can run on worker tasks.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

/// One symbol row for tool results.
#[derive(Debug, Clone)]
pub struct IndexedSymbol {
    pub name: String,
    pub kind: String,
    pub file_path: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub verified: bool,
}

/// Immutable snapshot of symbols + call graph for agent tools.
#[derive(Debug, Clone, Default)]
pub struct SymbolIndex {
    symbols: Vec<IndexedSymbol>,
    /// function name → names that call it
    callers_of: HashMap<String, Vec<String>>,
    /// function name → names it calls
    callees_of: HashMap<String, Vec<String>>,
}

impl SymbolIndex {
    pub fn new(symbols: Vec<IndexedSymbol>) -> Self {
        Self {
            symbols,
            callers_of: HashMap::new(),
            callees_of: HashMap::new(),
        }
    }

    pub fn from_query_engine(
        engine: &kode_query::QueryEngine,
    ) -> Result<Self, kode_query::QueryError> {
        let results = engine.search_symbols("", None)?;
        let symbols: Vec<IndexedSymbol> = results
            .into_iter()
            .map(|s| IndexedSymbol {
                name: s.name,
                kind: s.kind.as_str().to_string(),
                file_path: s.file_path,
                start_line: s.start_line,
                end_line: s.end_line,
                verified: s.verified,
            })
            .collect();

        // Build call adjacency from query engine for all function names.
        let mut callers_of: HashMap<String, Vec<String>> = HashMap::new();
        let mut callees_of: HashMap<String, Vec<String>> = HashMap::new();

        let fn_names: BTreeSet<String> = symbols
            .iter()
            .filter(|s| s.kind == "function")
            .map(|s| s.name.clone())
            .collect();

        for name in &fn_names {
            if let Ok(callers) = engine.find_callers(name) {
                let list: Vec<String> = callers.into_iter().map(|c| c.name).collect();
                if !list.is_empty() {
                    callers_of.insert(name.clone(), list);
                }
            }
            if let Ok(callees) = engine.find_callees(name) {
                let list: Vec<String> = callees.into_iter().map(|c| c.name).collect();
                if !list.is_empty() {
                    callees_of.insert(name.clone(), list);
                }
            }
        }

        Ok(Self {
            symbols,
            callers_of,
            callees_of,
        })
    }

    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    pub fn search(&self, query: &str) -> Vec<&IndexedSymbol> {
        self.symbols
            .iter()
            .filter(|s| s.name == query || s.name.starts_with(query))
            .collect()
    }

    pub fn find_exact(&self, name: &str) -> Option<&IndexedSymbol> {
        self.symbols.iter().find(|s| s.name == name)
    }

    fn symbols_named(&self, names: &[String]) -> Vec<&IndexedSymbol> {
        let set: BTreeSet<&str> = names.iter().map(|s| s.as_str()).collect();
        self.symbols
            .iter()
            .filter(|s| set.contains(s.name.as_str()))
            .collect()
    }

    pub fn find_callers(&self, name: &str) -> Vec<&IndexedSymbol> {
        let names = self.callers_of.get(name).cloned().unwrap_or_default();
        self.symbols_named(&names)
    }

    pub fn find_callees(&self, name: &str) -> Vec<&IndexedSymbol> {
        let names = self.callees_of.get(name).cloned().unwrap_or_default();
        self.symbols_named(&names)
    }

    /// Reverse-call BFS: functions that transitively call `name`.
    pub fn impact(&self, name: &str, max_depth: Option<usize>) -> Vec<&IndexedSymbol> {
        let mut impacted: BTreeSet<String> = BTreeSet::new();
        let mut visited: BTreeSet<String> = BTreeSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        queue.push_back((name.to_string(), 0));

        while let Some((current, depth)) = queue.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if depth > 0 {
                impacted.insert(current.clone());
            }
            if max_depth.is_some_and(|m| depth >= m) {
                continue;
            }
            if let Some(callers) = self.callers_of.get(&current) {
                for c in callers {
                    queue.push_back((c.clone(), depth + 1));
                }
            }
        }

        let names: Vec<String> = impacted.into_iter().collect();
        self.symbols_named(&names)
    }

    /// Number of recorded call edges (for diagnostics).
    pub fn call_edge_count(&self) -> usize {
        self.callees_of.values().map(|v| v.len()).sum()
    }

    /// Fan-in / fan-out for functions (optionally filtered by exact name).
    pub fn metrics(
        &self,
        name_filter: Option<&str>,
    ) -> Vec<(String, usize, usize, PathBuf, usize)> {
        let mut rows = Vec::new();
        for s in &self.symbols {
            if s.kind != "function" {
                continue;
            }
            if name_filter.is_some_and(|n| s.name != n) {
                continue;
            }
            let fan_in = self.callers_of.get(&s.name).map(|v| v.len()).unwrap_or(0);
            let fan_out = self.callees_of.get(&s.name).map(|v| v.len()).unwrap_or(0);
            rows.push((
                s.name.clone(),
                fan_in,
                fan_out,
                s.file_path.clone(),
                s.start_line,
            ));
        }
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        rows
    }

    /// Functions with no recorded callers (heuristic dead / unreferenced).
    pub fn dead_code(&self) -> Vec<&IndexedSymbol> {
        self.symbols
            .iter()
            .filter(|s| {
                s.kind == "function"
                    && s.name != "main"
                    && !s.name.starts_with("test_")
                    && !self.callers_of.contains_key(&s.name)
            })
            .collect()
    }
}

/// Shared index handle.
pub type SharedIndex = Arc<SymbolIndex>;
