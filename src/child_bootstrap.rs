use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

const MAX_REQUEST_LINE: usize = 4096;

#[derive(Debug, Clone, Deserialize)]
pub struct ChildBundleMetadata {
    pub binary_name: String,
    pub version: String,
    pub commit: String,
    pub platform: String,
    pub architecture: String,
    pub abi: Option<String>,
    pub file_name: String,
    pub file_size: u64,
    pub sha256: String,
    pub created_at: String,
    pub min_core_version: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapBundle {
    pub binary_name: String,
    pub version: String,
    pub commit: String,
    pub platform: String,
    pub architecture: String,
    pub abi: Option<String>,
    pub file_name: String,
    pub file_size: u64,
    pub sha256: String,
    pub created_at: String,
    pub min_core_version: Option<String>,
    pub notes: Option<String>,
    pub download_url: String,
    pub suggested_install_command: String,
    pub suggested_pairing_command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapListing {
    pub bootstrap_url: String,
    pub core_pairing_url: String,
    pub bundles: Vec<BootstrapBundle>,
}

#[derive(Debug, Clone)]
struct ValidatedBundle {
    metadata: ChildBundleMetadata,
    file_path: PathBuf,
}

pub fn list_bundles(
    bundle_dir: &Path,
    bootstrap_url: &str,
    core_pairing_url: &str,
) -> Result<BootstrapListing> {
    let bootstrap_url = trim_url(bootstrap_url);
    let core_pairing_url = trim_url(core_pairing_url);
    let bundles = valid_bundles(bundle_dir)?
        .into_iter()
        .map(|bundle| {
            render_bundle(
                &bundle.metadata,
                &bootstrap_url,
                &core_pairing_url,
                "android-tablet-01",
            )
        })
        .collect();
    Ok(BootstrapListing {
        bootstrap_url,
        core_pairing_url,
        bundles,
    })
}

pub fn render_listing_text(listing: &BootstrapListing) -> String {
    let mut out = String::from("Quant-M child binary bootstrap\n");
    out.push_str(&format!("bootstrap_url: {}\n", listing.bootstrap_url));
    out.push_str(&format!("core_pairing_url: {}\n", listing.core_pairing_url));
    if listing.bundles.is_empty() {
        out.push_str("bundles: none valid\n");
        out.push_str(
            "expected: place *.toml metadata files and listed binaries in the bundle dir\n",
        );
        return out;
    }
    for bundle in &listing.bundles {
        out.push_str(&format!(
            "\n{} version={} platform={} arch={} abi={} size={} sha256={}\n",
            bundle.binary_name,
            bundle.version,
            bundle.platform,
            bundle.architecture,
            bundle.abi.as_deref().unwrap_or("none"),
            bundle.file_size,
            bundle.sha256
        ));
        out.push_str(&format!("download: {}\n", bundle.download_url));
        out.push_str("install:\n");
        out.push_str(&bundle.suggested_install_command);
        out.push('\n');
        out.push_str(&format!("pair:\n{}\n", bundle.suggested_pairing_command));
    }
    out
}

pub fn serve(bundle_dir: PathBuf, bind: &str, core_pairing_url: &str) -> Result<()> {
    let listener = TcpListener::bind(bind).with_context(|| format!("failed to bind {bind}"))?;
    let local_addr = listener.local_addr()?;
    let bootstrap_url = format!("http://{local_addr}");
    println!("Quant-M child bootstrap serving");
    println!("bootstrap_url: {bootstrap_url}");
    println!("bundle_dir: {}", bundle_dir.display());
    println!("core_pairing_url: {}", trim_url(core_pairing_url));
    println!("safety: only metadata-listed files with matching SHA-256 are downloadable");
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(err) =
                    handle_connection(&mut stream, &bundle_dir, &bootstrap_url, core_pairing_url)
                {
                    let _ = write_response(
                        &mut stream,
                        500,
                        "text/plain; charset=utf-8",
                        format!("bootstrap error: {err}\n").as_bytes(),
                    );
                }
            }
            Err(err) => eprintln!("bootstrap connection failed: {err}"),
        }
    }
    Ok(())
}

fn handle_connection(
    stream: &mut TcpStream,
    bundle_dir: &Path,
    bootstrap_url: &str,
    core_pairing_url: &str,
) -> Result<()> {
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
    match target {
        "/" | "/index.html" => {
            let listing = list_bundles(bundle_dir, bootstrap_url, core_pairing_url)?;
            write_response(
                stream,
                200,
                "text/html; charset=utf-8",
                render_html(&listing).as_bytes(),
            )
        }
        "/api/bundles" => {
            let listing = list_bundles(bundle_dir, bootstrap_url, core_pairing_url)?;
            let body = serde_json::to_vec_pretty(&listing)?;
            write_response(stream, 200, "application/json", &body)
        }
        _ if target.starts_with("/download/") => {
            let requested = percent_decode(&target["/download/".len()..])?;
            let file_path = resolve_download(bundle_dir, &requested)?;
            let body = fs::read(&file_path)
                .with_context(|| format!("failed reading {}", file_path.display()))?;
            write_response(stream, 200, "application/octet-stream", &body)
        }
        _ => write_response(stream, 404, "text/plain; charset=utf-8", b"not found\n"),
    }
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

fn valid_bundles(bundle_dir: &Path) -> Result<Vec<ValidatedBundle>> {
    let mut bundles = Vec::new();
    let entries = match fs::read_dir(bundle_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(bundles),
        Err(err) => {
            return Err(err).with_context(|| format!("failed reading {}", bundle_dir.display()));
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        match load_valid_bundle(bundle_dir, &path) {
            Ok(bundle) => bundles.push(bundle),
            Err(err) => eprintln!("hiding invalid bootstrap bundle {}: {err}", path.display()),
        }
    }
    bundles.sort_by(|left, right| left.metadata.file_name.cmp(&right.metadata.file_name));
    Ok(bundles)
}

fn load_valid_bundle(bundle_dir: &Path, metadata_path: &Path) -> Result<ValidatedBundle> {
    let raw = fs::read_to_string(metadata_path)
        .with_context(|| format!("failed reading {}", metadata_path.display()))?;
    let metadata = parse_metadata(&raw)?;
    validate_metadata(bundle_dir, metadata)
}

pub fn parse_metadata(raw: &str) -> Result<ChildBundleMetadata> {
    let metadata: ChildBundleMetadata = toml::from_str(raw)?;
    if metadata.binary_name.trim().is_empty()
        || metadata.version.trim().is_empty()
        || metadata.commit.trim().is_empty()
        || metadata.platform.trim().is_empty()
        || metadata.architecture.trim().is_empty()
        || metadata.created_at.trim().is_empty()
    {
        bail!("bundle metadata contains empty required fields");
    }
    Ok(metadata)
}

fn validate_metadata(bundle_dir: &Path, metadata: ChildBundleMetadata) -> Result<ValidatedBundle> {
    validate_file_name(&metadata.file_name)?;
    let file_path = bundle_dir.join(&metadata.file_name);
    let stat = fs::metadata(&file_path)
        .with_context(|| format!("listed binary missing: {}", file_path.display()))?;
    if !stat.is_file() {
        bail!("listed binary is not a file");
    }
    if stat.len() != metadata.file_size {
        bail!(
            "file size mismatch for {}: metadata={} actual={}",
            metadata.file_name,
            metadata.file_size,
            stat.len()
        );
    }
    let actual = sha256_file(&file_path)?;
    if !metadata.sha256.eq_ignore_ascii_case(&actual) {
        bail!(
            "sha256 mismatch for {}: metadata={} actual={}",
            metadata.file_name,
            metadata.sha256,
            actual
        );
    }
    Ok(ValidatedBundle {
        metadata,
        file_path,
    })
}

fn resolve_download(bundle_dir: &Path, requested: &str) -> Result<PathBuf> {
    validate_file_name(requested)?;
    let bundle = valid_bundles(bundle_dir)?
        .into_iter()
        .find(|bundle| bundle.metadata.file_name == requested)
        .ok_or_else(|| anyhow!("download is not listed in valid bundle metadata"))?;
    Ok(bundle.file_path)
}

fn validate_file_name(file_name: &str) -> Result<()> {
    let path = Path::new(file_name);
    if file_name.trim().is_empty() || path.components().count() != 1 {
        bail!("bundle file_name must be a single file name");
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            bail!("bundle file_name cannot contain path traversal");
        }
    }
    Ok(())
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("failed opening {} for sha256", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn render_bundle(
    metadata: &ChildBundleMetadata,
    bootstrap_url: &str,
    core_pairing_url: &str,
    child_name: &str,
) -> BootstrapBundle {
    let download_url = format!(
        "{}/download/{}",
        bootstrap_url,
        url_path_encode(&metadata.file_name)
    );
    let local_name = &metadata.binary_name;
    let suggested_install_command = format!(
        "pkg update\npkg install curl openssh termux-api\ncurl -fL -o {local_name} {download_url}\nprintf '%s  %s\\n' '{}' {local_name} | sha256sum -c -\nchmod +x {local_name}",
        metadata.sha256
    );
    let suggested_pairing_command =
        format!("./{local_name} pair --core {core_pairing_url} --name {child_name}");
    BootstrapBundle {
        binary_name: metadata.binary_name.clone(),
        version: metadata.version.clone(),
        commit: metadata.commit.clone(),
        platform: metadata.platform.clone(),
        architecture: metadata.architecture.clone(),
        abi: metadata.abi.clone(),
        file_name: metadata.file_name.clone(),
        file_size: metadata.file_size,
        sha256: metadata.sha256.clone(),
        created_at: metadata.created_at.clone(),
        min_core_version: metadata.min_core_version.clone(),
        notes: metadata.notes.clone(),
        download_url,
        suggested_install_command,
        suggested_pairing_command,
    }
}

fn render_html(listing: &BootstrapListing) -> String {
    let mut out = String::from(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Quant-M Child Bootstrap</title><style>body{font-family:-apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;max-width:860px;margin:32px auto;padding:0 16px;line-height:1.45}pre{white-space:pre-wrap;overflow-wrap:anywhere;background:#f6f8fa;padding:12px;border-radius:6px}code{background:#f6f8fa;padding:2px 4px;border-radius:4px}</style></head><body><h1>Quant-M Child Bootstrap</h1><p>Download a verified observe-only child binary from this core, then pair manually. No GitHub clone or Cargo build is required on the child.</p>",
    );
    out.push_str(&format!(
        "<p><strong>Core pairing URL:</strong> <code>{}</code></p>",
        escape_html(&listing.core_pairing_url)
    ));
    if listing.bundles.is_empty() {
        out.push_str("<p>No valid child bundles are available.</p>");
    }
    for bundle in &listing.bundles {
        out.push_str(&format!(
            "<h2>{} {}</h2><p>platform={} architecture={} abi={} size={} sha256=<code>{}</code></p><p><a href=\"{}\">Download {}</a></p><h3>Install</h3><pre>{}</pre><h3>Pair</h3><pre>{}</pre>",
            escape_html(&bundle.binary_name),
            escape_html(&bundle.version),
            escape_html(&bundle.platform),
            escape_html(&bundle.architecture),
            escape_html(bundle.abi.as_deref().unwrap_or("none")),
            bundle.file_size,
            escape_html(&bundle.sha256),
            escape_html(&bundle.download_url),
            escape_html(&bundle.file_name),
            escape_html(&bundle.suggested_install_command),
            escape_html(&bundle.suggested_pairing_command)
        ));
    }
    out.push_str("</body></html>");
    out
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_bundle(dir: &Path, file_name: &str, body: &[u8]) -> ChildBundleMetadata {
        fs::write(dir.join(file_name), body).expect("write binary");
        let sha256 = sha256_file(&dir.join(file_name)).expect("sha256");
        let metadata = ChildBundleMetadata {
            binary_name: "quant-m-child".to_string(),
            version: "0.1.0".to_string(),
            commit: "abc1234".to_string(),
            platform: "android".to_string(),
            architecture: "armv7".to_string(),
            abi: Some("armeabi-v7a".to_string()),
            file_name: file_name.to_string(),
            file_size: body.len() as u64,
            sha256,
            created_at: "2026-06-30T00:00:00Z".to_string(),
            min_core_version: Some("0.1.0".to_string()),
            notes: Some("test bundle".to_string()),
        };
        fs::write(
            dir.join("bundle.toml"),
            format!(
                "binary_name = \"{}\"\nversion = \"{}\"\ncommit = \"{}\"\nplatform = \"{}\"\narchitecture = \"{}\"\nabi = \"{}\"\nfile_name = \"{}\"\nfile_size = {}\nsha256 = \"{}\"\ncreated_at = \"{}\"\nmin_core_version = \"{}\"\nnotes = \"{}\"\n",
                metadata.binary_name,
                metadata.version,
                metadata.commit,
                metadata.platform,
                metadata.architecture,
                metadata.abi.as_deref().unwrap(),
                metadata.file_name,
                metadata.file_size,
                metadata.sha256,
                metadata.created_at,
                metadata.min_core_version.as_deref().unwrap(),
                metadata.notes.as_deref().unwrap()
            ),
        )
        .expect("write metadata");
        metadata
    }

    #[test]
    fn bootstrap_metadata_parses() {
        let raw = r#"
binary_name = "quant-m-child"
version = "0.1.0"
commit = "abc1234"
platform = "android"
architecture = "armv7"
abi = "armeabi-v7a"
file_name = "quant-m-child"
file_size = 12
sha256 = "00"
created_at = "2026-06-30T00:00:00Z"
"#;
        let parsed = parse_metadata(raw).expect("metadata");
        assert_eq!(parsed.binary_name, "quant-m-child");
        assert_eq!(parsed.abi.as_deref(), Some("armeabi-v7a"));
    }

    #[test]
    fn bootstrap_checksum_calculation_matches_known_value() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("sample.bin");
        fs::write(&path, b"abc").expect("write");
        assert_eq!(
            sha256_file(&path).expect("sha"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn bootstrap_checksum_mismatch_is_hidden() {
        let dir = tempdir().expect("tempdir");
        let mut metadata = write_bundle(dir.path(), "quant-m-child", b"child");
        metadata.sha256 = "deadbeef".to_string();
        fs::write(
            dir.path().join("bundle.toml"),
            format!(
                "binary_name = \"{}\"\nversion = \"{}\"\ncommit = \"{}\"\nplatform = \"{}\"\narchitecture = \"{}\"\nfile_name = \"{}\"\nfile_size = {}\nsha256 = \"{}\"\ncreated_at = \"{}\"\n",
                metadata.binary_name,
                metadata.version,
                metadata.commit,
                metadata.platform,
                metadata.architecture,
                metadata.file_name,
                metadata.file_size,
                metadata.sha256,
                metadata.created_at
            ),
        )
        .expect("rewrite metadata");
        let listed =
            list_bundles(dir.path(), "http://core:8788", "http://core:8787").expect("list bundles");
        assert!(listed.bundles.is_empty());
    }

    #[test]
    fn bootstrap_path_traversal_is_rejected() {
        assert!(validate_file_name("../secret").is_err());
        assert!(validate_file_name("nested/child").is_err());
        assert!(validate_file_name("quant-m-child").is_ok());
    }

    #[test]
    fn bootstrap_unlisted_file_download_is_rejected() {
        let dir = tempdir().expect("tempdir");
        write_bundle(dir.path(), "quant-m-child", b"child");
        fs::write(dir.path().join("other"), b"other").expect("other");
        let err = resolve_download(dir.path(), "other").unwrap_err();
        assert!(err.to_string().contains("not listed"));
    }

    #[test]
    fn bootstrap_listing_includes_only_valid_bundles() {
        let dir = tempdir().expect("tempdir");
        let metadata = write_bundle(dir.path(), "quant-m-child", b"child");
        fs::write(
            dir.path().join("bad.toml"),
            "binary_name = \"quant-m-child\"\nversion = \"0.1.0\"\ncommit = \"abc\"\nplatform = \"android\"\narchitecture = \"arm\"\nfile_name = \"missing\"\nfile_size = 1\nsha256 = \"00\"\ncreated_at = \"now\"\n",
        )
        .expect("bad metadata");
        let listed =
            list_bundles(dir.path(), "http://core:8788", "http://core:8787").expect("list bundles");
        assert_eq!(listed.bundles.len(), 1);
        assert_eq!(listed.bundles[0].sha256, metadata.sha256);
    }

    #[test]
    fn bootstrap_install_instructions_are_child_only() {
        let dir = tempdir().expect("tempdir");
        write_bundle(dir.path(), "quant-m-child", b"child");
        let listed =
            list_bundles(dir.path(), "http://core:8788", "http://core:8787").expect("list bundles");
        let instructions = &listed.bundles[0].suggested_install_command;
        assert!(instructions.contains("curl -fL"));
        assert!(instructions.contains("sha256sum -c -"));
        assert!(instructions.contains("chmod +x"));
        assert!(
            listed.bundles[0]
                .suggested_pairing_command
                .contains(" pair ")
        );
        assert!(!instructions.to_ascii_lowercase().contains("cargo"));
        assert!(!instructions.to_ascii_lowercase().contains("git "));
    }
}
