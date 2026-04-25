use std::{
    env, fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepSeekApiKeySource {
    Environment,
    UserSecret,
    Missing,
}

#[derive(Debug, Error)]
pub enum SecretStoreError {
    #[error("HOME is not set; cannot resolve user secret store")]
    HomeUnavailable,
    #[error("DeepSeek API key cannot be empty")]
    EmptyApiKey,
    #[error("failed to create secret directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write secret file {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to read secret file {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to set permissions on {path}: {source}")]
    SetPermissions {
        path: PathBuf,
        source: std::io::Error,
    },
}

pub fn resolve_deepseek_api_key() -> Option<String> {
    env_deepseek_api_key().or_else(|| stored_deepseek_api_key().ok().flatten())
}

pub fn deepseek_api_key_source() -> DeepSeekApiKeySource {
    if env_deepseek_api_key().is_some() {
        DeepSeekApiKeySource::Environment
    } else if stored_deepseek_api_key().ok().flatten().is_some() {
        DeepSeekApiKeySource::UserSecret
    } else {
        DeepSeekApiKeySource::Missing
    }
}

pub fn store_deepseek_api_key(key: &str) -> Result<PathBuf, SecretStoreError> {
    let path = deepseek_api_key_secret_path()?;
    store_deepseek_api_key_at(&path, key)?;
    Ok(path)
}

pub fn stored_deepseek_api_key() -> Result<Option<String>, SecretStoreError> {
    read_deepseek_api_key_at(&deepseek_api_key_secret_path()?)
}

pub fn deepseek_api_key_secret_path() -> Result<PathBuf, SecretStoreError> {
    Ok(whale_secret_home_dir()?
        .join("secrets")
        .join("deepseek_api_key"))
}

pub fn whale_secret_home_dir() -> Result<PathBuf, SecretStoreError> {
    if let Some(path) = env::var_os("WHALE_SECRET_HOME") {
        Ok(PathBuf::from(path))
    } else if let Some(home) = env::var_os("HOME") {
        Ok(PathBuf::from(home).join(".whale"))
    } else {
        Err(SecretStoreError::HomeUnavailable)
    }
}

pub fn store_deepseek_api_key_at(path: &Path, key: &str) -> Result<(), SecretStoreError> {
    let key = key.trim();
    if key.is_empty() {
        return Err(SecretStoreError::EmptyApiKey);
    }
    let Some(parent) = path.parent() else {
        return Err(SecretStoreError::CreateDir {
            path: path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "secret path has no parent directory",
            ),
        });
    };
    fs::create_dir_all(parent).map_err(|source| SecretStoreError::CreateDir {
        path: parent.to_path_buf(),
        source,
    })?;
    harden_dir(parent)?;

    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, key).map_err(|source| SecretStoreError::Write {
        path: temp_path.clone(),
        source,
    })?;
    harden_file(&temp_path)?;
    fs::rename(&temp_path, path).map_err(|source| SecretStoreError::Write {
        path: path.to_path_buf(),
        source,
    })?;
    harden_file(path)?;
    Ok(())
}

pub fn read_deepseek_api_key_at(path: &Path) -> Result<Option<String>, SecretStoreError> {
    match fs::read_to_string(path) {
        Ok(value) => {
            let value = value.trim().to_owned();
            Ok((!value.is_empty()).then_some(value))
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(SecretStoreError::Read {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn env_deepseek_api_key() -> Option<String> {
    env::var("DEEPSEEK_API_KEY")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(unix)]
fn harden_dir(path: &Path) -> Result<(), SecretStoreError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|source| {
        SecretStoreError::SetPermissions {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn harden_dir(_path: &Path) -> Result<(), SecretStoreError> {
    Ok(())
}

#[cfg(unix)]
fn harden_file(path: &Path) -> Result<(), SecretStoreError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|source| {
        SecretStoreError::SetPermissions {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn harden_file(_path: &Path) -> Result<(), SecretStoreError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_loads_deepseek_api_key_with_private_permissions() {
        let dir = tempfile::tempdir().expect("dir");
        let path = dir.path().join("secrets/deepseek_api_key");

        store_deepseek_api_key_at(&path, "  test-key  ").expect("store key");

        assert_eq!(
            read_deepseek_api_key_at(&path).expect("read key"),
            Some("test-key".to_owned())
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn empty_secret_file_is_treated_as_missing() {
        let dir = tempfile::tempdir().expect("dir");
        let path = dir.path().join("deepseek_api_key");
        fs::write(&path, "  \n").expect("write empty secret");

        assert_eq!(read_deepseek_api_key_at(&path).expect("read key"), None);
    }
}
