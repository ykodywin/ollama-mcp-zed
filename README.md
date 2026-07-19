# ollama-mcp-zed

An [MCP](https://modelcontextprotocol.io) server that exposes Ollama's hosted
**web search** & **web fetch** ([docs](https://docs.ollama.com/capabilities/web-search))
as the `web_search` / `web_fetch` tools, usable from Zed's Assistant.

Two pieces:

- `server/` — the MCP server itself, a small Rust stdio binary
  (`ollama-search-mcp`). This is the thing that actually talks to Ollama's API.
- `extension.toml` + `src/` — a Zed **extension** that downloads the prebuilt
  binary for your platform from GitHub Releases and launches it as a context
  server.

> Why is the server a separate binary? Zed extensions run in a WASM sandbox
> and can't run arbitrary network/HTTP code or ship executables. So the
> extension's only job is to fetch the native binary and tell Zed to spawn it.
> ([Zed: MCP extensions](https://github.com/zed-industries/zed/blob/main/docs/src/extensions/mcp-extensions.md))

---

## Prerequisites

- A free **Ollama API key**: create one at <https://ollama.com/settings/keys>.
- [Rust](https://rustup.rs) (only needed to build the server locally).

## 1. Get web search working in Zed right now (no extension, no hosting)

Build the server and point Zed at the binary directly. This needs no GitHub
release and is the fastest way to try it.

```sh
cd server
cargo build --release
# Binary: server/target/release/ollama-search-mcp.exe   (Windows)
#         server/target/release/ollama-search-mcp        (macOS/Linux)
```

Set the API key as a **user** environment variable, then **restart Zed**:

```powershell
# Windows (PowerShell) — then restart Zed
setx OLLAMA_API_KEY "ollama-xxxxxxxxxxxxxxxx"
```
```sh
# macOS / Linux — add to ~/.zshrc or ~/.bashrc, then restart Zed
export OLLAMA_API_KEY="ollama-xxxxxxxxxxxxxxxx"
```

Add the server to Zed (`settings.json` → `context_servers`):

```jsonc
{
  "context_servers": {
    "ollama-search": {
      "command": "C:/path/to/ollama-search-mcp.exe",
      "args": []
    }
  }
}
```

> `env` is inherited from Zed's environment, so `OLLAMA_API_KEY` set above
> reaches the spawned process. (You can also inline it:
> `"env": { "OLLAMA_API_KEY": "ollama-..." }`.)

Restart Zed, open the Assistant panel, enable the **ollama-search** context
server, and ask any model something current — it can now call `web_search` /
`web_fetch`.

### Test the server by hand

```sh
printf '%s\n%s\n%s\n' \
'{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}' \
'{"jsonrpc":"2.0","method":"notifications/initialized"}' \
'{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"web_search","arguments":{"query":"latest rust release"}}}' \
| OLLAMA_API_KEY=yourkey ./server/target/release/ollama-search-mcp
```

---

## 2. Use it as a Zed extension (installs the server automatically)

This is the distributable form: users install the extension and it downloads
the right binary on first use.

### Build & publish the binaries

1. Push the repo to GitHub.
2. Replace `CHANGE_ME` in `extension.toml`, `src/lib.rs` (`RELEASE_BASE`), and
   `Cargo.toml` with your `<owner>/<repo>`.
3. Tag a release:

   ```sh
   git tag v0.1.0 && git push origin v0.1.0
   ```

   The `release` workflow builds the binary for Windows (x86_64), macOS
   (x86_64, aarch64) and Linux (x86_64, aarch64) and attaches them to the
   GitHub Release. Asset names follow `ollama-search-mcp-<os>-<arch>[.exe]`,
   which is exactly what `src/lib.rs` downloads.

### Install the dev extension locally

```sh
cargo check --target wasm32-wasip1   # quick sanity check it compiles
```

In Zed → Extensions → `Install Dev Extension` → select this folder. Zed
compiles the extension to WASM and installs it. Open the Assistant, enable
**Ollama Web Search**.

The extension downloads the binary into its working directory on first use;
make sure Zed can reach `github.com`. Set `OLLAMA_API_KEY` as in step 1.

### Publish to the Zed extension registry

Follow [Zed's publishing guide](https://github.com/zed-industries/zed/blob/main/docs/src/extensions/developing-extensions.md) —
open a PR adding this repo as a submodule under `extensions/ollama-search` in
[`zed-industries/extensions`](https://github.com/zed-industries/extensions).

> Note: Zed is moving MCP-server distribution toward the official
> [MCP registry](https://registry.modelcontextprotocol.io)
> ([issue #59351](https://github.com/zed-industries/zed/issues/59351)). Consider
> listing the server there as well.

---

## Tools

### `web_search`
- `query` (string, required) — search query
- `max_results` (integer, 1–10, default 5)

Returns results with `title`, `url`, `content`.

### `web_fetch`
- `url` (string, required) — absolute URL to fetch

Returns `title`, `content`, `links`.

## Layout

```
server/                Rust MCP stdio server (native binary)
  src/main.rs
extension.toml         Zed extension manifest
src/lib.rs             Extension: download + launch the binary
configuration/         Installation instructions / settings shown in Zed
.github/workflows/     Release build for each platform
```

## License

MIT