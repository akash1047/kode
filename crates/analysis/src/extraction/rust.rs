use std::path::Path;

use kode_acquisition::Language;

use crate::extraction::diagnostic::ExtractionDiagnostic;
use crate::extraction::extractor::Extractor;
use crate::extraction::model::*;
use crate::extraction::result::ExtractionOutcome;
use crate::parsing::syntax::SyntaxTree;

/// Extractor for Rust source files.
///
/// Walks the Tree-sitter syntax tree to extract entities with evidence.
#[derive(Debug)]
pub struct RustExtractor;

impl Extractor for RustExtractor {
    fn language(&self) -> Language {
        Language::Rust
    }

    fn extract(&self, tree: &SyntaxTree) -> ExtractionOutcome {
        let backend = tree.backend();
        let source = tree.source();
        let relative_path = tree.relative_path().to_path_buf();
        let language = tree.language().clone();

        let root = backend.tree.root_node();
        let mut entities = Vec::new();
        let mut diagnostics = Vec::new();

        if root.has_error() && !tree.source().trim().is_empty() {
            diagnostics.push(ExtractionDiagnostic::warning(
                "syntax tree contains errors; extraction may be incomplete",
                Some(relative_path.clone()),
                0,
                0,
                0,
                0,
            ));
        }

        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            process_top_level_node(
                child,
                source,
                &relative_path,
                &language,
                &mut entities,
                &mut diagnostics,
            );
        }

        if entities.is_empty() && !diagnostics.is_empty() {
            ExtractionOutcome::Failed(diagnostics)
        } else if !diagnostics.is_empty() {
            ExtractionOutcome::Partial(entities, diagnostics)
        } else {
            ExtractionOutcome::Success(entities)
        }
    }
}

// ---------------------------------------------------------------------------
// Top-level node dispatch
// ---------------------------------------------------------------------------

fn process_top_level_node(
    node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
    entities: &mut Vec<Entity>,
    diagnostics: &mut Vec<ExtractionDiagnostic>,
) {
    match node.kind() {
        "function_item" => match extract_function(node, source, file_path, language, None, None) {
            Ok(f) => entities.push(Entity::Function(f)),
            Err(d) => diagnostics.push(d),
        },
        "struct_item" => match extract_struct(node, source, file_path, language) {
            Ok(s) => entities.push(Entity::Struct(s)),
            Err(d) => diagnostics.push(d),
        },
        "enum_item" => match extract_enum(node, source, file_path, language) {
            Ok(e) => entities.push(Entity::Enum(e)),
            Err(d) => diagnostics.push(d),
        },
        "trait_item" => match extract_trait(node, source, file_path, language) {
            Ok(t) => entities.push(Entity::Trait(t)),
            Err(d) => diagnostics.push(d),
        },
        "impl_item" => match extract_impl(node, source, file_path, language) {
            Ok(i) => entities.push(Entity::ImplBlock(i)),
            Err(d) => diagnostics.push(d),
        },
        "type_item" => match extract_type_alias(node, source, file_path, language) {
            Ok(t) => entities.push(Entity::TypeAlias(t)),
            Err(d) => diagnostics.push(d),
        },
        "const_item" => match extract_constant(node, source, file_path, language) {
            Ok(c) => entities.push(Entity::Constant(c)),
            Err(d) => diagnostics.push(d),
        },
        "static_item" => match extract_static(node, source, file_path, language) {
            Ok(s) => entities.push(Entity::Static(s)),
            Err(d) => diagnostics.push(d),
        },
        "use_declaration" => match extract_import(node, source, file_path, language) {
            Ok(i) => entities.push(Entity::Import(i)),
            Err(d) => diagnostics.push(d),
        },
        "mod_item" => match extract_module(node, source, file_path, language) {
            Ok(m) => entities.push(Entity::Module(m)),
            Err(d) => diagnostics.push(d),
        },
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Evidence helpers
// ---------------------------------------------------------------------------

fn evidence_for_node(
    node: tree_sitter::Node,
    _source: &str,
    file_path: &Path,
    language: &Language,
) -> Evidence {
    Evidence::new(
        file_path.to_path_buf(),
        node.kind(),
        node.start_byte()..node.end_byte(),
        node.start_position().row + 1,
        node.start_position().column + 1,
        node.end_position().row + 1,
        node.end_position().column + 1,
        language.clone(),
    )
}

fn node_text<'a>(node: tree_sitter::Node<'a>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

fn evidence_for_node_with_kind(
    node: tree_sitter::Node,
    _source: &str,
    file_path: &Path,
    language: &Language,
    kind: &str,
) -> Evidence {
    Evidence::new(
        file_path.to_path_buf(),
        kind,
        node.start_byte()..node.end_byte(),
        node.start_position().row + 1,
        node.start_position().column + 1,
        node.end_position().row + 1,
        node.end_position().column + 1,
        language.clone(),
    )
}

// ---------------------------------------------------------------------------
// Visibility extraction
// ---------------------------------------------------------------------------

fn extract_visibility(node: tree_sitter::Node, source: &str) -> Visibility {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            let text = node_text(child, source);
            return match text {
                "pub" => Visibility::Public,
                "pub(crate)" => Visibility::Crate,
                "pub(super)" => Visibility::Super,
                _ if text.starts_with("pub(in ") => {
                    let inner = text.trim_start_matches("pub(in ").trim_end_matches(')');
                    Visibility::Restricted(inner.to_string())
                }
                _ => Visibility::Public,
            };
        }
    }
    Visibility::Private
}

fn has_keyword(node: tree_sitter::Node, source: &str, keyword: &str) -> bool {
    let text = &source[node.start_byte()..node.end_byte()];
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|w| w == keyword)
}

// ---------------------------------------------------------------------------
// Documentation extraction
// ---------------------------------------------------------------------------

fn extract_doc_comments(node: tree_sitter::Node, source: &str) -> Option<String> {
    if let Some(prev) = node.prev_sibling() {
        let comment_text = node_text(prev, source);
        if comment_text.starts_with("///") || comment_text.starts_with("/**") {
            let cleaned = clean_doc_comment(comment_text);
            return Some(cleaned);
        }
    }
    None
}

fn clean_doc_comment(text: &str) -> String {
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("///") {
                trimmed.trim_start_matches("///").trim_start()
            } else if trimmed.starts_with("//!") {
                trimmed.trim_start_matches("//!").trim_start()
            } else if trimmed.starts_with("*") && !trimmed.starts_with("**") {
                trimmed.trim_start_matches('*').trim_start()
            } else {
                trimmed
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

// ---------------------------------------------------------------------------
// Signature extraction
// ---------------------------------------------------------------------------

/// Extract the function signature text (everything from `fn` to just before the body).
fn extract_signature(node: tree_sitter::Node, source: &str) -> Option<String> {
    let body = node.child_by_field_name("body");
    let fn_keyword_pos = node
        .children(&mut node.walk())
        .find(|c| c.kind() == "fn")
        .map(|c| c.start_byte());

    let start = fn_keyword_pos?;
    let end = body.map_or(node.end_byte(), |b| b.start_byte());
    let sig_text = &source.as_bytes()[start..end];
    let sig = String::from_utf8_lossy(sig_text).trim().to_string();
    if sig.is_empty() {
        None
    } else {
        Some(sig)
    }
}

// ---------------------------------------------------------------------------
// Generics extraction
// ---------------------------------------------------------------------------

fn extract_generics(node: tree_sitter::Node, source: &str) -> Vec<String> {
    let mut generics = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameters" {
            for param in child.children(&mut child.walk()) {
                if param.kind() == "type_parameter" {
                    generics.push(node_text(param, source).to_string());
                }
            }
        }
    }
    generics
}

// ---------------------------------------------------------------------------
// Entity extraction helpers
// ---------------------------------------------------------------------------

fn extract_module(
    node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
) -> Result<ModuleFact, ExtractionDiagnostic> {
    let name_node = node
        .child_by_field_name("name")
        .ok_or_else(|| diagnostic_for(node, source, file_path, language, "module has no name"))?;
    let name = node_text(name_node, source);

    let visibility = extract_visibility(node, source);
    let evidence = evidence_for_node(node, source, file_path, language);
    let id = EntityId::from_location(language, "module", file_path, name, node.start_byte());

    Ok(ModuleFact::new(id, name, visibility, evidence))
}

fn extract_struct(
    node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
) -> Result<StructFact, ExtractionDiagnostic> {
    let name_node = node
        .child_by_field_name("name")
        .ok_or_else(|| diagnostic_for(node, source, file_path, language, "struct has no name"))?;
    let name = node_text(name_node, source);

    let visibility = extract_visibility(node, source);
    let generics = extract_generics(node, source);

    let mut fields = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for field_node in body.children(&mut cursor) {
            if field_node.kind() == "field_declaration" {
                if let Some(f) = extract_struct_field(field_node, source, file_path, language) {
                    fields.push(f);
                }
            }
        }
    }

    let evidence = evidence_for_node(node, source, file_path, language);
    let id = EntityId::from_location(language, "struct", file_path, name, node.start_byte());

    Ok(StructFact::new(
        id, name, fields, visibility, generics, evidence,
    ))
}

fn extract_struct_field(
    node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
) -> Option<StructField> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source);

    let field_type = node
        .children(&mut node.walk())
        .find(|c| {
            c.kind() == "type_identifier"
                || c.kind() == "generic_type"
                || c.kind() == "array_type"
                || c.kind() == "reference_type"
        })
        .map(|t| node_text(t, source).to_string());

    let visibility = extract_visibility(node, source);
    let evidence = evidence_for_node(node, source, file_path, language);
    let id = EntityId::from_location(language, "field", file_path, name, node.start_byte());

    Some(StructField::new(id, name, field_type, visibility, evidence))
}

fn extract_enum(
    node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
) -> Result<EnumFact, ExtractionDiagnostic> {
    let name_node = node
        .child_by_field_name("name")
        .ok_or_else(|| diagnostic_for(node, source, file_path, language, "enum has no name"))?;
    let name = node_text(name_node, source);

    let visibility = extract_visibility(node, source);

    let mut variants = Vec::new();
    for child in node.children(&mut node.walk()) {
        match child.kind() {
            "enum_variant_list" => {
                for variant_child in child.children(&mut child.walk()) {
                    if variant_child.kind() == "enum_variant" {
                        if let Some(v) =
                            extract_enum_variant(variant_child, source, file_path, language)
                        {
                            variants.push(v);
                        }
                    }
                }
            }
            "enum_variant" => {
                if let Some(v) = extract_enum_variant(child, source, file_path, language) {
                    variants.push(v);
                }
            }
            _ => {}
        }
    }

    let evidence = evidence_for_node(node, source, file_path, language);
    let id = EntityId::from_location(language, "enum", file_path, name, node.start_byte());

    Ok(EnumFact::new(id, name, variants, visibility, evidence))
}

fn extract_enum_variant(
    node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
) -> Option<EnumVariant> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source);

    let discriminant = node
        .children(&mut node.walk())
        .find(|c| c.kind() == "discriminant")
        .map(|d| node_text(d, source).to_string());

    let evidence = evidence_for_node(node, source, file_path, language);
    let id = EntityId::from_location(language, "enum_variant", file_path, name, node.start_byte());

    Some(EnumVariant::new(id, name, discriminant, evidence))
}

fn extract_function(
    node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
    containing_trait: Option<EntityId>,
    containing_impl: Option<EntityId>,
) -> Result<FunctionFact, ExtractionDiagnostic> {
    let name_node = node
        .child_by_field_name("name")
        .ok_or_else(|| diagnostic_for(node, source, file_path, language, "function has no name"))?;
    let name = node_text(name_node, source);

    let visibility = extract_visibility(node, source);
    let is_async = has_keyword(node, source, "async");
    let is_const = has_keyword(node, source, "const");
    let is_unsafe = has_keyword(node, source, "unsafe");
    let generics = extract_generics(node, source);
    let signature = extract_signature(node, source);
    let documentation = extract_doc_comments(node, source);

    let evidence = evidence_for_node(node, source, file_path, language);
    let id = EntityId::from_location(language, "function", file_path, name, node.start_byte());

    let calls = extract_calls_in_function(node, source, file_path, language);

    Ok(FunctionFact::new(
        id,
        name,
        visibility,
        signature,
        generics,
        is_async,
        is_const,
        is_unsafe,
        documentation,
        containing_trait,
        containing_impl,
        evidence,
    )
    .with_calls(calls))
}

// ---------------------------------------------------------------------------
// Call expression extraction
// ---------------------------------------------------------------------------

/// Walk a function body's AST and collect call sites.
fn extract_calls_in_function(
    function_node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
) -> Vec<CallSite> {
    let Some(body) = function_node.child_by_field_name("body") else {
        return Vec::new();
    };

    let mut calls = Vec::new();
    let mut stack = vec![body];
    while let Some(node) = stack.pop() {
        if node.kind() == "call_expression" {
            if let Some(site) = call_site_from_expression(node, source, file_path, language) {
                calls.push(site);
            }
        }
        // Nested functions/closures: still collect calls (attributed to outer
        // function for MVP — good enough for call graph exploration).
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
    // Deterministic order by source byte range.
    calls.sort_by(|a, b| {
        a.evidence()
            .byte_range()
            .start
            .cmp(&b.evidence().byte_range().start)
            .then_with(|| a.callee_name().cmp(b.callee_name()))
    });
    calls
}

fn call_site_from_expression(
    node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
) -> Option<CallSite> {
    let func = node.child_by_field_name("function")?;
    let (name, path, is_method) = resolve_callee(func, source)?;
    if name.is_empty() {
        return None;
    }
    let evidence =
        evidence_for_node_with_kind(node, source, file_path, language, "call_expression");
    Some(CallSite::new(name, path, is_method, evidence))
}

/// Resolve the callee of a call_expression's `function` child.
fn resolve_callee(node: tree_sitter::Node, source: &str) -> Option<(String, Option<String>, bool)> {
    match node.kind() {
        "identifier" => {
            let name = node_text(node, source).to_string();
            Some((name, None, false))
        }
        "scoped_identifier" => {
            let full = node_text(node, source).to_string();
            let name = full.rsplit("::").next().unwrap_or(&full).to_string();
            Some((name, Some(full), false))
        }
        "field_expression" => {
            // receiver.method — field name is the method.
            let field = node.child_by_field_name("field")?;
            let name = node_text(field, source).to_string();
            let full = node_text(node, source).to_string();
            Some((name, Some(full), true))
        }
        "generic_function" => {
            // foo::<T> — recurse into function child.
            let inner = node.child_by_field_name("function")?;
            resolve_callee(inner, source)
        }
        _ => {
            // Fallback: last identifier-like token in the text.
            let text = node_text(node, source);
            let name = text
                .rsplit(|c: char| !c.is_alphanumeric() && c != '_')
                .find(|s| !s.is_empty())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                None
            } else {
                Some((name, Some(text.to_string()), false))
            }
        }
    }
}

fn extract_trait(
    node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
) -> Result<TraitFact, ExtractionDiagnostic> {
    let name_node = node
        .child_by_field_name("name")
        .ok_or_else(|| diagnostic_for(node, source, file_path, language, "trait has no name"))?;
    let name = node_text(name_node, source);

    let visibility = extract_visibility(node, source);

    let trait_id = EntityId::from_location(language, "trait", file_path, name, node.start_byte());
    let mut method_ids = Vec::new();
    let mut type_alias_ids = Vec::new();
    let mut const_ids = Vec::new();

    let collect_trait_items = |inner_node: tree_sitter::Node,
                               methods: &mut Vec<EntityId>,
                               types: &mut Vec<EntityId>,
                               consts: &mut Vec<EntityId>| {
        let mut cursor = inner_node.walk();
        for child in inner_node.children(&mut cursor) {
            match child.kind() {
                "function_signature" | "function_item" => {
                    if let Ok(f) =
                        extract_function(child, source, file_path, language, Some(trait_id), None)
                    {
                        methods.push(*f.id());
                    }
                }
                "type_item" => {
                    if let Ok(t) = extract_type_alias(child, source, file_path, language) {
                        types.push(*t.id());
                    }
                }
                "const_item" => {
                    if let Ok(c) = extract_constant(child, source, file_path, language) {
                        consts.push(*c.id());
                    }
                }
                "declaration_list" | "trait_body" => {
                    let mut inner_cursor = child.walk();
                    for inner in child.children(&mut inner_cursor) {
                        match inner.kind() {
                            "function_signature" | "function_item" => {
                                if let Ok(f) = extract_function(
                                    inner,
                                    source,
                                    file_path,
                                    language,
                                    Some(trait_id),
                                    None,
                                ) {
                                    methods.push(*f.id());
                                }
                            }
                            "type_item" => {
                                if let Ok(t) =
                                    extract_type_alias(inner, source, file_path, language)
                                {
                                    types.push(*t.id());
                                }
                            }
                            "const_item" => {
                                if let Ok(c) = extract_constant(inner, source, file_path, language)
                                {
                                    consts.push(*c.id());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    };
    collect_trait_items(node, &mut method_ids, &mut type_alias_ids, &mut const_ids);

    let evidence = evidence_for_node(node, source, file_path, language);

    Ok(TraitFact::new(
        trait_id,
        name,
        visibility,
        method_ids,
        type_alias_ids,
        const_ids,
        evidence,
    ))
}

fn extract_impl(
    node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
) -> Result<ImplBlockFact, ExtractionDiagnostic> {
    let target_type = node
        .child_by_field_name("type")
        .map(|t| node_text(t, source).to_string())
        .unwrap_or_default();

    let implemented_trait = node
        .child_by_field_name("trait")
        .map(|t| node_text(t, source).to_string());

    let impl_type_name = implemented_trait
        .as_ref()
        .map(|t| format!("impl {} for {}", t, target_type))
        .unwrap_or_else(|| format!("impl {}", target_type));

    let impl_id = EntityId::from_location(
        language,
        "impl",
        file_path,
        &impl_type_name,
        node.start_byte(),
    );
    let mut method_ids = Vec::new();

    let collect_impl_fns = |node: tree_sitter::Node, methods: &mut Vec<EntityId>| {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "function_item" {
                if let Ok(f) =
                    extract_function(child, source, file_path, language, None, Some(impl_id))
                {
                    methods.push(*f.id());
                }
            }
            // Recurse into body/declaration_list nodes
            if child.kind() == "declaration_list" || child.kind() == "block" {
                let mut inner_cursor = child.walk();
                for inner in child.children(&mut inner_cursor) {
                    if inner.kind() == "function_item" {
                        if let Ok(f) = extract_function(
                            inner,
                            source,
                            file_path,
                            language,
                            None,
                            Some(impl_id),
                        ) {
                            methods.push(*f.id());
                        }
                    }
                }
            }
        }
    };
    collect_impl_fns(node, &mut method_ids);

    let evidence = evidence_for_node(node, source, file_path, language);

    Ok(ImplBlockFact::new(
        impl_id,
        target_type,
        implemented_trait,
        method_ids,
        evidence,
    ))
}

fn extract_type_alias(
    node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
) -> Result<TypeAliasFact, ExtractionDiagnostic> {
    let name_node = node.child_by_field_name("name").ok_or_else(|| {
        diagnostic_for(node, source, file_path, language, "type alias has no name")
    })?;
    let name = node_text(name_node, source);

    let aliased_type = node
        .child_by_field_name("type")
        .map(|t| node_text(t, source).to_string());

    let evidence = evidence_for_node(node, source, file_path, language);
    let id = EntityId::from_location(language, "type_alias", file_path, name, node.start_byte());

    Ok(TypeAliasFact::new(id, name, aliased_type, evidence))
}

fn extract_constant(
    node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
) -> Result<ConstantFact, ExtractionDiagnostic> {
    let name_node = node
        .child_by_field_name("name")
        .ok_or_else(|| diagnostic_for(node, source, file_path, language, "constant has no name"))?;
    let name = node_text(name_node, source);

    let const_type = node
        .child_by_field_name("type")
        .map(|t| node_text(t, source).to_string());

    let value = node
        .child_by_field_name("value")
        .map(|v| node_text(v, source).to_string());

    let evidence = evidence_for_node(node, source, file_path, language);
    let id = EntityId::from_location(language, "constant", file_path, name, node.start_byte());

    Ok(ConstantFact::new(id, name, const_type, value, evidence))
}

fn extract_static(
    node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
) -> Result<StaticFact, ExtractionDiagnostic> {
    let name_node = node
        .child_by_field_name("name")
        .ok_or_else(|| diagnostic_for(node, source, file_path, language, "static has no name"))?;
    let name = node_text(name_node, source);

    let static_type = node
        .child_by_field_name("type")
        .map(|t| node_text(t, source).to_string());

    let is_mutable = has_keyword(node, source, "mut");

    let evidence = evidence_for_node(node, source, file_path, language);
    let id = EntityId::from_location(language, "static", file_path, name, node.start_byte());

    Ok(StaticFact::new(id, name, static_type, is_mutable, evidence))
}

fn extract_import(
    node: tree_sitter::Node,
    source: &str,
    file_path: &Path,
    language: &Language,
) -> Result<ImportFact, ExtractionDiagnostic> {
    let path_text = node_text(node, source);

    let is_glob = path_text.contains('*');
    let alias = extract_import_alias(node, source, path_text);

    let import_path = if alias.is_some() {
        path_text
            .split(" as ")
            .next()
            .unwrap_or(path_text)
            .trim()
            .to_string()
    } else {
        path_text.to_string()
    };

    let evidence =
        evidence_for_node_with_kind(node, source, file_path, language, "use_declaration");
    let id = EntityId::from_location(
        language,
        "import",
        file_path,
        &import_path,
        node.start_byte(),
    );

    Ok(ImportFact::new(id, import_path, alias, is_glob, evidence))
}

fn extract_import_alias(node: tree_sitter::Node, source: &str, path_text: &str) -> Option<String> {
    if let Some(as_node) = find_child_by_kind(node, "use_as_clause") {
        return as_node
            .children(&mut as_node.walk())
            .find(|c| {
                c.kind() == "identifier"
                    || c.kind() == "self_keyword"
                    || c.kind() == "crate_keyword"
                    || c.kind() == "super_keyword"
            })
            .map(|c| node_text(c, source).to_string());
    }
    if path_text.contains(" as ") {
        return path_text.split(" as ").nth(1).map(|s| s.trim().to_string());
    }
    None
}

fn find_child_by_kind<'a>(
    node: tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).find(|c| c.kind() == kind);
    result
}

fn diagnostic_for(
    node: tree_sitter::Node,
    _source: &str,
    file_path: &Path,
    _language: &Language,
    message: &str,
) -> ExtractionDiagnostic {
    ExtractionDiagnostic::warning(
        message,
        Some(file_path.to_path_buf()),
        node.start_position().row + 1,
        node.start_position().column + 1,
        node.end_position().row + 1,
        node.end_position().column + 1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use kode_acquisition::Language;

    use crate::parsing::{self, Severity};

    fn parse_rust(source: &str) -> SyntaxTree {
        let (backend, diagnostics) = parsing::ts::parse_source(Language::Rust, source).unwrap();
        let has_errors = diagnostics.iter().any(|d| *d.severity() == Severity::Error);
        SyntaxTree::new(
            PathBuf::from("test.rs"),
            Language::Rust,
            Arc::from(source),
            has_errors,
            diagnostics,
            "tree-sitter-rust".to_string(),
            None,
            None,
            Some("tree-sitter".to_string()),
            backend,
        )
    }

    #[test]
    fn extract_functions() {
        let tree = parse_rust(
            r#"
            fn foo() {}
            pub fn bar(x: i32) -> i32 { x }
            pub(crate) fn baz<T>(x: T) -> T { x }
            "#,
        );
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let names: Vec<&str> = entities
                    .iter()
                    .filter_map(|e| match e {
                        Entity::Function(f) => Some(f.name()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(names, vec!["foo", "bar", "baz"]);
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_visibility() {
        let tree = parse_rust(
            r#"
            fn private() {}
            pub fn public() {}
            "#,
        );
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let funcs: Vec<&FunctionFact> = entities
                    .iter()
                    .filter_map(|e| match e {
                        Entity::Function(f) => Some(f),
                        _ => None,
                    })
                    .collect();
                assert_eq!(funcs[0].visibility(), &Visibility::Private);
                assert_eq!(funcs[1].visibility(), &Visibility::Public);
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_async_function() {
        let tree = parse_rust("pub async fn fetch() {}");
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let f = entities
                    .iter()
                    .find_map(|e| match e {
                        Entity::Function(f) => Some(f),
                        _ => None,
                    })
                    .unwrap();
                assert!(f.is_async());
                assert_eq!(f.name(), "fetch");
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_unsafe_function() {
        let tree = parse_rust("pub unsafe fn dangerous() {}");
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let f = entities
                    .iter()
                    .find_map(|e| match e {
                        Entity::Function(f) => Some(f),
                        _ => None,
                    })
                    .unwrap();
                assert!(f.is_unsafe());
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_const_function() {
        let tree = parse_rust("pub const fn compute() {}");
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let f = entities
                    .iter()
                    .find_map(|e| match e {
                        Entity::Function(f) => Some(f),
                        _ => None,
                    })
                    .unwrap();
                assert!(f.is_const());
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_structs() {
        let tree = parse_rust(
            r#"
            struct Empty;
            pub struct Point {
                pub x: i32,
                y: f64,
            }
            "#,
        );
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let structs: Vec<&StructFact> = entities
                    .iter()
                    .filter_map(|e| match e {
                        Entity::Struct(s) => Some(s),
                        _ => None,
                    })
                    .collect();
                assert_eq!(structs.len(), 2);
                assert_eq!(structs[0].name(), "Empty");
                assert_eq!(structs[1].name(), "Point");
                assert_eq!(structs[1].fields().len(), 2);
                assert_eq!(structs[1].fields()[0].name(), "x");
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_enums() {
        let tree = parse_rust(
            r#"
            enum Color {
                Red,
                Green,
                Blue,
            }
            "#,
        );
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let enums: Vec<&EnumFact> = entities
                    .iter()
                    .filter_map(|e| match e {
                        Entity::Enum(s) => Some(s),
                        _ => None,
                    })
                    .collect();
                assert_eq!(enums.len(), 1);
                assert_eq!(enums[0].name(), "Color");
                assert_eq!(enums[0].variants().len(), 3);
                assert_eq!(enums[0].variants()[0].name(), "Red");
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_type_aliases() {
        let tree = parse_rust("type MyResult<T> = Result<T, String>;");
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let aliases: Vec<&TypeAliasFact> = entities
                    .iter()
                    .filter_map(|e| match e {
                        Entity::TypeAlias(t) => Some(t),
                        _ => None,
                    })
                    .collect();
                assert_eq!(aliases.len(), 1);
                assert_eq!(aliases[0].name(), "MyResult");
                assert!(aliases[0].aliased_type().is_some());
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_constants() {
        let tree = parse_rust("const MAX_SIZE: usize = 1024;");
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let consts: Vec<&ConstantFact> = entities
                    .iter()
                    .filter_map(|e| match e {
                        Entity::Constant(c) => Some(c),
                        _ => None,
                    })
                    .collect();
                assert_eq!(consts.len(), 1);
                assert_eq!(consts[0].name(), "MAX_SIZE");
                assert_eq!(consts[0].const_type(), Some("usize"));
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_statics() {
        let tree = parse_rust("static LOG_LEVEL: &str = \"info\";\nstatic mut COUNTER: i32 = 0;");
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let statics: Vec<&StaticFact> = entities
                    .iter()
                    .filter_map(|e| match e {
                        Entity::Static(s) => Some(s),
                        _ => None,
                    })
                    .collect();
                assert_eq!(statics.len(), 2);
                assert!(!statics[0].is_mutable());
                assert!(statics[1].is_mutable());
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_imports() {
        let tree = parse_rust(
            r#"
            use std::collections::HashMap;
            use std::io::{self, Write};
            use crate::utils::*;
            "#,
        );
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let imports: Vec<&ImportFact> = entities
                    .iter()
                    .filter_map(|e| match e {
                        Entity::Import(i) => Some(i),
                        _ => None,
                    })
                    .collect();
                assert!(
                    imports.len() >= 2,
                    "expected at least 2 imports, got {}",
                    imports.len()
                );
                let glob_imports: Vec<&&ImportFact> =
                    imports.iter().filter(|i| i.is_glob()).collect();
                assert_eq!(glob_imports.len(), 1);
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_traits() {
        let tree = parse_rust(
            r#"
            pub trait Iterable {
                type Item;
                fn next(&self) -> Option<Self::Item>;
                fn count(&self) -> usize { 0 }
            }
            "#,
        );
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let traits: Vec<&TraitFact> = entities
                    .iter()
                    .filter_map(|e| match e {
                        Entity::Trait(t) => Some(t),
                        _ => None,
                    })
                    .collect();
                assert_eq!(traits.len(), 1);
                assert_eq!(traits[0].name(), "Iterable");
                assert!(!traits[0].methods().is_empty());
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_impl_blocks() {
        let tree = parse_rust(
            r#"
            struct Foo;
            impl Foo {
                fn bar(&self) {}
                fn baz(&self) {}
            }
            "#,
        );
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let impls: Vec<&ImplBlockFact> = entities
                    .iter()
                    .filter_map(|e| match e {
                        Entity::ImplBlock(i) => Some(i),
                        _ => None,
                    })
                    .collect();
                assert_eq!(impls.len(), 1);
                assert_eq!(impls[0].target_type(), "Foo");
                assert!(impls[0].implemented_trait().is_none());
                assert_eq!(impls[0].methods().len(), 2);
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_evidence_attached() {
        let tree = parse_rust("pub fn foo() {}");
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let f = entities
                    .iter()
                    .find_map(|e| match e {
                        Entity::Function(f) => Some(f),
                        _ => None,
                    })
                    .unwrap();
                let ev = f.evidence();
                assert_eq!(ev.source_file(), Path::new("test.rs"));
                assert_eq!(ev.node_kind(), "function_item");
                assert_eq!(ev.language(), &Language::Rust);
                assert!(ev.start_line() > 0);
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_empty_file() {
        let tree = parse_rust("");
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                assert!(entities.is_empty());
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_generic_function() {
        let tree = parse_rust("fn identity<T: Clone>(x: T) -> T { x }");
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let f = entities
                    .iter()
                    .find_map(|e| match e {
                        Entity::Function(f) => Some(f),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(f.name(), "identity");
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn extract_call_sites_in_function_body() {
        let tree = parse_rust(
            r#"
            fn helper() {}
            fn caller() {
                helper();
                crate::helper();
                x.method();
            }
            "#,
        );
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let caller = entities
                    .iter()
                    .find_map(|e| match e {
                        Entity::Function(f) if f.name() == "caller" => Some(f),
                        _ => None,
                    })
                    .expect("caller function");
                let names: Vec<&str> = caller.calls().iter().map(|c| c.callee_name()).collect();
                assert!(
                    names.contains(&"helper"),
                    "expected helper call, got {names:?}"
                );
                assert!(
                    names.contains(&"method"),
                    "expected method call, got {names:?}"
                );
                assert!(
                    caller.calls().iter().any(|c| c.is_method()),
                    "expected a method call site"
                );
            }
            other => panic!("expected Success, got {other:?}"),
        }
    }

    #[test]
    fn extract_modules() {
        let tree = parse_rust(
            r#"
            mod foo;
            pub mod bar {
                pub fn inside() {}
            }
            "#,
        );
        let extractor = RustExtractor;
        let outcome = extractor.extract(&tree);
        match outcome {
            ExtractionOutcome::Success(entities) => {
                let modules: Vec<&ModuleFact> = entities
                    .iter()
                    .filter_map(|e| match e {
                        Entity::Module(m) => Some(m),
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    modules.len(),
                    2,
                    "expected 2 modules, got {}",
                    modules.len()
                );
                assert_eq!(modules[0].name(), "foo");
                assert_eq!(modules[1].name(), "bar");
                assert_eq!(modules[1].visibility(), &Visibility::Public);
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn deterministic_extraction() {
        let source = r#"
            fn b() {}
            fn a() {}
            fn c() {}
        "#;
        let tree1 = parse_rust(source);
        let tree2 = parse_rust(source);
        let extractor = RustExtractor;

        let outcome1 = extractor.extract(&tree1);
        let outcome2 = extractor.extract(&tree2);

        let names1: Vec<String> = match outcome1 {
            ExtractionOutcome::Success(ref entities) => entities
                .iter()
                .filter_map(|e| match e {
                    Entity::Function(f) => Some(f.name().to_string()),
                    _ => None,
                })
                .collect(),
            _ => panic!("expected Success"),
        };

        let names2: Vec<String> = match outcome2 {
            ExtractionOutcome::Success(ref entities) => entities
                .iter()
                .filter_map(|e| match e {
                    Entity::Function(f) => Some(f.name().to_string()),
                    _ => None,
                })
                .collect(),
            _ => panic!("expected Success"),
        };

        assert_eq!(names1, names2);
    }
}
