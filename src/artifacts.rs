use nauron_contracts::ArtifactRef;

/// Returns the preferred markdown/text artifact if present (content-type text/* or .md/.txt).
pub fn select_document_artifact(artifacts: &[ArtifactRef]) -> Option<&ArtifactRef> {
    artifacts.iter().find(|artifact| is_document(artifact))
}

/// Returns the preferred archive artifact (tar/zip) if present.
pub fn select_archive_artifact(artifacts: &[ArtifactRef]) -> Option<&ArtifactRef> {
    artifacts.iter().find(|artifact| is_archive(artifact))
}

fn is_document(artifact: &ArtifactRef) -> bool {
    let ct_matches = artifact
        .content_type
        .as_deref()
        .map(|ct| ct.starts_with("text/"))
        .unwrap_or(false);
    let key_matches = artifact
        .key
        .as_str()
        .rsplit('/')
        .next()
        .map(|name| name.ends_with(".md") || name.ends_with(".txt"))
        .unwrap_or(false);
    ct_matches || key_matches
}

fn is_archive(artifact: &ArtifactRef) -> bool {
    let ct_matches = artifact
        .content_type
        .as_deref()
        .map(|ct| {
            matches!(
                ct,
                "application/gzip" | "application/x-tar" | "application/zip"
            )
        })
        .unwrap_or(false);
    let key_matches = artifact
        .key
        .as_str()
        .rsplit('/')
        .next()
        .map(|name| name.ends_with(".tar.gz") || name.ends_with(".tgz") || name.ends_with(".zip"))
        .unwrap_or(false);
    ct_matches || key_matches
}
