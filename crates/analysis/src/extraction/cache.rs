//! Serializable cache of [`RepositoryFacts`] for incremental pipeline merges.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use kode_acquisition::Language;

use super::model::*;

const CACHE_VERSION: u32 = 1;

/// Versioned on-disk facts payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactsCache {
    pub version: u32,
    pub entities: Vec<EntityDto>,
}

/// One cached entity (enough to rebuild graph construction inputs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EntityDto {
    Module {
        id: u64,
        name: String,
        visibility: String,
        evidence: EvidenceDto,
    },
    Function {
        id: u64,
        name: String,
        visibility: String,
        signature: Option<String>,
        generics: Vec<String>,
        is_async: bool,
        is_const: bool,
        is_unsafe: bool,
        documentation: Option<String>,
        containing_trait: Option<u64>,
        containing_impl: Option<u64>,
        calls: Vec<CallSiteDto>,
        evidence: EvidenceDto,
    },
    Struct {
        id: u64,
        name: String,
        visibility: String,
        generics: Vec<String>,
        evidence: EvidenceDto,
    },
    Enum {
        id: u64,
        name: String,
        visibility: String,
        evidence: EvidenceDto,
    },
    Trait {
        id: u64,
        name: String,
        visibility: String,
        methods: Vec<u64>,
        evidence: EvidenceDto,
    },
    ImplBlock {
        id: u64,
        target_type: String,
        implemented_trait: Option<String>,
        methods: Vec<u64>,
        evidence: EvidenceDto,
    },
    TypeAlias {
        id: u64,
        name: String,
        aliased_type: Option<String>,
        evidence: EvidenceDto,
    },
    Constant {
        id: u64,
        name: String,
        const_type: Option<String>,
        value: Option<String>,
        evidence: EvidenceDto,
    },
    Static {
        id: u64,
        name: String,
        static_type: Option<String>,
        is_mutable: bool,
        evidence: EvidenceDto,
    },
    Import {
        id: u64,
        path: String,
        alias: Option<String>,
        is_glob: bool,
        evidence: EvidenceDto,
    },
    Export {
        id: u64,
        path: String,
        alias: Option<String>,
        is_reexport: bool,
        evidence: EvidenceDto,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceDto {
    pub source_file: String,
    pub node_kind: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSiteDto {
    pub callee_name: String,
    pub callee_path: Option<String>,
    pub is_method: bool,
    pub evidence: EvidenceDto,
}

impl FactsCache {
    pub fn from_facts(facts: &RepositoryFacts) -> Self {
        let mut entities = Vec::with_capacity(facts.entity_count());
        for m in facts.modules() {
            entities.push(EntityDto::Module {
                id: m.id().as_u64(),
                name: m.name().to_string(),
                visibility: m.visibility().to_string(),
                evidence: EvidenceDto::from(m.evidence()),
            });
        }
        for f in facts.functions() {
            entities.push(EntityDto::Function {
                id: f.id().as_u64(),
                name: f.name().to_string(),
                visibility: f.visibility().to_string(),
                signature: f.signature().map(str::to_string),
                generics: f.generics().to_vec(),
                is_async: f.is_async(),
                is_const: f.is_const(),
                is_unsafe: f.is_unsafe(),
                documentation: f.documentation().map(str::to_string),
                containing_trait: f.containing_trait().map(|id| id.as_u64()),
                containing_impl: f.containing_impl().map(|id| id.as_u64()),
                calls: f
                    .calls()
                    .iter()
                    .map(|c| CallSiteDto {
                        callee_name: c.callee_name().to_string(),
                        callee_path: c.callee_path().map(str::to_string),
                        is_method: c.is_method(),
                        evidence: EvidenceDto::from(c.evidence()),
                    })
                    .collect(),
                evidence: EvidenceDto::from(f.evidence()),
            });
        }
        for s in facts.structs() {
            entities.push(EntityDto::Struct {
                id: s.id().as_u64(),
                name: s.name().to_string(),
                visibility: s.visibility().to_string(),
                generics: s.generics().to_vec(),
                evidence: EvidenceDto::from(s.evidence()),
            });
        }
        for e in facts.enums() {
            entities.push(EntityDto::Enum {
                id: e.id().as_u64(),
                name: e.name().to_string(),
                visibility: e.visibility().to_string(),
                evidence: EvidenceDto::from(e.evidence()),
            });
        }
        for t in facts.traits() {
            entities.push(EntityDto::Trait {
                id: t.id().as_u64(),
                name: t.name().to_string(),
                visibility: t.visibility().to_string(),
                methods: t.methods().iter().map(|id| id.as_u64()).collect(),
                evidence: EvidenceDto::from(t.evidence()),
            });
        }
        for i in facts.impl_blocks() {
            entities.push(EntityDto::ImplBlock {
                id: i.id().as_u64(),
                target_type: i.target_type().to_string(),
                implemented_trait: i.implemented_trait().map(str::to_string),
                methods: i.methods().iter().map(|id| id.as_u64()).collect(),
                evidence: EvidenceDto::from(i.evidence()),
            });
        }
        for a in facts.type_aliases() {
            entities.push(EntityDto::TypeAlias {
                id: a.id().as_u64(),
                name: a.name().to_string(),
                aliased_type: a.aliased_type().map(str::to_string),
                evidence: EvidenceDto::from(a.evidence()),
            });
        }
        for c in facts.constants() {
            entities.push(EntityDto::Constant {
                id: c.id().as_u64(),
                name: c.name().to_string(),
                const_type: c.const_type().map(str::to_string),
                value: c.value().map(str::to_string),
                evidence: EvidenceDto::from(c.evidence()),
            });
        }
        for s in facts.statics() {
            entities.push(EntityDto::Static {
                id: s.id().as_u64(),
                name: s.name().to_string(),
                static_type: s.static_type().map(str::to_string),
                is_mutable: s.is_mutable(),
                evidence: EvidenceDto::from(s.evidence()),
            });
        }
        for imp in facts.imports() {
            entities.push(EntityDto::Import {
                id: imp.id().as_u64(),
                path: imp.path().to_string(),
                alias: imp.alias().map(str::to_string),
                is_glob: imp.is_glob(),
                evidence: EvidenceDto::from(imp.evidence()),
            });
        }
        for ex in facts.exports() {
            entities.push(EntityDto::Export {
                id: ex.id().as_u64(),
                path: ex.path().to_string(),
                alias: ex.alias().map(str::to_string),
                is_reexport: ex.is_reexport(),
                evidence: EvidenceDto::from(ex.evidence()),
            });
        }
        Self {
            version: CACHE_VERSION,
            entities,
        }
    }

    pub fn to_facts(&self) -> Result<RepositoryFacts, String> {
        if self.version != CACHE_VERSION {
            return Err(format!(
                "unsupported facts cache version {} (expected {CACHE_VERSION})",
                self.version
            ));
        }
        let mut entities = Vec::with_capacity(self.entities.len());
        for dto in &self.entities {
            entities.push(dto.to_entity()?);
        }
        Ok(RepositoryFacts::from_entities(entities, Vec::new()))
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

impl EvidenceDto {
    fn from(ev: &Evidence) -> Self {
        Self {
            source_file: ev.source_file().display().to_string(),
            node_kind: ev.node_kind().to_string(),
            byte_start: ev.byte_range().start,
            byte_end: ev.byte_range().end,
            start_line: ev.start_line(),
            start_column: ev.start_column(),
            end_line: ev.end_line(),
            end_column: ev.end_column(),
            language: ev.language().to_string(),
        }
    }

    fn to_evidence(&self) -> Result<Evidence, String> {
        let language = self
            .language
            .parse::<Language>()
            .map_err(|e| e.to_string())?;
        Ok(Evidence::new(
            PathBuf::from(&self.source_file),
            self.node_kind.clone(),
            self.byte_start..self.byte_end,
            self.start_line,
            self.start_column,
            self.end_line,
            self.end_column,
            language,
        ))
    }
}

impl EntityDto {
    fn to_entity(&self) -> Result<Entity, String> {
        match self {
            EntityDto::Module {
                id,
                name,
                visibility,
                evidence,
            } => Ok(Entity::Module(ModuleFact::new(
                EntityId::from_u64(*id),
                name.clone(),
                parse_vis(visibility)?,
                evidence.to_evidence()?,
            ))),
            EntityDto::Function {
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
                calls,
                evidence,
            } => {
                let call_sites: Result<Vec<_>, String> = calls
                    .iter()
                    .map(|c| {
                        Ok(CallSite::new(
                            c.callee_name.clone(),
                            c.callee_path.clone(),
                            c.is_method,
                            c.evidence.to_evidence()?,
                        ))
                    })
                    .collect();
                Ok(Entity::Function(
                    FunctionFact::new(
                        EntityId::from_u64(*id),
                        name.clone(),
                        parse_vis(visibility)?,
                        signature.clone(),
                        generics.clone(),
                        *is_async,
                        *is_const,
                        *is_unsafe,
                        documentation.clone(),
                        containing_trait.map(EntityId::from_u64),
                        containing_impl.map(EntityId::from_u64),
                        evidence.to_evidence()?,
                    )
                    .with_calls(call_sites?),
                ))
            }
            EntityDto::Struct {
                id,
                name,
                visibility,
                generics,
                evidence,
            } => Ok(Entity::Struct(StructFact::new(
                EntityId::from_u64(*id),
                name.clone(),
                Vec::new(),
                parse_vis(visibility)?,
                generics.clone(),
                evidence.to_evidence()?,
            ))),
            EntityDto::Enum {
                id,
                name,
                visibility,
                evidence,
            } => Ok(Entity::Enum(EnumFact::new(
                EntityId::from_u64(*id),
                name.clone(),
                Vec::new(),
                parse_vis(visibility)?,
                evidence.to_evidence()?,
            ))),
            EntityDto::Trait {
                id,
                name,
                visibility,
                methods,
                evidence,
            } => Ok(Entity::Trait(TraitFact::new(
                EntityId::from_u64(*id),
                name.clone(),
                parse_vis(visibility)?,
                methods.iter().copied().map(EntityId::from_u64).collect(),
                Vec::new(),
                Vec::new(),
                evidence.to_evidence()?,
            ))),
            EntityDto::ImplBlock {
                id,
                target_type,
                implemented_trait,
                methods,
                evidence,
            } => Ok(Entity::ImplBlock(ImplBlockFact::new(
                EntityId::from_u64(*id),
                target_type.clone(),
                implemented_trait.clone(),
                methods.iter().copied().map(EntityId::from_u64).collect(),
                evidence.to_evidence()?,
            ))),
            EntityDto::TypeAlias {
                id,
                name,
                aliased_type,
                evidence,
            } => Ok(Entity::TypeAlias(TypeAliasFact::new(
                EntityId::from_u64(*id),
                name.clone(),
                aliased_type.clone(),
                evidence.to_evidence()?,
            ))),
            EntityDto::Constant {
                id,
                name,
                const_type,
                value,
                evidence,
            } => Ok(Entity::Constant(ConstantFact::new(
                EntityId::from_u64(*id),
                name.clone(),
                const_type.clone(),
                value.clone(),
                evidence.to_evidence()?,
            ))),
            EntityDto::Static {
                id,
                name,
                static_type,
                is_mutable,
                evidence,
            } => Ok(Entity::Static(StaticFact::new(
                EntityId::from_u64(*id),
                name.clone(),
                static_type.clone(),
                *is_mutable,
                evidence.to_evidence()?,
            ))),
            EntityDto::Import {
                id,
                path,
                alias,
                is_glob,
                evidence,
            } => Ok(Entity::Import(ImportFact::new(
                EntityId::from_u64(*id),
                path.clone(),
                alias.clone(),
                *is_glob,
                evidence.to_evidence()?,
            ))),
            EntityDto::Export {
                id,
                path,
                alias,
                is_reexport,
                evidence,
            } => Ok(Entity::Export(ExportFact::new(
                EntityId::from_u64(*id),
                path.clone(),
                alias.clone(),
                *is_reexport,
                evidence.to_evidence()?,
            ))),
        }
    }
}

fn parse_vis(s: &str) -> Result<Visibility, String> {
    s.parse()
}

impl RepositoryFacts {
    /// Drop entities whose source file path is listed in `paths`.
    pub fn without_files(&self, paths: &HashSet<PathBuf>) -> Self {
        let drop = |p: &Path| {
            paths
                .iter()
                .any(|x| x == p || x.as_os_str() == p.as_os_str())
        };
        let mut entities = Vec::new();
        macro_rules! keep {
            ($acc:ident, $wrap:ident) => {
                for e in self.$acc() {
                    if !drop(e.evidence().source_file()) {
                        entities.push(Entity::$wrap(e.clone()));
                    }
                }
            };
        }
        keep!(modules, Module);
        keep!(functions, Function);
        keep!(structs, Struct);
        keep!(enums, Enum);
        keep!(traits, Trait);
        keep!(impl_blocks, ImplBlock);
        keep!(type_aliases, TypeAlias);
        keep!(constants, Constant);
        keep!(statics, Static);
        keep!(imports, Import);
        keep!(exports, Export);
        RepositoryFacts::from_entities(entities, self.diagnostics().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn roundtrip_empty() {
        let facts = RepositoryFacts::empty();
        let cache = FactsCache::from_facts(&facts);
        let back = cache.to_facts().unwrap();
        assert_eq!(back.entity_count(), 0);
    }

    #[test]
    fn roundtrip_function_with_calls() {
        let path = PathBuf::from("src/a.rs");
        let ev = Evidence::new(
            path.clone(),
            "function_item",
            0..10,
            1,
            1,
            3,
            1,
            Language::Rust,
        );
        let call_ev = Evidence::new(
            path.clone(),
            "call_expression",
            5..8,
            2,
            1,
            2,
            5,
            Language::Rust,
        );
        let f = FunctionFact::new(
            EntityId::from_u64(1),
            "foo",
            Visibility::Public,
            None,
            Vec::new(),
            false,
            false,
            false,
            None,
            None,
            None,
            ev,
        )
        .with_calls(vec![CallSite::new("bar", None, false, call_ev)]);
        let facts = RepositoryFacts::from_entities(vec![Entity::Function(f)], Vec::new());
        let json = FactsCache::from_facts(&facts).to_json().unwrap();
        let back = FactsCache::from_json(&json).unwrap().to_facts().unwrap();
        assert_eq!(back.functions().len(), 1);
        assert_eq!(back.functions()[0].name(), "foo");
        assert_eq!(back.functions()[0].calls().len(), 1);
        assert_eq!(back.functions()[0].calls()[0].callee_name(), "bar");
    }

    #[test]
    fn without_files_drops_entities() {
        let path = PathBuf::from("src/a.rs");
        let ev = Evidence::new(
            path.clone(),
            "function_item",
            0..10,
            1,
            1,
            1,
            1,
            Language::Rust,
        );
        let f = FunctionFact::new(
            EntityId::from_u64(1),
            "foo",
            Visibility::Private,
            None,
            Vec::new(),
            false,
            false,
            false,
            None,
            None,
            None,
            ev,
        );
        let facts = RepositoryFacts::from_entities(vec![Entity::Function(f)], Vec::new());
        let mut drop_set = HashSet::new();
        drop_set.insert(path);
        let kept = facts.without_files(&drop_set);
        assert_eq!(kept.entity_count(), 0);
    }
}
