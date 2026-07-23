use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Serialize};

use super::error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult};

pub fn atomic_write_json(path: &Path, value: &impl Serialize) -> VoiceModelResult<()> {
    let parent = path.parent().ok_or_else(|| {
        VoiceModelError::new(
            VoiceModelErrorCode::PathValidationFailure,
            "Managed metadata requires a parent directory.",
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| VoiceModelError::storage("Cannot create model storage", error))?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| {
        VoiceModelError::new(
            VoiceModelErrorCode::AtomicWriteFailure,
            "Model metadata could not be serialized.",
        )
    })?;
    let temporary = sibling(path, ".tmp");
    let backup = sibling(path, ".bak");
    if temporary.exists() {
        fs::remove_file(&temporary)
            .map_err(|error| VoiceModelError::storage("Cannot clear stale metadata", error))?;
    }
    let mut file = File::create(&temporary)
        .map_err(|error| VoiceModelError::storage("Cannot create temporary metadata", error))?;
    file.write_all(&bytes)
        .and_then(|_| file.write_all(b"\n"))
        .and_then(|_| file.sync_all())
        .map_err(|error| {
            VoiceModelError::new(
                VoiceModelErrorCode::AtomicWriteFailure,
                format!("Cannot flush model metadata: {error}"),
            )
        })?;
    drop(file);
    if path.exists() {
        if backup.exists() {
            fs::remove_file(&backup)
                .map_err(|error| VoiceModelError::storage("Cannot clear old metadata", error))?;
        }
        fs::rename(path, &backup)
            .map_err(|error| VoiceModelError::storage("Cannot stage metadata backup", error))?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::AtomicWriteFailure,
            format!("Cannot commit model metadata: {error}"),
        ));
    }
    Ok(())
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> VoiceModelResult<T> {
    recover(path)?;
    let contents = fs::read_to_string(path)
        .map_err(|error| VoiceModelError::storage("Cannot read model metadata", error))?;
    serde_json::from_str(&contents).map_err(|_| {
        VoiceModelError::new(
            VoiceModelErrorCode::StorageUnavailable,
            "Managed model metadata is invalid.",
        )
    })
}

pub fn ensure_relative_path(path: &str) -> VoiceModelResult<()> {
    let candidate = Path::new(path);
    let valid = !path.is_empty()
        && !candidate.is_absolute()
        && !path.contains('\\')
        && candidate
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)));
    if valid {
        Ok(())
    } else {
        Err(VoiceModelError::new(
            VoiceModelErrorCode::PathValidationFailure,
            "Managed model paths must be normalized and relative.",
        ))
    }
}

pub fn managed_join(root: &Path, relative: &str) -> VoiceModelResult<PathBuf> {
    ensure_relative_path(relative)?;
    let path = root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    if path.starts_with(root) {
        Ok(path)
    } else {
        Err(VoiceModelError::new(
            VoiceModelErrorCode::PathValidationFailure,
            "The managed path escapes model storage.",
        ))
    }
}

pub fn directory_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            entry.metadata().map_or(0, |metadata| {
                if metadata.is_dir() {
                    directory_size(&entry.path())
                } else {
                    metadata.len()
                }
            })
        })
        .sum()
}

pub fn remove_managed_directory(root: &Path, target: &Path) -> VoiceModelResult<()> {
    if target == root || !target.starts_with(root) {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::PathValidationFailure,
            "Refusing to delete outside the managed model root.",
        ));
    }
    if !target.exists() {
        return Ok(());
    }
    let name = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            VoiceModelError::new(
                VoiceModelErrorCode::PathValidationFailure,
                "Managed deletion target has no safe opaque name.",
            )
        })?;
    let tombstone = root.join(format!(".delete-{name}.tombstone"));
    fs::write(&tombstone, name)
        .map_err(|error| VoiceModelError::storage("Cannot create deletion tombstone", error))?;
    fs::remove_dir_all(target)
        .map_err(|error| VoiceModelError::storage("Cannot delete managed model data", error))?;
    fs::remove_file(tombstone)
        .map_err(|error| VoiceModelError::storage("Cannot clear deletion tombstone", error))
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    path.with_file_name(format!(
        "{}{suffix}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("model.json")
    ))
}

fn recover(path: &Path) -> VoiceModelResult<()> {
    if path.exists() {
        return Ok(());
    }
    for candidate in [sibling(path, ".bak"), sibling(path, ".tmp")] {
        if candidate.exists() {
            fs::rename(candidate, path).map_err(|error| {
                VoiceModelError::new(
                    VoiceModelErrorCode::AtomicWriteFailure,
                    format!("Cannot recover interrupted model metadata: {error}"),
                )
            })?;
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ensure_relative_path, managed_join};
    use std::path::Path;

    #[test]
    fn rejects_path_traversal_and_absolute_paths() {
        assert!(ensure_relative_path("model/checkpoint.pth").is_ok());
        assert!(ensure_relative_path("../outside.pth").is_err());
        assert!(ensure_relative_path("C:/outside.pth").is_err());
        assert!(ensure_relative_path("model\\outside.pth").is_err());
        assert!(managed_join(Path::new("root"), "../outside").is_err());
    }
}
