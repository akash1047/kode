//! Graph analysis over the Knowledge Graph: metrics, dead code, cycles.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

use kode_graph::{GraphNodeId, KnowledgeGraph, NodeKind, RelationshipKind};

use crate::SymbolResult;

/// Fan-in / fan-out metrics for a function symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionMetrics {
    pub name: String,
    pub file_path: PathBuf,
    pub start_line: usize,
    /// Number of distinct callers (incoming `Calls`).
    pub fan_in: usize,
    /// Number of distinct callees (outgoing `Calls`).
    pub fan_out: usize,
    /// Instability = fan_out / (fan_in + fan_out), scaled to 0–100.
    /// 100 = fully unstable (only depends outward); 0 = only depended upon.
    pub instability_pct: u8,
    pub verified: bool,
}

/// One cycle in the call graph (ordered function names, closed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallCycle {
    pub members: Vec<String>,
}

/// Compute fan-in / fan-out for functions in the graph.
///
/// If `name_filter` is set, only functions with that exact name are returned.
pub fn function_metrics(graph: &KnowledgeGraph, name_filter: Option<&str>) -> Vec<FunctionMetrics> {
    let mut out = Vec::new();
    for node in graph.nodes() {
        if node.kind() != NodeKind::Function {
            continue;
        }
        if name_filter.is_some_and(|n| node.name() != n) {
            continue;
        }
        let id = node.id();
        let fan_in = graph
            .incoming(id)
            .filter(|r| r.kind() == RelationshipKind::Calls)
            .map(|r| *r.source())
            .collect::<BTreeSet<_>>()
            .len();
        let fan_out = graph
            .outgoing(id)
            .filter(|r| r.kind() == RelationshipKind::Calls)
            .map(|r| *r.target())
            .collect::<BTreeSet<_>>()
            .len();
        let denom = fan_in + fan_out;
        let instability_pct = fan_out
            .checked_mul(100)
            .and_then(|n| n.checked_div(denom))
            .unwrap_or(0) as u8;
        let sym = SymbolResult::from_node(node);
        out.push(FunctionMetrics {
            name: sym.name,
            file_path: sym.file_path,
            start_line: sym.start_line,
            fan_in,
            fan_out,
            instability_pct,
            verified: sym.verified,
        });
    }
    out.sort_by(|a, b| {
        b.fan_in
            .cmp(&a.fan_in)
            .then_with(|| b.fan_out.cmp(&a.fan_out))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.file_path.cmp(&b.file_path))
    });
    out
}

/// Functions with zero incoming `Calls` edges (heuristic dead / entry points).
///
/// Excludes common entry-point names (`main`, `lib`, test helpers are not
/// special-cased beyond name). Callers should interpret results carefully.
pub fn dead_code_candidates(graph: &KnowledgeGraph) -> Vec<SymbolResult> {
    let mut results = Vec::new();
    for node in graph.nodes() {
        if node.kind() != NodeKind::Function {
            continue;
        }
        let name = node.name();
        // Keep obvious entry points out of the dead list.
        if name == "main" || name == "test" || name.starts_with("test_") {
            continue;
        }
        let has_caller = graph
            .incoming(node.id())
            .any(|r| r.kind() == RelationshipKind::Calls);
        if !has_caller {
            results.push(SymbolResult::from_node(node));
        }
    }
    results.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.file_path.cmp(&b.file_path))
    });
    results
}

/// Find call-graph cycles (SCCs of size ≥ 2, or self-loops).
pub fn call_cycles(graph: &KnowledgeGraph) -> Vec<CallCycle> {
    // Build adjacency: function id -> called function ids
    let mut adj: HashMap<GraphNodeId, Vec<GraphNodeId>> = HashMap::new();
    let mut names: HashMap<GraphNodeId, String> = HashMap::new();

    for node in graph.nodes() {
        if node.kind() != NodeKind::Function {
            continue;
        }
        names.insert(*node.id(), node.name().to_string());
        adj.entry(*node.id()).or_default();
    }
    for rel in graph.relationships() {
        if rel.kind() != RelationshipKind::Calls {
            continue;
        }
        if names.contains_key(rel.source()) && names.contains_key(rel.target()) {
            adj.entry(*rel.source()).or_default().push(*rel.target());
        }
    }

    // Tarjan SCC (state bundled to avoid too-many-arguments).
    struct Tarjan<'a> {
        adj: &'a HashMap<GraphNodeId, Vec<GraphNodeId>>,
        index: usize,
        stack: Vec<GraphNodeId>,
        on_stack: HashSet<GraphNodeId>,
        indices: HashMap<GraphNodeId, usize>,
        lowlink: HashMap<GraphNodeId, usize>,
        sccs: Vec<Vec<GraphNodeId>>,
    }

    impl Tarjan<'_> {
        fn strongconnect(&mut self, v: GraphNodeId) {
            self.indices.insert(v, self.index);
            self.lowlink.insert(v, self.index);
            self.index += 1;
            self.stack.push(v);
            self.on_stack.insert(v);

            if let Some(neighbors) = self.adj.get(&v).cloned() {
                for w in neighbors {
                    if !self.indices.contains_key(&w) {
                        self.strongconnect(w);
                        let lw = *self.lowlink.get(&w).unwrap_or(&0);
                        let lv = *self.lowlink.get(&v).unwrap_or(&0);
                        self.lowlink.insert(v, lv.min(lw));
                    } else if self.on_stack.contains(&w) {
                        let iw = *self.indices.get(&w).unwrap_or(&0);
                        let lv = *self.lowlink.get(&v).unwrap_or(&0);
                        self.lowlink.insert(v, lv.min(iw));
                    }
                }
            }

            if self.lowlink.get(&v) == self.indices.get(&v) {
                let mut comp = Vec::new();
                loop {
                    let w = self.stack.pop().expect("tarjan stack");
                    self.on_stack.remove(&w);
                    comp.push(w);
                    if w == v {
                        break;
                    }
                }
                self.sccs.push(comp);
            }
        }
    }

    let mut tarjan = Tarjan {
        adj: &adj,
        index: 0,
        stack: Vec::new(),
        on_stack: HashSet::new(),
        indices: HashMap::new(),
        lowlink: HashMap::new(),
        sccs: Vec::new(),
    };

    let nodes: Vec<GraphNodeId> = names.keys().copied().collect();
    for v in nodes {
        if !tarjan.indices.contains_key(&v) {
            tarjan.strongconnect(v);
        }
    }
    let sccs = tarjan.sccs;

    let mut cycles = Vec::new();
    for scc in sccs {
        if scc.is_empty() {
            continue;
        }
        // Self-loop: single node that calls itself
        if scc.len() == 1 {
            let id = scc[0];
            let self_loop = adj.get(&id).map(|ns| ns.contains(&id)).unwrap_or(false);
            if !self_loop {
                continue;
            }
        }
        if scc.len() < 2
            && !adj
                .get(&scc[0])
                .map(|n| n.contains(&scc[0]))
                .unwrap_or(false)
        {
            continue;
        }
        let mut members: Vec<String> = scc.iter().filter_map(|id| names.get(id).cloned()).collect();
        members.sort();
        members.dedup();
        if members.len() >= 2
            || (members.len() == 1
                && adj
                    .get(&scc[0])
                    .map(|n| n.contains(&scc[0]))
                    .unwrap_or(false))
        {
            cycles.push(CallCycle { members });
        }
    }
    cycles.sort_by(|a, b| a.members.cmp(&b.members));
    cycles
}

/// Format metrics rows for text CLI output.
pub fn format_metrics(metrics: &[FunctionMetrics]) -> String {
    let mut out = String::from("── kode metrics ────────────────────────────────────────────\n");
    if metrics.is_empty() {
        out.push_str("  (no functions)\n");
        return out;
    }
    out.push_str(&format!(
        "  {:<28} {:>7} {:>8} {:>6}  location\n",
        "function", "fan_in", "fan_out", "inst%"
    ));
    for m in metrics.iter().take(200) {
        let mark = if m.verified { "✓" } else { "?" };
        out.push_str(&format!(
            "  {mark} {:<26} {:>7} {:>8} {:>5}%  {}:{}\n",
            m.name,
            m.fan_in,
            m.fan_out,
            m.instability_pct,
            m.file_path.display(),
            m.start_line
        ));
    }
    if metrics.len() > 200 {
        out.push_str(&format!("  … {} more\n", metrics.len() - 200));
    }
    out
}

/// Format cycles for text CLI output.
pub fn format_cycles(cycles: &[CallCycle]) -> String {
    let mut out = String::from("── kode cycles ─────────────────────────────────────────────\n");
    if cycles.is_empty() {
        out.push_str("  (no call-graph cycles)\n");
        return out;
    }
    for (i, c) in cycles.iter().enumerate() {
        out.push_str(&format!("  {}. {}\n", i + 1, c.members.join(" → ")));
    }
    out
}

/// Cohesion helper: fraction of calls that stay within the same file (0–100).
pub fn file_cohesion_pct(graph: &KnowledgeGraph) -> BTreeMap<String, u8> {
    let mut file_of: HashMap<GraphNodeId, String> = HashMap::new();
    for node in graph.nodes() {
        if node.kind() != NodeKind::Function {
            continue;
        }
        let path = match node.evidence() {
            kode_graph::GraphEvidence::Source(ev) => ev.source_file().display().to_string(),
            _ => continue,
        };
        file_of.insert(*node.id(), path);
    }

    let mut internal: HashMap<String, usize> = HashMap::new();
    let mut total: HashMap<String, usize> = HashMap::new();

    for rel in graph.relationships() {
        if rel.kind() != RelationshipKind::Calls {
            continue;
        }
        let Some(sf) = file_of.get(rel.source()) else {
            continue;
        };
        let Some(tf) = file_of.get(rel.target()) else {
            continue;
        };
        *total.entry(sf.clone()).or_default() += 1;
        if sf == tf {
            *internal.entry(sf.clone()).or_default() += 1;
        }
    }

    let mut result = BTreeMap::new();
    for (file, tot) in total {
        let inn = *internal.get(&file).unwrap_or(&0);
        let pct = inn
            .checked_mul(100)
            .and_then(|n| n.checked_div(tot))
            .unwrap_or(0) as u8;
        result.insert(file, pct);
    }
    result
}
