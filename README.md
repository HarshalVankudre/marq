# Marq

**Markdown, beautifully typeset.**

Marq is an instant Markdown reader for Windows. It waits in your system tray, opens `.md` files the moment you click them, renders them like a professionally typeset document — serif typography, syntax-highlighted code, GitHub-style alerts, emoji, tables, task lists — and exports print-quality PDFs with one keystroke.

![Marq](assets/demo.png)

## Why it feels instant

Most markdown apps launch an entire browser runtime per open. Marq stays resident:

- A single ~10 MB native binary (Rust + Tauri 2) sits in the tray.
- Clicking a `.md` file forwards the path to the running instance — the window is on screen in milliseconds.
- When you close the window, the renderer is suspended and its memory target dropped; after 5 idle minutes the webview is torn down entirely, leaving only the tiny Rust process.
- Markdown is parsed and highlighted **in Rust** (comrak + syntect) — no JavaScript frameworks, no bundler, no Node runtime.

## Features

- **Typeset rendering** — Sitka serif body, editorial alerts (`[!NOTE]` … `[!CAUTION]`), horizontal-rule asterisms, book-style tables, custom task-list checks, footnotes, `<details>`, definition lists.
- **Full GFM + extras** — tables, strikethrough, autolinks, task lists, footnotes, emoji shortcodes (`:tada:` → 🎉), smart punctuation, YAML front-matter hidden automatically.
- **Real syntax highlighting** — syntect (Sublime grammars) with light/dark themes, language chips, hover copy button.
- **PDF export** — `Ctrl+E` prints an A4 PDF through WebView2 with backgrounds, embedded fonts, and no browser headers. Also scriptable: `Marq.exe doc.md --pdf out.pdf`.
- **Live reload** — edits from any editor re-render in ~200 ms, scroll position preserved.
- **Light / dark / auto themes**, zoom (`Ctrl` `+`/`−`/`0` or `Ctrl`+wheel), word count & reading time, drag-and-drop, local images, relative `.md` links open in Marq.
- **Sanitized HTML** — raw HTML in documents renders, but scripts/iframes are stripped (GitHub-parity via ammonia).
- **Encodings** — UTF-8 (±BOM) and UTF-16 LE/BE, so Notepad files just work.

## Install

Download `Marq_1.0.0_x64-setup.exe` from the latest release and run it (per-user, no admin). The installer registers Marq with Windows and offers to open the default-apps page.

**Making Marq the default `.md` app** (Windows requires one click from you):
right-click any `.md` file → *Open with* → *Choose another app* → **Marq** → *Always*. Or use the tray menu → *Make default for .md files*.

On first run Marq enables *Start with Windows* (tray, hidden) so files open instantly after login — toggle it from the tray menu.

## Build from source

```powershell
# prerequisites: Rust (MSVC toolchain), WebView2 runtime (ships with Windows 11)
cd src-tauri
cargo build --release            # → target/release/marq.exe (portable)
npx --yes @tauri-apps/cli@^2 build   # → NSIS installer in target/release/bundle/nsis
```

`tools/gen-icon.ps1` regenerates the icon set. `install.ps1` / `uninstall.ps1` are manual dev installs if you don't want the installer.

## CLI

```
Marq.exe <file.md>              open a document
Marq.exe <file.md> --pdf <out>  headless PDF export (exits when done)
Marq.exe --hidden               start parked in the tray (autostart uses this)
Marq.exe --render-html <in> <out>  dump the rendered, sanitized HTML
```

## Stack

Rust · Tauri 2 · comrak (CommonMark/GFM) · syntect (highlighting) · ammonia (sanitizer) · notify (live reload) · WebView2 (rendering + PDF) · zero JS dependencies.
