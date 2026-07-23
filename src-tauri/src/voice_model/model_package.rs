use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{Read, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::voice_dataset::{
    hash::sha256_bytes,
    storage::{new_id, timestamp},
};

use super::{
    artifact::{
        ArtifactHealth, LicensingStatus, ModelApprovalStatus, PortabilityStatus,
        VoiceModelArtifactV1,
    },
    error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult},
    storage::{atomic_write_json, ensure_relative_path, managed_join},
};

pub const MODEL_PACKAGE_SCHEMA_VERSION: u32 = 1;
pub const MAX_PACKAGE_FILES: usize = 256;
pub const MAX_PACKAGE_UNCOMPRESSED_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageFileInventory {
    pub relative_path: String,
    pub size_bytes: u64,
    pub content_hash: String,
    pub hash_algorithm: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelPackageManifestV1 {
    pub schema_version: u32,
    pub package_id: String,
    pub artifact_id: String,
    pub required_backend_id: String,
    pub required_compatibility_profile_id: String,
    pub required_external_checkpoint_hashes: Vec<String>,
    pub consent_provenance_reference: String,
    pub synthetic_use_notice: String,
    pub exported_at: String,
    pub application_version: String,
    pub portability_status: PortabilityStatus,
    pub licensing_acknowledged: bool,
    pub file_inventory: Vec<PackageFileInventory>,
    pub aggregate_content_hash: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelPackageExportResult {
    pub package_id: String,
    pub output_file: String,
    pub file_count: u32,
    pub total_bytes: u64,
    pub portability_status: PortabilityStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelPackageImportRequest {
    pub package_path: String,
    pub profile_id: String,
    pub active_consent_version: String,
    pub association_confirmed: bool,
}

pub fn export_package(
    artifact_directory: &Path,
    artifact: &VoiceModelArtifactV1,
    destination: &Path,
    licensing_acknowledged: bool,
) -> VoiceModelResult<ModelPackageExportResult> {
    if artifact
        .model_files
        .iter()
        .any(|file| file.licensing_status == LicensingStatus::Unknown)
        && !licensing_acknowledged
    {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::LicensingAcknowledgementRequired,
            "Redistribution permission has not been verified for one or more files.",
        ));
    }
    let exported_at = timestamp().map_err(clock_error)?;
    let package_id = new_id("package", &exported_at);
    let mut entries = BTreeMap::<String, Vec<u8>>::new();
    let artifact_json = serde_json::to_vec_pretty(artifact).map_err(|_| {
        VoiceModelError::new(
            VoiceModelErrorCode::PackageInvalid,
            "Cannot serialize the artifact package manifest.",
        )
    })?;
    entries.insert("artifact/artifact.json".to_owned(), artifact_json);
    for file in &artifact.model_files {
        ensure_relative_path(&file.relative_path)?;
        let source = managed_join(artifact_directory, &file.relative_path)?;
        let metadata = fs::symlink_metadata(&source)
            .map_err(|error| VoiceModelError::storage("Cannot inspect exported artifact", error))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::PackageInvalid,
                "Model export rejects links and unsupported files.",
            ));
        }
        let bytes = fs::read(&source)
            .map_err(|error| VoiceModelError::storage("Cannot read exported model file", error))?;
        if sha256_bytes(&bytes) != file.content_hash {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::ArtifactHashMismatch,
                "A model file changed before export.",
            ));
        }
        entries.insert(format!("artifact/{}", file.relative_path), bytes);
    }
    if let Some(evaluation) = &artifact.evaluation {
        entries.insert(
            "evaluation/evaluation.json".to_owned(),
            serde_json::to_vec_pretty(evaluation).map_err(|_| {
                VoiceModelError::new(
                    VoiceModelErrorCode::PackageInvalid,
                    "Cannot serialize evaluation metadata.",
                )
            })?,
        );
    }
    if let Some(environment) = &artifact.environment_fingerprint {
        entries.insert(
            "provenance/environment.json".to_owned(),
            serde_json::to_vec_pretty(environment).map_err(|_| {
                VoiceModelError::new(
                    VoiceModelErrorCode::PackageInvalid,
                    "Cannot serialize environment provenance.",
                )
            })?,
        );
    }
    entries.insert(
        "provenance/checkpoints.json".to_owned(),
        serde_json::to_vec_pretty(&artifact.checkpoint_identities).map_err(|_| {
            VoiceModelError::new(
                VoiceModelErrorCode::PackageInvalid,
                "Cannot serialize checkpoint provenance.",
            )
        })?,
    );
    entries.insert(
        "README.txt".to_owned(),
        b"Mam Voice Changer offline synthetic model package. Imported content is untrusted and remains unapproved. Dataset and consent audio are intentionally excluded. External checkpoints and the compatible local worker/runtime may still be required.\n".to_vec(),
    );
    entries.insert(
        "LICENSES/mam-voice-changer-notice.txt".to_owned(),
        b"Mam Voice Changer package metadata is project-generated. Check repository license notices separately.\n".to_vec(),
    );
    entries.insert(
        "LICENSES/backend-notice.txt".to_owned(),
        b"Third-party backend and model-weight licensing are separate. Redistribution permission has not been verified for excluded external checkpoints.\n".to_vec(),
    );
    reject_private_content_names(entries.keys())?;
    let inventory = inventory(&entries);
    let aggregate_content_hash = aggregate_hash(&inventory);
    let manifest = ModelPackageManifestV1 {
        schema_version: MODEL_PACKAGE_SCHEMA_VERSION,
        package_id: package_id.clone(),
        artifact_id: artifact.artifact_id.clone(),
        required_backend_id: artifact.backend_id.clone(),
        required_compatibility_profile_id: artifact.compatibility_profile_id.clone(),
        required_external_checkpoint_hashes: artifact
            .checkpoint_identities
            .iter()
            .filter_map(|checkpoint| checkpoint.content_hash.clone())
            .collect(),
        consent_provenance_reference: sha256_bytes(
            format!("{}:{}", artifact.profile_id, artifact.consent_version).as_bytes(),
        ),
        synthetic_use_notice: artifact.synthetic_use_notice_version.clone(),
        exported_at,
        application_version: env!("CARGO_PKG_VERSION").to_owned(),
        portability_status: artifact.portability_status,
        licensing_acknowledged,
        file_inventory: inventory,
        aggregate_content_hash,
    };
    entries.insert(
        "model-package.json".to_owned(),
        serde_json::to_vec_pretty(&manifest).map_err(|_| {
            VoiceModelError::new(
                VoiceModelErrorCode::PackageInvalid,
                "Cannot serialize package metadata.",
            )
        })?,
    );
    write_stored_zip(destination, &entries)?;
    let total_bytes = entries.values().map(|value| value.len() as u64).sum();
    Ok(ModelPackageExportResult {
        package_id,
        output_file: destination
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("model-package.zip")
            .to_owned(),
        file_count: entries.len().min(u32::MAX as usize) as u32,
        total_bytes,
        portability_status: artifact.portability_status,
    })
}

pub fn import_package(
    models_root: &Path,
    request: &ModelPackageImportRequest,
) -> VoiceModelResult<VoiceModelArtifactV1> {
    if !request.association_confirmed {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::ConsentInactive,
            "Explicit confirmation is required before associating an imported model with active consent.",
        ));
    }
    validate_managed_id(&request.profile_id, "profile")?;
    let entries = read_stored_zip(Path::new(&request.package_path))?;
    reject_private_content_names(entries.keys())?;
    let manifest_bytes = entries.get("model-package.json").ok_or_else(|| {
        VoiceModelError::new(
            VoiceModelErrorCode::PackageInvalid,
            "The package manifest is missing.",
        )
    })?;
    if manifest_bytes.len() > MAX_MANIFEST_BYTES {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::PackageLimitExceeded,
            "The package manifest exceeds its size limit.",
        ));
    }
    let manifest: ModelPackageManifestV1 =
        serde_json::from_slice(manifest_bytes).map_err(|_| {
            VoiceModelError::new(
                VoiceModelErrorCode::PackageInvalid,
                "The package manifest is invalid.",
            )
        })?;
    if manifest.schema_version != MODEL_PACKAGE_SCHEMA_VERSION {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::PackageSchemaUnsupported,
            "The model-package schema is unsupported.",
        ));
    }
    validate_inventory(&entries, &manifest)?;
    let packaged_artifact: VoiceModelArtifactV1 =
        serde_json::from_slice(entries.get("artifact/artifact.json").ok_or_else(|| {
            VoiceModelError::new(
                VoiceModelErrorCode::PackageInvalid,
                "The artifact manifest is missing.",
            )
        })?)
        .map_err(|_| {
            VoiceModelError::new(
                VoiceModelErrorCode::PackageInvalid,
                "The artifact manifest is invalid.",
            )
        })?;
    if packaged_artifact.backend_id != manifest.required_backend_id
        || packaged_artifact.compatibility_profile_id != manifest.required_compatibility_profile_id
    {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::PackageInvalid,
            "Package backend requirements do not match the artifact provenance.",
        ));
    }
    let now = timestamp().map_err(clock_error)?;
    let artifact_id = new_id("artifact", &now);
    let profile_root = models_root
        .join("profiles")
        .join(&request.profile_id)
        .join("artifacts");
    let final_directory = profile_root.join(&artifact_id);
    let temporary = profile_root.join(format!(".{artifact_id}.importing"));
    fs::create_dir_all(&temporary).map_err(|error| {
        VoiceModelError::storage("Cannot create temporary import storage", error)
    })?;
    let result = (|| {
        for file in &packaged_artifact.model_files {
            let package_path = format!("artifact/{}", file.relative_path);
            let bytes = entries.get(&package_path).ok_or_else(|| {
                VoiceModelError::new(
                    VoiceModelErrorCode::PackageInvalid,
                    "A packaged model file is missing.",
                )
            })?;
            if sha256_bytes(bytes) != file.content_hash {
                return Err(VoiceModelError::new(
                    VoiceModelErrorCode::PackageHashMismatch,
                    "A packaged model file failed SHA-256 validation.",
                ));
            }
            let output = managed_join(&temporary, &file.relative_path)?;
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    VoiceModelError::storage("Cannot create imported model storage", error)
                })?;
            }
            fs::write(output, bytes).map_err(|error| {
                VoiceModelError::storage("Cannot write imported model file", error)
            })?;
        }
        let mut imported = packaged_artifact;
        imported.artifact_id = artifact_id.clone();
        imported.profile_id = request.profile_id.clone();
        imported.consent_version = request.active_consent_version.clone();
        imported.consent_confirmed_at = now.clone();
        imported.approval_status = ModelApprovalStatus::Unevaluated;
        imported.evaluation = None;
        imported.health = ArtifactHealth::Unqualified;
        imported.portability_status = if manifest.required_external_checkpoint_hashes.is_empty() {
            PortabilityStatus::Portable
        } else {
            PortabilityStatus::PortableWithExternalDependencies
        };
        imported.imported_package_id = Some(manifest.package_id.clone());
        imported.notes = Some(
            "Imported package is untrusted, unevaluated, unapproved, and requires local dependency revalidation. Original package consent provenance was retained in the package index."
                .to_owned(),
        );
        imported.updated_at = now.clone();
        imported.created_at = now.clone();
        atomic_write_json(&temporary.join("artifact.json"), &imported)?;
        fs::create_dir_all(&profile_root).map_err(|error| {
            VoiceModelError::storage("Cannot create profile artifact storage", error)
        })?;
        fs::rename(&temporary, &final_directory).map_err(|error| {
            VoiceModelError::new(
                VoiceModelErrorCode::AtomicWriteFailure,
                format!("Cannot atomically install the imported model package: {error}"),
            )
        })?;
        let imports_root = models_root.join("imports");
        fs::create_dir_all(&imports_root).map_err(|error| {
            VoiceModelError::storage("Cannot create import index storage", error)
        })?;
        atomic_write_json(
            &imports_root.join(format!("{}.json", manifest.package_id)),
            &manifest,
        )?;
        Ok(imported)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&temporary);
    }
    result
}

fn inventory(entries: &BTreeMap<String, Vec<u8>>) -> Vec<PackageFileInventory> {
    entries
        .iter()
        .map(|(path, bytes)| PackageFileInventory {
            relative_path: path.clone(),
            size_bytes: bytes.len() as u64,
            content_hash: sha256_bytes(bytes),
            hash_algorithm: "sha256".to_owned(),
        })
        .collect()
}

fn aggregate_hash(inventory: &[PackageFileInventory]) -> String {
    let mut stable = String::new();
    for file in inventory {
        stable.push_str(&file.relative_path);
        stable.push(':');
        stable.push_str(&file.content_hash);
        stable.push('\n');
    }
    sha256_bytes(stable.as_bytes())
}

fn validate_inventory(
    entries: &BTreeMap<String, Vec<u8>>,
    manifest: &ModelPackageManifestV1,
) -> VoiceModelResult<()> {
    let expected: HashSet<_> = manifest
        .file_inventory
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect();
    let actual: HashSet<_> = entries
        .keys()
        .filter(|path| path.as_str() != "model-package.json")
        .map(String::as_str)
        .collect();
    if expected != actual {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::PackageInvalid,
            "The package contains missing or unexpected files.",
        ));
    }
    for file in &manifest.file_inventory {
        ensure_relative_path(&file.relative_path)?;
        let bytes = entries.get(&file.relative_path).ok_or_else(|| {
            VoiceModelError::new(
                VoiceModelErrorCode::PackageInvalid,
                "A package inventory file is missing.",
            )
        })?;
        if file.hash_algorithm != "sha256"
            || file.size_bytes != bytes.len() as u64
            || file.content_hash != sha256_bytes(bytes)
        {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::PackageHashMismatch,
                "The package file inventory failed validation.",
            ));
        }
    }
    if aggregate_hash(&manifest.file_inventory) != manifest.aggregate_content_hash {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::PackageHashMismatch,
            "The aggregate package-content hash is invalid.",
        ));
    }
    Ok(())
}

fn reject_private_content_names<'a>(
    paths: impl Iterator<Item = &'a String>,
) -> VoiceModelResult<()> {
    for path in paths {
        ensure_relative_path(path)?;
        let lower = path.to_ascii_lowercase();
        if lower.contains("dataset")
            || lower.contains("consent.wav")
            || lower.contains("snapshot/audio")
            || lower.contains("temporary-inference")
            || lower.contains(".venv")
        {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::PackageInvalid,
                "The model package contains a forbidden private-content path.",
            ));
        }
    }
    Ok(())
}

fn write_stored_zip(path: &Path, entries: &BTreeMap<String, Vec<u8>>) -> VoiceModelResult<()> {
    if entries.len() > MAX_PACKAGE_FILES {
        return Err(limit_error());
    }
    let total: u64 = entries.values().map(|bytes| bytes.len() as u64).sum();
    if total > MAX_PACKAGE_UNCOMPRESSED_BYTES {
        return Err(limit_error());
    }
    let parent = path.parent().ok_or_else(|| {
        VoiceModelError::new(
            VoiceModelErrorCode::PathValidationFailure,
            "Export requires a destination directory.",
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        VoiceModelError::storage("Cannot create model-package destination", error)
    })?;
    let temporary = path.with_extension("partial");
    let mut file = fs::File::create(&temporary)
        .map_err(|error| VoiceModelError::storage("Cannot create model package", error))?;
    let mut central = Vec::new();
    let mut offset = 0_u32;
    for (name, bytes) in entries {
        ensure_relative_path(name)?;
        let name_bytes = name.as_bytes();
        let name_len = u16::try_from(name_bytes.len()).map_err(|_| limit_error())?;
        let size = u32::try_from(bytes.len()).map_err(|_| limit_error())?;
        let crc = crc32(bytes);
        let mut local = Vec::new();
        push_u32(&mut local, 0x0403_4b50);
        push_u16(&mut local, 20);
        push_u16(&mut local, 0);
        push_u16(&mut local, 0);
        push_u16(&mut local, 0);
        push_u16(&mut local, 0);
        push_u32(&mut local, crc);
        push_u32(&mut local, size);
        push_u32(&mut local, size);
        push_u16(&mut local, name_len);
        push_u16(&mut local, 0);
        local.extend_from_slice(name_bytes);
        file.write_all(&local)
            .and_then(|_| file.write_all(bytes))
            .map_err(|error| VoiceModelError::storage("Cannot write model package", error))?;
        central.push((name_bytes.to_vec(), crc, size, offset));
        offset = offset
            .checked_add(u32::try_from(local.len()).map_err(|_| limit_error())?)
            .and_then(|value| value.checked_add(size))
            .ok_or_else(limit_error)?;
    }
    let central_offset = offset;
    let mut central_bytes = Vec::new();
    for (name, crc, size, local_offset) in &central {
        push_u32(&mut central_bytes, 0x0201_4b50);
        push_u16(&mut central_bytes, 20);
        push_u16(&mut central_bytes, 20);
        push_u16(&mut central_bytes, 0);
        push_u16(&mut central_bytes, 0);
        push_u16(&mut central_bytes, 0);
        push_u16(&mut central_bytes, 0);
        push_u32(&mut central_bytes, *crc);
        push_u32(&mut central_bytes, *size);
        push_u32(&mut central_bytes, *size);
        push_u16(&mut central_bytes, name.len() as u16);
        push_u16(&mut central_bytes, 0);
        push_u16(&mut central_bytes, 0);
        push_u16(&mut central_bytes, 0);
        push_u16(&mut central_bytes, 0);
        push_u32(&mut central_bytes, 0);
        push_u32(&mut central_bytes, *local_offset);
        central_bytes.extend_from_slice(name);
    }
    file.write_all(&central_bytes)
        .map_err(|error| VoiceModelError::storage("Cannot write model-package directory", error))?;
    let count = u16::try_from(central.len()).map_err(|_| limit_error())?;
    let mut end = Vec::new();
    push_u32(&mut end, 0x0605_4b50);
    push_u16(&mut end, 0);
    push_u16(&mut end, 0);
    push_u16(&mut end, count);
    push_u16(&mut end, count);
    push_u32(
        &mut end,
        u32::try_from(central_bytes.len()).map_err(|_| limit_error())?,
    );
    push_u32(&mut end, central_offset);
    push_u16(&mut end, 0);
    file.write_all(&end)
        .and_then(|_| file.sync_all())
        .map_err(|error| VoiceModelError::storage("Cannot finalize model package", error))?;
    drop(file);
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        VoiceModelError::new(
            VoiceModelErrorCode::AtomicWriteFailure,
            format!("Cannot commit model package: {error}"),
        )
    })
}

fn read_stored_zip(path: &Path) -> VoiceModelResult<BTreeMap<String, Vec<u8>>> {
    let metadata = fs::metadata(path)
        .map_err(|error| VoiceModelError::storage("Cannot inspect model package", error))?;
    if metadata.len() > MAX_PACKAGE_UNCOMPRESSED_BYTES + 64 * 1024 * 1024 {
        return Err(limit_error());
    }
    let mut file = fs::File::open(path)
        .map_err(|error| VoiceModelError::storage("Cannot open model package", error))?;
    let mut entries = BTreeMap::new();
    let mut total = 0_u64;
    loop {
        let signature = read_u32(&mut file)?;
        if signature == 0x0201_4b50 || signature == 0x0605_4b50 {
            break;
        }
        if signature != 0x0403_4b50 {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::PackageInvalid,
                "The package ZIP structure is unsupported.",
            ));
        }
        let mut header = [0_u8; 26];
        file.read_exact(&mut header)
            .map_err(|error| VoiceModelError::storage("Cannot read model-package header", error))?;
        let flags = u16::from_le_bytes([header[2], header[3]]);
        let method = u16::from_le_bytes([header[4], header[5]]);
        let crc = u32::from_le_bytes(header[10..14].try_into().expect("four bytes"));
        let compressed = u32::from_le_bytes(header[14..18].try_into().expect("four bytes"));
        let uncompressed = u32::from_le_bytes(header[18..22].try_into().expect("four bytes"));
        let name_len = u16::from_le_bytes([header[22], header[23]]) as usize;
        let extra_len = u16::from_le_bytes([header[24], header[25]]) as usize;
        if flags != 0
            || method != 0
            || compressed != uncompressed
            || name_len == 0
            || name_len > 1024
        {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::PackageInvalid,
                "Only bounded, unencrypted, stored model-package ZIP entries are supported.",
            ));
        }
        let mut name = vec![0_u8; name_len];
        file.read_exact(&mut name).map_err(|error| {
            VoiceModelError::storage("Cannot read model-package filename", error)
        })?;
        let name = String::from_utf8(name).map_err(|_| {
            VoiceModelError::new(
                VoiceModelErrorCode::PackageInvalid,
                "Package paths must be UTF-8.",
            )
        })?;
        ensure_relative_path(&name)?;
        if extra_len > 4096 {
            return Err(limit_error());
        }
        let mut extra = vec![0_u8; extra_len];
        file.read_exact(&mut extra).map_err(|error| {
            VoiceModelError::storage("Cannot read model-package extra data", error)
        })?;
        total = total
            .checked_add(u64::from(uncompressed))
            .ok_or_else(limit_error)?;
        if entries.len() >= MAX_PACKAGE_FILES || total > MAX_PACKAGE_UNCOMPRESSED_BYTES {
            return Err(limit_error());
        }
        let mut bytes = vec![0_u8; uncompressed as usize];
        file.read_exact(&mut bytes).map_err(|error| {
            VoiceModelError::storage("Cannot read model-package content", error)
        })?;
        if crc32(&bytes) != crc || entries.insert(name, bytes).is_some() {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::PackageHashMismatch,
                "The package contains a duplicate or CRC-mismatched entry.",
            ));
        }
    }
    if entries.is_empty() {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::PackageInvalid,
            "The model package is empty.",
        ));
    }
    Ok(entries)
}

fn read_u32(reader: &mut impl Read) -> VoiceModelResult<u32> {
    let mut bytes = [0_u8; 4];
    reader
        .read_exact(&mut bytes)
        .map_err(|error| VoiceModelError::storage("Cannot read model-package signature", error))?;
    Ok(u32::from_le_bytes(bytes))
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (0_u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

fn limit_error() -> VoiceModelError {
    VoiceModelError::new(
        VoiceModelErrorCode::PackageLimitExceeded,
        "The model package exceeds its file-count or size limit.",
    )
}

fn validate_managed_id(id: &str, kind: &str) -> VoiceModelResult<()> {
    let valid = id.starts_with(&format!("{kind}-"))
        && id.len() <= 80
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(VoiceModelError::new(
            VoiceModelErrorCode::PathValidationFailure,
            format!("Invalid managed {kind} identifier."),
        ))
    }
}

fn clock_error(error: crate::voice_dataset::error::DatasetError) -> VoiceModelError {
    VoiceModelError::new(VoiceModelErrorCode::StorageUnavailable, error.message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice_model::{
        artifact::{ArtifactFileRole, LicenseNoticeReference, ModelArtifactFile, TrainingSummary},
        qualification::QualificationLevel,
        state::{TrainingConfiguration, TrainingPreset},
    };

    #[test]
    fn stored_zip_round_trip_rejects_traversal_and_hash_mismatch() {
        let root = std::env::temp_dir().join(format!("mam-package-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp");
        let package = root.join("test.zip");
        let entries = BTreeMap::from([
            ("model-package.json".to_owned(), b"{}".to_vec()),
            ("artifact/model/file.pth".to_owned(), b"model".to_vec()),
        ]);
        write_stored_zip(&package, &entries).expect("write");
        assert_eq!(read_stored_zip(&package).expect("read"), entries);
        assert!(ensure_relative_path("../outside").is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn package_limits_and_crc_are_deterministic() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        const {
            assert!(MAX_PACKAGE_FILES < 1_000);
            assert!(MAX_PACKAGE_UNCOMPRESSED_BYTES <= 8 * 1024 * 1024 * 1024);
        }
    }

    #[test]
    fn export_import_round_trip_excludes_dataset_and_remains_unapproved() {
        let root =
            std::env::temp_dir().join(format!("mam-package-roundtrip-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let artifact_directory = root.join("source-artifact");
        fs::create_dir_all(artifact_directory.join("model")).expect("model root");
        let model_bytes = b"mock-model";
        fs::write(artifact_directory.join("model/file.pth"), model_bytes).expect("model");
        let artifact = VoiceModelArtifactV1 {
            schema_version: 1,
            artifact_id: "artifact-source".to_owned(),
            profile_id: "profile-source".to_owned(),
            display_name: "Portable test".to_owned(),
            backend_id: "mock-qualification".to_owned(),
            backend_version: "mock-v1".to_owned(),
            worker_protocol_version: 1,
            compatibility_profile_id: "mock-profile-v1".to_owned(),
            environment_fingerprint: None,
            checkpoint_identities: Vec::new(),
            backend_revision: Some("a".repeat(40)),
            adapter_version: "mock-adapter-v1".to_owned(),
            snapshot_id: "snapshot-source".to_owned(),
            snapshot_hash: "snapshot-hash".to_owned(),
            consent_version: "consent-v1".to_owned(),
            consent_confirmed_at: "1".to_owned(),
            training_configuration: TrainingConfiguration::for_preset(
                TrainingPreset::QuickExperiment,
            ),
            training_summary: TrainingSummary::default(),
            model_files: vec![ModelArtifactFile {
                relative_path: "model/file.pth".to_owned(),
                content_hash: sha256_bytes(model_bytes),
                size_bytes: model_bytes.len() as u64,
                role: ArtifactFileRole::ModelWeights,
                licensing_status: LicensingStatus::Unknown,
            }],
            model_content_hash: sha256_bytes(
                format!("model/file.pth{}", sha256_bytes(model_bytes)).as_bytes(),
            ),
            expected_inference_sample_rate: 48_000,
            supported_inference_controls: vec!["diffusionSteps".to_owned()],
            portability_status: PortabilityStatus::Portable,
            qualification_level: QualificationLevel::BackendLoaded,
            license_notices: vec![LicenseNoticeReference {
                role: "artifact".to_owned(),
                label: "Mock artifact".to_owned(),
                status: LicensingStatus::Unknown,
                notice: "Redistribution permission has not been verified for this file.".to_owned(),
            }],
            synthetic_use_notice_version: "mam-synthetic-use-v1".to_owned(),
            health: ArtifactHealth::Unqualified,
            imported_package_id: None,
            evaluation: None,
            approval_status: ModelApprovalStatus::Unevaluated,
            notes: None,
            created_at: "1".to_owned(),
            updated_at: "1".to_owned(),
        };
        let package = root.join("portable.zip");
        let exported =
            export_package(&artifact_directory, &artifact, &package, true).expect("export");
        assert_eq!(exported.portability_status, PortabilityStatus::Portable);
        let entries = read_stored_zip(&package).expect("entries");
        assert!(entries
            .keys()
            .all(|path| !path.to_ascii_lowercase().contains("dataset")));
        assert!(!entries.keys().any(|path| path.contains("consent.wav")));
        let imported = import_package(
            &root.join("managed"),
            &ModelPackageImportRequest {
                package_path: package.to_string_lossy().to_string(),
                profile_id: "profile-target".to_owned(),
                active_consent_version: "consent-target".to_owned(),
                association_confirmed: true,
            },
        )
        .expect("import");
        assert_eq!(imported.profile_id, "profile-target");
        assert_eq!(imported.approval_status, ModelApprovalStatus::Unevaluated);
        assert_eq!(imported.health, ArtifactHealth::Unqualified);
        assert_eq!(
            imported.imported_package_id.as_deref(),
            Some(exported.package_id.as_str())
        );
        let _ = fs::remove_dir_all(root);
    }
}
