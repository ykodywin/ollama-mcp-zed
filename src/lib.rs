//! Zed extension that runs the Ollama web search / web fetch MCP server.
//!
//! The extension itself runs in Zed's WASM sandbox and therefore cannot ship
//! or execute the MCP server logic directly. Instead it downloads the
//! prebuilt native `ollama-search-mcp` binary for the current platform from
//! GitHub Releases into the extension's working directory and returns a
//! `Command` that Zed then spawns as a stdio MCP subprocess.
//!
//! The binary reads the API key from its own `OLLAMA_API_KEY` environment
//! variable. Configure it as a user environment variable (see
//! configuration/installation.md) — the sandboxed extension cannot read the
//! host environment from `context_server_command`.

use zed_extension_api as zed;

/// GitHub owner/repo that hosts the releases produced by
/// .github/workflows/release.yml.
const RELEASE_BASE: &str = "https://github.com/ykodywin/ollama-mcp-zed/releases/latest/download";

struct OllamaSearchExtension;

impl zed::Extension for OllamaSearchExtension {
    fn new() -> Self {
        Self
    }

    fn context_server_command(
        &mut self,
        _id: &zed::ContextServerId,
        _project: &zed::Project,
    ) -> zed::Result<zed::Command> {
        let bin = ensure_server_binary()?;
        Ok(zed::Command {
            command: bin,
            args: vec![],
            env: vec![],
        })
    }

    fn context_server_configuration(
        &mut self,
        _id: &zed::ContextServerId,
        _project: &zed::Project,
    ) -> zed::Result<Option<zed::ContextServerConfiguration>> {
        Ok(Some(zed::ContextServerConfiguration {
            installation_instructions: include_str!("../configuration/installation.md").to_string(),
            default_settings: include_str!("../configuration/default_settings.jsonc").to_string(),
            settings_schema: include_str!("../configuration/settings_schema.json").to_string(),
        }))
    }
}

/// Downloads (once) the platform-appropriate server binary into the
/// extension's working directory and returns its absolute path.
fn ensure_server_binary() -> zed::Result<String> {
    let (os, arch) = zed::current_platform();
    let (os_s, arch_s, ext) = match (os, arch) {
        (zed::Os::Windows, zed::Architecture::X8664) => ("windows", "x86_64", ".exe"),
        (zed::Os::Windows, zed::Architecture::Aarch64) => ("windows", "aarch64", ".exe"),
        (zed::Os::Mac, zed::Architecture::X8664) => ("darwin", "x86_64", ""),
        (zed::Os::Mac, zed::Architecture::Aarch64) => ("darwin", "aarch64", ""),
        (zed::Os::Linux, zed::Architecture::X8664) => ("linux", "x86_64", ""),
        (zed::Os::Linux, zed::Architecture::Aarch64) => ("linux", "aarch64", ""),
        _ => return Err(format!("unsupported platform: {os:?} {arch:?}")),
    };

    let filename = format!("ollama-search-mcp-{os_s}-{arch_s}{ext}");

    // Download only if not already present (the work dir persists between
    // Zed sessions, so this is a one-time download).
    if !std::path::Path::new(&filename).exists() {
        let url = format!("{RELEASE_BASE}/{filename}");
        zed::download_file(&url, &filename, zed::DownloadedFileType::Uncompressed)?;
    }

    // No-op on Windows; required on macOS/Linux.
    zed::make_file_executable(&filename)?;

    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    Ok(cwd.join(&filename).to_string_lossy().into_owned())
}

zed::register_extension!(OllamaSearchExtension);