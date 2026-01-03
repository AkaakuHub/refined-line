use anyhow::{anyhow, Result};
use base64::engine::general_purpose::STANDARD as base64_standard;
use base64::Engine;
use prost::Message;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use tauri::path::BaseDirectory;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri::webview::PageLoadEvent;
use url::Url;
use zip::ZipArchive;
#[cfg(target_os = "windows")]
use tauri::webview::PlatformWebview;
#[cfg(target_os = "windows")]
use webview2_com::{
    wait_with_pump,
    BrowserExtensionEnableCompletedHandler,
    ProfileAddBrowserExtensionCompletedHandler,
    take_pwstr,
};
#[cfg(target_os = "windows")]
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2BrowserExtension,
    ICoreWebView2Profile7,
    ICoreWebView2_13,
};
#[cfg(target_os = "windows")]
use windows::core::{HSTRING, Interface, PWSTR};
#[cfg(target_os = "windows")]
use windows::core::BOOL;

const FONT_SCRIPT: &str = include_str!("../injections/font.js");
const SIDEBAR_SCRIPT: &str = include_str!("../injections/sidebar.js");

#[derive(serde::Deserialize)]
struct AppConfig {
    #[serde(rename = "lineExtensionId")]
    line_extension_id: String,
    #[serde(rename = "lineEntryPath")]
    line_entry_path: String,
    #[serde(rename = "update2BaseUrl")]
    update2_base_url: String,
}

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

struct ParsedCrx {
    public_key: Vec<u8>,
    zip_bytes: Vec<u8>,
}

fn build_update_url(base: &str, extension_id: &str) -> String {
    format!("{base}?response=redirect&os=win&arch=x64&os_arch=x86_64&nacl_arch=x86-64&prod=chromecrx&prodchannel=unknown&prodversion=120.0.0.0&acceptformat=crx2%2Ccrx3&x=id%3D{extension_id}%26installsource%3Dondemand%26uc")
}

fn download_crx(url: &str) -> Result<Vec<u8>> {
    let mut current = Url::parse(url)?;
    for _ in 0..5 {
        let response = ureq::get(current.as_str())
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

fn parse_crx3(bytes: &[u8]) -> Result<ParsedCrx> {
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
    let crx_id = signed_data.crx_id.ok_or_else(|| anyhow!("missing crx_id"))?;
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

    hex.chars()
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

fn ensure_clean_dir(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn extract_zip(zip_bytes: &[u8], dest: &Path) -> Result<()> {
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

fn inject_manifest_key(extension_dir: &Path, public_key: &[u8]) -> Result<()> {
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

fn prepare_extensions(app: &tauri::AppHandle) -> Result<(PathBuf, PathBuf)> {
    let config = load_config(app)?;
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| anyhow!("app data dir error: {error}"))?;

    let extensions_root = app_data.join("extensions");
    let line_dir = extensions_root.join("line");
    let user_dir = extensions_root.join("user");

    fs::create_dir_all(&user_dir)?;

    let update_url = build_update_url(&config.update2_base_url, &config.line_extension_id);
    println!("[update] start {}", update_url);
    let crx_bytes = download_crx(&update_url)?;
    let parsed = parse_crx3(&crx_bytes)?;

    ensure_clean_dir(&line_dir)?;
    extract_zip(&parsed.zip_bytes, &line_dir)?;
    inject_manifest_key(&line_dir, &parsed.public_key)?;

    Ok((line_dir, user_dir))
}

#[cfg(target_os = "windows")]
fn add_browser_extension(
    profile: &ICoreWebView2Profile7,
    extension_dir: &Path,
) -> Result<ICoreWebView2BrowserExtension> {
    let path = extension_dir.canonicalize()?;
    let path_hs = HSTRING::from(path.as_path());
    let (tx, rx) = mpsc::channel();
    let handler = ProfileAddBrowserExtensionCompletedHandler::create(Box::new(
        move |result, extension| {
            let _ = tx.send((result, extension));
            Ok(())
        },
    ));

    unsafe {
        profile.AddBrowserExtension(&path_hs, &handler)?;
    }

    let (result, extension) = wait_with_pump(rx)
        .map_err(|error| anyhow!("extension install callback error: {error:?}"))?;

    if let Err(error) = result {
        return Err(anyhow!("add extension failed: {error:?}"));
    }

    extension.ok_or_else(|| anyhow!("add extension returned no extension"))
}

#[cfg(target_os = "windows")]
fn browser_extension_id(extension: &ICoreWebView2BrowserExtension) -> Result<String> {
    let mut id_ptr = PWSTR::null();
    unsafe {
        extension.Id(&mut id_ptr)?;
    }
    Ok(take_pwstr(id_ptr))
}

#[cfg(target_os = "windows")]
fn ensure_extension_enabled(extension: &ICoreWebView2BrowserExtension) -> Result<()> {
    let mut enabled = BOOL(0);
    unsafe {
        extension.IsEnabled(&mut enabled)?;
    }
    if enabled == true {
        return Ok(());
    }

    let (tx, rx) = mpsc::channel();
    let handler = BrowserExtensionEnableCompletedHandler::create(Box::new(move |result| {
        let _ = tx.send(result);
        Ok(())
    }));

    unsafe {
        extension.Enable(true, &handler)?;
    }

    let result =
        wait_with_pump(rx).map_err(|error| anyhow!("extension enable callback error: {error:?}"))?;
    if let Err(error) = result {
        return Err(anyhow!("enable extension failed: {error:?}"));
    }

    Ok(())
}

fn is_extension_dir(path: &Path) -> bool {
    path.join("manifest.json").is_file()
}

fn collect_user_extension_dirs(user_dir: &Path) -> Result<Vec<PathBuf>> {
    if is_extension_dir(user_dir) {
        return Ok(vec![user_dir.to_path_buf()]);
    }

    let mut dirs = Vec::new();
    if user_dir.is_dir() {
        for entry in fs::read_dir(user_dir)? {
            let path = entry?.path();
            if path.is_dir() && is_extension_dir(&path) {
                dirs.push(path);
            }
        }
    }

    Ok(dirs)
}

#[cfg(target_os = "windows")]
fn install_extensions_and_open(
    webview: PlatformWebview,
    line_dir: PathBuf,
    user_dir: PathBuf,
    entry_path: String,
) -> Result<()> {
    let controller = webview.controller();
    let core = unsafe { controller.CoreWebView2()? };
    unsafe {
        let settings = core.Settings()?;
        settings.SetIsScriptEnabled(true)?;
    }
    let profile = unsafe {
        core
            .cast::<ICoreWebView2_13>()?
            .Profile()?
            .cast::<ICoreWebView2Profile7>()?
    };

    let line_extension = add_browser_extension(&profile, &line_dir)?;
    ensure_extension_enabled(&line_extension)?;
    let line_id = browser_extension_id(&line_extension)?;

    for user_extension in collect_user_extension_dirs(&user_dir)? {
        let extension = add_browser_extension(&profile, &user_extension)?;
        ensure_extension_enabled(&extension)?;
    }

    let page_url = format!("chrome-extension://{line_id}{entry_path}");
    println!("[open] {}", page_url);
    let target = HSTRING::from(page_url.as_str());
    unsafe {
        core.Navigate(&target)?;
    }

    Ok(())
}

fn inject_scripts(window: &tauri::WebviewWindow) -> Result<(), tauri::Error> {
    window.eval(FONT_SCRIPT)?;
    window.eval(SIDEBAR_SCRIPT)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle();
            let (line_dir, user_dir) = match prepare_extensions(&app_handle) {
                Ok(result) => result,
                Err(error) => {
                    eprintln!("[update] failed: {error:#}");
                    return Err(Box::<dyn std::error::Error>::from(error));
                }
            };

            let config = load_config(&app_handle)?;

            WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::App("index.html".into()),
            )
            .title("better-line")
            .inner_size(1280.0, 800.0)
            .browser_extensions_enabled(true)
            .on_page_load({
                move |window, payload| {
                    if payload.event() == PageLoadEvent::Finished {
                        let current_url = payload.url().as_str();
                        if current_url.starts_with("chrome-extension://") {
                            let _ = inject_scripts(&window);
                            return;
                        }
                    }
                }
            })
            .build()?
            .with_webview(move |webview| {
                let result = install_extensions_and_open(
                    webview,
                    line_dir,
                    user_dir,
                    config.line_entry_path,
                );

                if let Err(error) = result {
                    eprintln!("[open] failed: {error:#}");
                    panic!("failed to open LINE extension");
                }
            })?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn load_config(app: &tauri::AppHandle) -> Result<AppConfig> {
    let config_path = app
        .path()
        .resolve("config.json", BaseDirectory::Resource)
        .map_err(|error| anyhow!("config path error: {error}"))?;
    let raw = fs::read_to_string(&config_path)?;
    let config: AppConfig = serde_json::from_str(&raw)?;
    Ok(config)
}
