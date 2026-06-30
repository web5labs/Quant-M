use crate::child_bootstrap::sha256_file;
use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

const MAX_REQUEST_LINE: usize = 4096;

#[derive(Debug, Clone, Deserialize)]
pub struct PackMetadata {
    pub pack_id: String,
    pub version: String,
    pub desk: String,
    pub archive_name: String,
    pub archive_size: u64,
    pub sha256: String,
    pub created_at: String,
    pub max_authority: String,
    #[serde(default)]
    pub allowed_roles: Vec<String>,
    #[serde(default)]
    pub schemas: Vec<String>,
    pub timing_policy: Option<String>,
    pub skills_manifest: Option<String>,
    #[serde(default)]
    pub revoked: bool,
    #[serde(default)]
    pub script_execution: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackListingItem {
    pub pack_id: String,
    pub version: String,
    pub desk: String,
    pub archive_name: String,
    pub archive_size: u64,
    pub sha256: String,
    pub created_at: String,
    pub max_authority: String,
    pub allowed_roles: Vec<String>,
    pub schemas: Vec<String>,
    pub timing_policy: Option<String>,
    pub skills_manifest: Option<String>,
    pub notes: Option<String>,
    pub download_url: String,
    pub suggested_sync_command: String,
    pub heartbeat_active_pack: ActivePackReport,
    pub evidence_active_pack: EvidencePackReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackListing {
    pub pack_url: String,
    pub role: Option<String>,
    pub packs: Vec<PackListingItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivePackReport {
    pub active_pack_id: String,
    pub active_pack_version: String,
    pub active_pack_hash: String,
    pub max_authority: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvidencePackReport {
    pub pack_id: String,
    pub pack_version: String,
    pub pack_hash: String,
    pub non_authoritative: bool,
}

#[derive(Debug, Clone)]
struct ValidPack {
    metadata: PackMetadata,
    archive_path: PathBuf,
}

pub fn list_packs(pack_dir: &Path, pack_url: &str, role: Option<&str>) -> Result<PackListing> {
    let pack_url = trim_url(pack_url);
    let role = role.map(str::trim).filter(|value| !value.is_empty());
    let packs = valid_packs(pack_dir)?
        .into_iter()
        .filter(|pack| role_allowed(&pack.metadata, role))
        .map(|pack| render_pack(&pack.metadata, &pack_url))
        .collect();
    Ok(PackListing {
        pack_url,
        role: role.map(ToString::to_string),
        packs,
    })
}

pub fn render_listing_text(listing: &PackListing) -> String {
    let mut out = String::from("Quant-M child pack sync\n");
    out.push_str(&format!("pack_url: {}\n", listing.pack_url));
    out.push_str(&format!(
        "role: {}\n",
        listing.role.as_deref().unwrap_or("none")
    ));
    if listing.packs.is_empty() {
        out.push_str("packs: none valid for role\n");
        return out;
    }
    for pack in &listing.packs {
        out.push_str(&format!(
            "\n{} version={} desk={} max_authority={} size={} sha256={}\n",
            pack.pack_id,
            pack.version,
            pack.desk,
            pack.max_authority,
            pack.archive_size,
            pack.sha256
        ));
        out.push_str(&format!("download: {}\n", pack.download_url));
        out.push_str("sync:\n");
        out.push_str(&pack.suggested_sync_command);
        out.push('\n');
        out.push_str(&format!(
            "heartbeat active_pack_hash={}\n",
            pack.heartbeat_active_pack.active_pack_hash
        ));
        out.push_str(&format!(
            "evidence pack_hash={} non_authoritative=true\n",
            pack.evidence_active_pack.pack_hash
        ));
    }
    out
}

pub fn serve(pack_dir: PathBuf, bind: &str) -> Result<()> {
    let listener = TcpListener::bind(bind).with_context(|| format!("failed to bind {bind}"))?;
    let local_addr = listener.local_addr()?;
    let pack_url = format!("http://{local_addr}");
    println!("Quant-M child pack sync serving");
    println!("pack_url: {pack_url}");
    println!("pack_dir: {}", pack_dir.display());
    println!(
        "safety: only non-revoked metadata-listed packs with matching SHA-256 are downloadable"
    );
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(err) = handle_connection(&mut stream, &pack_dir, &pack_url) {
                    let _ = write_response(
                        &mut stream,
                        500,
                        "text/plain; charset=utf-8",
                        format!("pack sync error: {err}\n").as_bytes(),
                    );
                }
            }
            Err(err) => eprintln!("pack sync connection failed: {err}"),
        }
    }
    Ok(())
}

fn handle_connection(stream: &mut TcpStream, pack_dir: &Path, pack_url: &str) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    if request_line.len() > MAX_REQUEST_LINE {
        bail!("request line too large");
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or("/");
    if method != "GET" {
        return write_response(stream, 405, "text/plain; charset=utf-8", b"GET only\n");
    }
    let (path, role) = split_target_role(target)?;
    match path.as_str() {
        "/" | "/index.html" => {
            let listing = list_packs(pack_dir, pack_url, role.as_deref())?;
            write_response(
                stream,
                200,
                "text/html; charset=utf-8",
                render_html(&listing).as_bytes(),
            )
        }
        "/api/packs" => {
            let listing = list_packs(pack_dir, pack_url, role.as_deref())?;
            let body = serde_json::to_vec_pretty(&listing)?;
            write_response(stream, 200, "application/json", &body)
        }
        _ if path.starts_with("/download/") => {
            let requested = percent_decode(&path["/download/".len()..])?;
            let archive_path = resolve_download(pack_dir, &requested)?;
            let body = fs::read(&archive_path)
                .with_context(|| format!("failed reading {}", archive_path.display()))?;
            write_response(stream, 200, "application/octet-stream", &body)
        }
        _ => write_response(stream, 404, "text/plain; charset=utf-8", b"not found\n"),
    }
}

fn valid_packs(pack_dir: &Path) -> Result<Vec<ValidPack>> {
    let mut packs = Vec::new();
    let entries = match fs::read_dir(pack_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(packs),
        Err(err) => {
            return Err(err).with_context(|| format!("failed reading {}", pack_dir.display()));
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        match load_valid_pack(pack_dir, &path) {
            Ok(pack) => packs.push(pack),
            Err(err) => eprintln!("hiding invalid child pack {}: {err}", path.display()),
        }
    }
    packs.sort_by(|left, right| {
        left.metadata
            .pack_id
            .cmp(&right.metadata.pack_id)
            .then(left.metadata.version.cmp(&right.metadata.version))
    });
    Ok(packs)
}

fn load_valid_pack(pack_dir: &Path, metadata_path: &Path) -> Result<ValidPack> {
    let raw = fs::read_to_string(metadata_path)
        .with_context(|| format!("failed reading {}", metadata_path.display()))?;
    let metadata = parse_metadata(&raw)?;
    validate_metadata(pack_dir, metadata)
}

pub fn parse_metadata(raw: &str) -> Result<PackMetadata> {
    let metadata: PackMetadata = toml::from_str(raw)?;
    if metadata.pack_id.trim().is_empty()
        || metadata.version.trim().is_empty()
        || metadata.desk.trim().is_empty()
        || metadata.created_at.trim().is_empty()
        || metadata.max_authority.trim().is_empty()
    {
        bail!("pack metadata contains empty required fields");
    }
    Ok(metadata)
}

fn validate_metadata(pack_dir: &Path, metadata: PackMetadata) -> Result<ValidPack> {
    validate_archive_name(&metadata.archive_name)?;
    validate_authority(&metadata.max_authority)?;
    if metadata.revoked {
        bail!("pack is revoked");
    }
    if metadata.script_execution {
        bail!("pack requests script execution");
    }
    let archive_path = pack_dir.join(&metadata.archive_name);
    let stat = fs::metadata(&archive_path)
        .with_context(|| format!("listed pack archive missing: {}", archive_path.display()))?;
    if !stat.is_file() {
        bail!("listed pack archive is not a file");
    }
    if stat.len() != metadata.archive_size {
        bail!(
            "archive size mismatch for {}: metadata={} actual={}",
            metadata.archive_name,
            metadata.archive_size,
            stat.len()
        );
    }
    let actual = sha256_file(&archive_path)?;
    if !metadata.sha256.eq_ignore_ascii_case(&actual) {
        bail!(
            "sha256 mismatch for {}: metadata={} actual={}",
            metadata.archive_name,
            metadata.sha256,
            actual
        );
    }
    Ok(ValidPack {
        metadata,
        archive_path,
    })
}

fn resolve_download(pack_dir: &Path, requested: &str) -> Result<PathBuf> {
    validate_archive_name(requested)?;
    let pack = valid_packs(pack_dir)?
        .into_iter()
        .find(|pack| pack.metadata.archive_name == requested)
        .ok_or_else(|| anyhow!("download is not listed in valid pack metadata"))?;
    Ok(pack.archive_path)
}

fn role_allowed(metadata: &PackMetadata, role: Option<&str>) -> bool {
    if metadata.allowed_roles.is_empty() {
        return true;
    }
    let Some(role) = role else {
        return false;
    };
    metadata.allowed_roles.iter().any(|allowed| allowed == role)
}

fn validate_archive_name(archive_name: &str) -> Result<()> {
    let path = Path::new(archive_name);
    if archive_name.trim().is_empty() || path.components().count() != 1 {
        bail!("archive_name must be a single file name");
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            bail!("archive_name cannot contain path traversal");
        }
    }
    Ok(())
}

fn validate_authority(value: &str) -> Result<()> {
    match value {
        "observe" | "evidence" | "none" => Ok(()),
        other => bail!("unsupported max_authority '{other}'; expected observe, evidence, or none"),
    }
}

fn render_pack(metadata: &PackMetadata, pack_url: &str) -> PackListingItem {
    let download_url = format!(
        "{}/download/{}",
        pack_url,
        url_path_encode(&metadata.archive_name)
    );
    let local_name = &metadata.archive_name;
    let suggested_sync_command = format!(
        "mkdir -p packs\ncurl -fL -o packs/{local_name} {download_url}\nprintf '%s  %s\\n' '{}' packs/{local_name} | sha256sum -c -\n# cache only; do not execute scripts from packs",
        metadata.sha256
    );
    PackListingItem {
        pack_id: metadata.pack_id.clone(),
        version: metadata.version.clone(),
        desk: metadata.desk.clone(),
        archive_name: metadata.archive_name.clone(),
        archive_size: metadata.archive_size,
        sha256: metadata.sha256.clone(),
        created_at: metadata.created_at.clone(),
        max_authority: metadata.max_authority.clone(),
        allowed_roles: metadata.allowed_roles.clone(),
        schemas: metadata.schemas.clone(),
        timing_policy: metadata.timing_policy.clone(),
        skills_manifest: metadata.skills_manifest.clone(),
        notes: metadata.notes.clone(),
        download_url,
        suggested_sync_command,
        heartbeat_active_pack: ActivePackReport {
            active_pack_id: metadata.pack_id.clone(),
            active_pack_version: metadata.version.clone(),
            active_pack_hash: metadata.sha256.clone(),
            max_authority: metadata.max_authority.clone(),
        },
        evidence_active_pack: EvidencePackReport {
            pack_id: metadata.pack_id.clone(),
            pack_version: metadata.version.clone(),
            pack_hash: metadata.sha256.clone(),
            non_authoritative: true,
        },
    }
}

fn render_html(listing: &PackListing) -> String {
    let mut out = String::from(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Quant-M Child Pack Sync</title><style>body{font-family:-apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;max-width:860px;margin:32px auto;padding:0 16px;line-height:1.45}pre{white-space:pre-wrap;overflow-wrap:anywhere;background:#f6f8fa;padding:12px;border-radius:6px}code{background:#f6f8fa;padding:2px 4px;border-radius:4px}</style></head><body><h1>Quant-M Child Pack Sync</h1><p>Download an approved knowledge pack from the core, verify it, cache it, and report its active hash in heartbeat/evidence. Packs do not grant execution authority.</p>",
    );
    if listing.packs.is_empty() {
        out.push_str("<p>No valid packs are available for this role.</p>");
    }
    for pack in &listing.packs {
        out.push_str(&format!(
            "<h2>{} {}</h2><p>desk={} max_authority={} sha256=<code>{}</code></p><p><a href=\"{}\">Download {}</a></p><h3>Sync</h3><pre>{}</pre><h3>Heartbeat report</h3><pre>{}</pre><h3>Evidence report</h3><pre>{}</pre>",
            escape_html(&pack.pack_id),
            escape_html(&pack.version),
            escape_html(&pack.desk),
            escape_html(&pack.max_authority),
            escape_html(&pack.sha256),
            escape_html(&pack.download_url),
            escape_html(&pack.archive_name),
            escape_html(&pack.suggested_sync_command),
            escape_html(&serde_json::to_string_pretty(&pack.heartbeat_active_pack).unwrap_or_default()),
            escape_html(&serde_json::to_string_pretty(&pack.evidence_active_pack).unwrap_or_default())
        ));
    }
    out.push_str("</body></html>");
    out
}

fn split_target_role(target: &str) -> Result<(String, Option<String>)> {
    let Some((path, query)) = target.split_once('?') else {
        return Ok((target.to_string(), None));
    };
    let mut role = None;
    for pair in query.split('&') {
        if let Some(value) = pair.strip_prefix("role=") {
            role = Some(percent_decode(value)?);
        }
    }
    Ok((path.to_string(), role))
}

fn trim_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

fn url_path_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn percent_decode(value: &str) -> Result<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                bail!("invalid percent encoding");
            }
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3])?;
            out.push(u8::from_str_radix(hex, 16)?);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    Ok(String::from_utf8(out)?)
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> Result<()> {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_pack(dir: &Path, archive_name: &str, body: &[u8]) -> PackMetadata {
        fs::write(dir.join(archive_name), body).expect("write archive");
        let sha256 = sha256_file(&dir.join(archive_name)).expect("sha256");
        let metadata = PackMetadata {
            pack_id: "forex-worker-basic".to_string(),
            version: "0.1.0".to_string(),
            desk: "forex".to_string(),
            archive_name: archive_name.to_string(),
            archive_size: body.len() as u64,
            sha256,
            created_at: "2026-06-30T00:00:00Z".to_string(),
            max_authority: "observe".to_string(),
            allowed_roles: vec!["forex_worker".to_string()],
            schemas: vec!["evidence.schema.json".to_string()],
            timing_policy: Some("timing.toml".to_string()),
            skills_manifest: Some("skills.manifest.json".to_string()),
            revoked: false,
            script_execution: false,
            notes: Some("test pack".to_string()),
        };
        fs::write(
            dir.join("pack.toml"),
            format!(
                "pack_id = \"{}\"\nversion = \"{}\"\ndesk = \"{}\"\narchive_name = \"{}\"\narchive_size = {}\nsha256 = \"{}\"\ncreated_at = \"{}\"\nmax_authority = \"{}\"\nallowed_roles = [\"{}\"]\nschemas = [\"{}\"]\ntiming_policy = \"{}\"\nskills_manifest = \"{}\"\nrevoked = false\nscript_execution = false\nnotes = \"{}\"\n",
                metadata.pack_id,
                metadata.version,
                metadata.desk,
                metadata.archive_name,
                metadata.archive_size,
                metadata.sha256,
                metadata.created_at,
                metadata.max_authority,
                metadata.allowed_roles[0],
                metadata.schemas[0],
                metadata.timing_policy.as_deref().unwrap(),
                metadata.skills_manifest.as_deref().unwrap(),
                metadata.notes.as_deref().unwrap()
            ),
        )
        .expect("write metadata");
        metadata
    }

    #[test]
    fn pack_metadata_parses() {
        let raw = r#"
pack_id = "forex-worker-basic"
version = "0.1.0"
desk = "forex"
archive_name = "forex-worker-basic.tar"
archive_size = 12
sha256 = "00"
created_at = "2026-06-30T00:00:00Z"
max_authority = "observe"
allowed_roles = ["forex_worker"]
schemas = ["evidence.schema.json"]
timing_policy = "timing.toml"
skills_manifest = "skills.manifest.json"
"#;
        let parsed = parse_metadata(raw).expect("metadata");
        assert_eq!(parsed.pack_id, "forex-worker-basic");
        assert_eq!(parsed.allowed_roles, vec!["forex_worker"]);
    }

    #[test]
    fn pack_listing_filters_by_role() {
        let dir = tempdir().expect("tempdir");
        write_pack(dir.path(), "pack.tar", b"pack");
        assert_eq!(
            list_packs(dir.path(), "http://core:8789", Some("forex_worker"))
                .expect("list")
                .packs
                .len(),
            1
        );
        assert_eq!(
            list_packs(dir.path(), "http://core:8789", Some("sports_worker"))
                .expect("list")
                .packs
                .len(),
            0
        );
    }

    #[test]
    fn pack_checksum_mismatch_is_hidden() {
        let dir = tempdir().expect("tempdir");
        write_pack(dir.path(), "pack.tar", b"pack");
        let raw = fs::read_to_string(dir.path().join("pack.toml"))
            .expect("read metadata")
            .replace("sha256 = \"", "sha256 = \"dead");
        fs::write(dir.path().join("pack.toml"), raw).expect("rewrite");
        assert!(
            list_packs(dir.path(), "http://core:8789", Some("forex_worker"))
                .expect("list")
                .packs
                .is_empty()
        );
    }

    #[test]
    fn revoked_pack_is_hidden() {
        let dir = tempdir().expect("tempdir");
        write_pack(dir.path(), "pack.tar", b"pack");
        let raw = fs::read_to_string(dir.path().join("pack.toml"))
            .expect("read metadata")
            .replace("revoked = false", "revoked = true");
        fs::write(dir.path().join("pack.toml"), raw).expect("rewrite");
        assert!(
            list_packs(dir.path(), "http://core:8789", Some("forex_worker"))
                .expect("list")
                .packs
                .is_empty()
        );
    }

    #[test]
    fn script_execution_pack_is_hidden() {
        let dir = tempdir().expect("tempdir");
        write_pack(dir.path(), "pack.tar", b"pack");
        let raw = fs::read_to_string(dir.path().join("pack.toml"))
            .expect("read metadata")
            .replace("script_execution = false", "script_execution = true");
        fs::write(dir.path().join("pack.toml"), raw).expect("rewrite");
        assert!(
            list_packs(dir.path(), "http://core:8789", Some("forex_worker"))
                .expect("list")
                .packs
                .is_empty()
        );
    }

    #[test]
    fn path_traversal_and_unlisted_downloads_are_rejected() {
        let dir = tempdir().expect("tempdir");
        write_pack(dir.path(), "pack.tar", b"pack");
        fs::write(dir.path().join("other.tar"), b"other").expect("other");
        assert!(validate_archive_name("../pack.tar").is_err());
        assert!(validate_archive_name("nested/pack.tar").is_err());
        assert!(resolve_download(dir.path(), "other.tar").is_err());
        assert!(resolve_download(dir.path(), "pack.tar").is_ok());
    }

    #[test]
    fn sync_instructions_report_active_pack_without_execution() {
        let dir = tempdir().expect("tempdir");
        let metadata = write_pack(dir.path(), "pack.tar", b"pack");
        let listing =
            list_packs(dir.path(), "http://core:8789", Some("forex_worker")).expect("list packs");
        let pack = &listing.packs[0];
        assert!(pack.suggested_sync_command.contains("curl -fL"));
        assert!(pack.suggested_sync_command.contains("sha256sum -c -"));
        assert!(
            pack.suggested_sync_command
                .contains("do not execute scripts")
        );
        assert_eq!(pack.heartbeat_active_pack.active_pack_hash, metadata.sha256);
        assert_eq!(pack.evidence_active_pack.pack_hash, metadata.sha256);
        assert!(pack.evidence_active_pack.non_authoritative);
        assert!(!pack.suggested_sync_command.contains("bash "));
        assert!(!pack.suggested_sync_command.contains("cargo"));
        assert!(!pack.suggested_sync_command.contains("git "));
    }
}
