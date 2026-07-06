use std::path::{Path, PathBuf};

use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepositoryId(String);

impl RepositoryId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub trait RepositoryIdentityService {
    fn generate_id(&self, root: &Path) -> RepositoryId;
}

#[derive(Debug, Clone)]
pub struct Repository {
    identity: Option<RepositoryId>,
    root: PathBuf,
}

impl Repository {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();

        let canonical = path.canonicalize().map_err(|source| {
            if !path.exists() {
                Error::NotFound(path.to_path_buf())
            } else {
                Error::Canonicalization {
                    path: path.to_path_buf(),
                    source,
                }
            }
        })?;

        if !canonical.is_dir() {
            return Err(Error::NotADirectory(canonical));
        }

        Ok(Self {
            identity: None,
            root: canonical,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn identity(&self) -> Option<&RepositoryId> {
        self.identity.as_ref()
    }

    pub fn set_identity(&mut self, id: RepositoryId) {
        self.identity = Some(id);
    }

    pub fn with_identity(mut self, id: RepositoryId) -> Self {
        self.identity = Some(id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct PathIdentityProvider;

    impl RepositoryIdentityService for PathIdentityProvider {
        fn generate_id(&self, root: &Path) -> RepositoryId {
            RepositoryId(root.to_string_lossy().to_string())
        }
    }

    #[test]
    fn valid_path() {
        let repo = Repository::new(".").unwrap();
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(repo.root(), cwd.canonicalize().unwrap());
    }

    #[test]
    fn missing_path() {
        let err = Repository::new("/nonexistent/path/xyz123").unwrap_err();
        assert!(matches!(err, Error::NotFound(_)));
    }

    #[test]
    fn non_directory_path() {
        let file = std::env::temp_dir().join("_kode_test_repo_file");
        std::fs::write(&file, "test").ok();
        let err = Repository::new(&file).unwrap_err();
        std::fs::remove_file(&file).ok();
        assert!(matches!(err, Error::NotADirectory(_)));
    }

    #[test]
    fn no_identity_by_default() {
        let repo = Repository::new(".").unwrap();
        assert!(repo.identity().is_none());
    }

    #[test]
    fn assign_identity() {
        let mut repo = Repository::new(".").unwrap();
        let id = RepositoryId::new("test-repo");
        repo.set_identity(id.clone());
        assert_eq!(repo.identity(), Some(&id));
    }

    #[test]
    fn assign_identity_via_service() {
        let repo = Repository::new(".").unwrap();
        let provider = PathIdentityProvider;
        let id = provider.generate_id(repo.root());
        let repo = repo.with_identity(id);
        assert!(repo.identity().is_some());
    }
}
