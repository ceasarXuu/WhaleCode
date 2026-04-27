use codex_utils_absolute_path::AbsolutePathBuf;
use dirs::home_dir;
use std::path::PathBuf;

/// Returns the path to the Whale configuration directory, which can be
/// specified by the `WHALE_HOME` environment variable. If not set, defaults to
/// `~/.whale`.
///
/// - If `WHALE_HOME` is set, the value must exist and be a directory. The
///   value will be canonicalized and this function will Err otherwise.
/// - If `WHALE_HOME` is not set, this function does not verify that the
///   directory exists.
pub fn find_codex_home() -> std::io::Result<AbsolutePathBuf> {
    let codex_home_env = std::env::var("WHALE_HOME")
        .ok()
        .filter(|val| !val.is_empty());
    find_codex_home_from_env(codex_home_env.as_deref())
}

fn find_codex_home_from_env(codex_home_env: Option<&str>) -> std::io::Result<AbsolutePathBuf> {
    // Honor the `WHALE_HOME` environment variable when it is set to allow users
    // (and tests) to override the default location.
    match codex_home_env {
        Some(val) => {
            let path = PathBuf::from(val);
            let metadata = std::fs::metadata(&path).map_err(|err| match err.kind() {
                std::io::ErrorKind::NotFound => std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("WHALE_HOME points to {val:?}, but that path does not exist"),
                ),
                _ => std::io::Error::new(
                    err.kind(),
                    format!("failed to read WHALE_HOME {val:?}: {err}"),
                ),
            })?;

            if !metadata.is_dir() {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("WHALE_HOME points to {val:?}, but that path is not a directory"),
                ))
            } else {
                let canonical = path.canonicalize().map_err(|err| {
                    std::io::Error::new(
                        err.kind(),
                        format!("failed to canonicalize WHALE_HOME {val:?}: {err}"),
                    )
                })?;
                validate_whale_home_path(canonical, std::env::var("CODEX_HOME").ok().as_deref())
            }
        }
        None => {
            let mut p = home_dir().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not find home directory",
                )
            })?;
            p.push(".whale");
            AbsolutePathBuf::from_absolute_path(p)
        }
    }
}

fn validate_whale_home_path(
    canonical: PathBuf,
    codex_home_env: Option<&str>,
) -> std::io::Result<AbsolutePathBuf> {
    if canonical.file_name().and_then(|name| name.to_str()) == Some(".codex") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "WHALE_HOME must not point at an official Codex state directory: {}",
                canonical.display()
            ),
        ));
    }

    if let Some(codex_home) = codex_home_env.filter(|value| !value.is_empty()) {
        let codex_path = PathBuf::from(codex_home)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(codex_home));
        if canonical == codex_path {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "WHALE_HOME and CODEX_HOME must not point at the same directory: {}",
                    canonical.display()
                ),
            ));
        }
    }

    AbsolutePathBuf::from_absolute_path(canonical)
}

#[cfg(test)]
mod tests {
    use super::find_codex_home_from_env;
    use codex_utils_absolute_path::AbsolutePathBuf;
    use dirs::home_dir;
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::io::ErrorKind;
    use tempfile::TempDir;

    #[test]
    fn find_codex_home_env_missing_path_is_fatal() {
        let temp_home = TempDir::new().expect("temp home");
        let missing = temp_home.path().join("missing-whale-home");
        let missing_str = missing
            .to_str()
            .expect("missing whale home path should be valid utf-8");

        let err = find_codex_home_from_env(Some(missing_str)).expect_err("missing WHALE_HOME");
        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert!(
            err.to_string().contains("WHALE_HOME"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn find_codex_home_env_file_path_is_fatal() {
        let temp_home = TempDir::new().expect("temp home");
        let file_path = temp_home.path().join("whale-home.txt");
        fs::write(&file_path, "not a directory").expect("write temp file");
        let file_str = file_path
            .to_str()
            .expect("file whale home path should be valid utf-8");

        let err = find_codex_home_from_env(Some(file_str)).expect_err("file WHALE_HOME");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("not a directory"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn find_codex_home_env_valid_directory_canonicalizes() {
        let temp_home = TempDir::new().expect("temp home");
        let temp_str = temp_home
            .path()
            .to_str()
            .expect("temp whale home path should be valid utf-8");

        let resolved = find_codex_home_from_env(Some(temp_str)).expect("valid WHALE_HOME");
        let expected = temp_home
            .path()
            .canonicalize()
            .expect("canonicalize temp home");
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn find_codex_home_env_rejects_dot_codex_directory() {
        let temp_home = TempDir::new().expect("temp home");
        let dot_codex = temp_home.path().join(".codex");
        fs::create_dir_all(&dot_codex).expect("create dot codex");
        let dot_codex_str = dot_codex
            .to_str()
            .expect("dot codex path should be valid utf-8");

        let err = find_codex_home_from_env(Some(dot_codex_str)).expect_err("reject .codex");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("official Codex state"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validate_whale_home_rejects_matching_codex_home() {
        let temp_home = TempDir::new().expect("temp home");
        let canonical = temp_home
            .path()
            .canonicalize()
            .expect("canonical temp home");
        let codex_home = canonical
            .to_str()
            .expect("codex home path should be valid utf-8")
            .to_string();

        let err = super::validate_whale_home_path(canonical, Some(codex_home.as_str()))
            .expect_err("matching homes must be rejected");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("WHALE_HOME and CODEX_HOME"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn find_codex_home_without_env_uses_default_home_dir() {
        let resolved =
            find_codex_home_from_env(/*codex_home_env*/ None).expect("default WHALE_HOME");
        let mut expected = home_dir().expect("home dir");
        expected.push(".whale");
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }
}
