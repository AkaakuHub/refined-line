use anyhow::{anyhow, Result};
use base64::engine::general_purpose::STANDARD as base64_standard;
use base64::Engine;
use prost::Message;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::time::Duration;
use url::Url;
use zip::ZipArchive;

#[derive(Clone, PartialEq, Message)]
struct AsymmetricKeyProof {
  #[prost(bytes, optional, tag = "1")]
  public_key: Option<Vec<u8>>,
  #[prost(bytes, optional, tag = "2")]
  signature: Option<Vec<u8>>,
}

#[derive(Clone, PartialEq, Message)]
struct CrxFileHeader {
  #[prost(message, repeated, tag = "2")]
  sha256_with_rsa: Vec<AsymmetricKeyProof>,
  #[prost(message, repeated, tag = "3")]
  sha256_with_ecdsa: Vec<AsymmetricKeyProof>,
  #[prost(bytes, optional, tag = "10000")]
  signed_header_data: Option<Vec<u8>>,
}

#[derive(Clone, PartialEq, Message)]
struct SignedData {
  #[prost(bytes, optional, tag = "1")]
  crx_id: Option<Vec<u8>>,
}

pub(crate) struct ParsedCrx {
  pub(crate) public_key: Vec<u8>,
  pub(crate) zip_bytes: Vec<u8>,
}

pub(crate) fn build_update_url(base: &str, extension_id: &str, version: Option<&str>) -> String {
  let mut x = format!("id%3D{extension_id}%26installsource%3Dondemand");
  if let Some(version) = version {
    x.push_str(&format!("%26v%3D{version}"));
  }
  x.push_str("%26uc");
  format!("{base}?response=redirect&os=win&arch=x64&os_arch=x86_64&nacl_arch=x86-64&prod=chromecrx&prodchannel=unknown&prodversion=120.0.0.0&acceptformat=crx2%2Ccrx3&x={x}")
}

pub(crate) enum UpdateCheck {
  NoUpdate,
  UpdateAvailable(Option<Vec<u8>>),
}

pub(crate) fn check_update(url: &str) -> Result<UpdateCheck> {
  let agent = ureq::AgentBuilder::new()
    .timeout_connect(Duration::from_secs(10))
    .timeout_read(Duration::from_secs(10))
    .timeout_write(Duration::from_secs(10))
    .redirects(0)
    .build();
  let response = agent
    .get(url)
    .call()
    .map_err(|error| anyhow!("update check failed: {error}"))?;

  match response.status() {
    204 => Ok(UpdateCheck::NoUpdate),
    200 => {
      let mut reader = response.into_reader();
      let mut buffer = Vec::new();
      reader.read_to_end(&mut buffer)?;
      Ok(UpdateCheck::UpdateAvailable(Some(buffer)))
    }
    301 | 302 | 307 | 308 => Ok(UpdateCheck::UpdateAvailable(None)),
    status => Err(anyhow!("update check failed: {}", status)),
  }
}

pub(crate) fn download_crx(url: &str) -> Result<Vec<u8>> {
  let agent = ureq::AgentBuilder::new()
    .timeout_connect(Duration::from_secs(10))
    .timeout_read(Duration::from_secs(30))
    .timeout_write(Duration::from_secs(30))
    .build();
  let mut current = Url::parse(url)?;
  for _ in 0..5 {
    let response = agent
      .get(current.as_str())
      .call()
      .map_err(|error| anyhow!("download failed: {error}"))?;

    if response.status() == 200 {
      let mut reader = response.into_reader();
      let mut buffer = Vec::new();
      reader.read_to_end(&mut buffer)?;
      return Ok(buffer);
    }

    if response.status() == 302 || response.status() == 301 {
      if let Some(location) = response.header("Location") {
        current = current.join(location)?;
        continue;
      }
    }

    return Err(anyhow!("download failed: {}", response.status()));
  }

  Err(anyhow!("download failed: too many redirects"))
}

pub(crate) fn parse_crx3(bytes: &[u8]) -> Result<ParsedCrx> {
  if bytes.len() < 12 {
    return Err(anyhow!("crx too small"));
  }

  if &bytes[0..4] != b"Cr24" {
    return Err(anyhow!("invalid crx magic"));
  }

  let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
  if version != 3 {
    return Err(anyhow!("unsupported crx version"));
  }

  let header_size = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
  let header_start = 12;
  let header_end = header_start + header_size;
  if bytes.len() < header_end {
    return Err(anyhow!("crx header truncated"));
  }

  let header = CrxFileHeader::decode(&bytes[header_start..header_end])?;
  let signed_header = header
    .signed_header_data
    .ok_or_else(|| anyhow!("missing signed_header_data"))?;
  let signed_data = SignedData::decode(signed_header.as_slice())?;
  let crx_id = signed_data
    .crx_id
    .ok_or_else(|| anyhow!("missing crx_id"))?;
  let expected_id = format_extension_id(&crx_id);
  if expected_id.len() != 32 {
    return Err(anyhow!("invalid crx_id length: {}", expected_id.len()));
  }

  let mut public_key = None;
  for proof in header
    .sha256_with_rsa
    .iter()
    .chain(header.sha256_with_ecdsa.iter())
  {
    if let Some(candidate) = proof.public_key.as_ref() {
      if extension_id_from_public_key(candidate) == expected_id {
        public_key = Some(candidate.clone());
        break;
      }
    }
  }

  let public_key = public_key.ok_or_else(|| anyhow!("no public_key matched crx_id"))?;

  Ok(ParsedCrx {
    public_key,
    zip_bytes: bytes[header_end..].to_vec(),
  })
}

fn format_extension_id(raw_id: &[u8]) -> String {
  let mut hex = String::with_capacity(raw_id.len() * 2);
  for byte in raw_id {
    use std::fmt::Write;
    write!(&mut hex, "{:02x}", byte).unwrap();
  }

  hex
    .chars()
    .map(|c| {
      let n = c.to_digit(16).unwrap_or(0);
      let code = (n as u8) + b'a';
      code as char
    })
    .collect()
}

fn extension_id_from_public_key(public_key: &[u8]) -> String {
  let digest = Sha256::digest(public_key);
  format_extension_id(&digest[..16])
}

pub(crate) fn ensure_clean_dir(path: &Path) -> Result<()> {
  if path.exists() {
    fs::remove_dir_all(path)?;
  }
  fs::create_dir_all(path)?;
  Ok(())
}

pub(crate) fn extract_zip(zip_bytes: &[u8], dest: &Path) -> Result<()> {
  let reader = Cursor::new(zip_bytes);
  let mut archive = ZipArchive::new(reader)?;

  for i in 0..archive.len() {
    let mut file = archive.by_index(i)?;
    let Some(path) = file.enclosed_name() else {
      continue;
    };
    let out_path = dest.join(path);

    if file.name().ends_with('/') {
      fs::create_dir_all(&out_path)?;
      continue;
    }

    if let Some(parent) = out_path.parent() {
      fs::create_dir_all(parent)?;
    }

    let mut out_file = fs::File::create(&out_path)?;
    std::io::copy(&mut file, &mut out_file)?;
  }

  Ok(())
}

pub(crate) fn inject_manifest_key(extension_dir: &Path, public_key: &[u8]) -> Result<()> {
  let manifest_path = extension_dir.join("manifest.json");
  let raw = fs::read_to_string(&manifest_path)?;
  let mut value: Value = serde_json::from_str(&raw)?;
  let key = base64_standard.encode(public_key);
  value["key"] = Value::String(key);
  let pretty = serde_json::to_string_pretty(&value)?;
  let mut file = fs::File::create(&manifest_path)?;
  file.write_all(pretty.as_bytes())?;
  Ok(())
}
