# Ollama Web Search

This context server exposes Ollama's hosted **web search** and **web fetch** as
MCP tools (`web_search`, `web_fetch`) you can use from Zed's Assistant panel.

## 1. Get an API key

Create a free key at <https://ollama.com/settings/keys>.

## 2. Set the API key as an environment variable

The server reads the key from the `OLLAMA_API_KEY` environment variable, which
the extension passes through to the spawned process. Set it as a **user**
environment variable and **restart Zed** afterwards.

- **Windows** (PowerShell):
  ```powershell
  setx OLLAMA_API_KEY "ollama-xxxxxxxxxxxxxxxx"
  ```
- **macOS / Linux** (add to `~/.zshrc` / `~/.bashrc`):
  ```sh
  export OLLAMA_API_KEY="ollama-xxxxxxxxxxxxxxxx"
  ```

## 3. Enable the context server

On first use the extension downloads the prebuilt binary for your platform;
make sure Zed can reach `github.com` (or replace the URL in `src/lib.rs` with
your own release host).

In Zed's Assistant, enable the **Ollama Web Search** context server. The
`web_search` and `web_fetch` tools become available to any model you're
chatting with.