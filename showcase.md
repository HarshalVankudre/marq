---
title: Marq feature tour
author: you
---

# Marq showcase 🚀

Everything below renders **live** — edit this file in any editor and watch Marq update instantly. Emoji shortcodes work too: :sparkles: :rocket: :tada: :crab: :fire:

Unicode emoji render natively: 🎨 📚 ✅ 🧠 🌈 👩‍💻

## Text styling

**Bold**, *italic*, ***both***, ~~strikethrough~~, `inline code`, and <mark>highlighted</mark> text.
Smart punctuation works -- like this em-dash, "curly quotes", and ellipses...

Here's a [link to the CommonMark spec](https://commonmark.org), an autolink https://github.com, and a footnote reference[^1].

[^1]: Footnotes land at the bottom of the document, styled subtly.

## Code with real syntax highlighting

```rust
use comrak::{markdown_to_html_with_plugins, Options};

/// Marq renders Markdown with comrak + syntect — all in Rust.
fn main() {
    let opts = Options::default();
    let html = markdown_to_html_with_plugins("# Hello 🌍", &opts, &plugins());
    println!("{html}");
}
```

```python
from pathlib import Path

def render(path: Path) -> str:
    """Python, too."""
    text = path.read_text(encoding="utf-8")
    return f"<article>{text}</article>"
```

```json
{ "name": "marq", "fast": true, "memory_mb": 40, "stack": ["rust", "tauri", "webview2"] }
```

## Tables

| Feature            | Status | Notes                          |
| :----------------- | :----: | -----------------------------: |
| GitHub styling     |   ✅   | light + dark themes            |
| Emoji shortcodes   |   ✅   | `:tada:` → :tada:              |
| Syntax highlight   |   ✅   | via syntect, zero JS           |
| PDF export         |   ✅   | Ctrl+E, prints backgrounds     |
| Live reload        |   ✅   | file watcher, 200 ms debounce  |

## Task lists

- [x] Runs in the background (tray)
- [x] Opens .md files instantly
- [x] Exports beautiful PDFs
- [ ] World domination

## Alerts

> [!NOTE]
> Marq supports GitHub-style alerts — all five of them.

> [!TIP]
> Press <kbd>Ctrl</kbd>+<kbd>E</kbd> to export this document as a PDF.

> [!IMPORTANT]
> Closing the window keeps Marq in the tray. Quit from the tray menu.

> [!WARNING]
> Files over 20 MB are refused — that's a book, not a note.

> [!CAUTION]
> Regular expressions and markdown parsers are natural enemies.

## Blockquotes & details

> "The best tool is the one you never notice."
> — someone, probably

<details>
<summary>Click to expand raw-HTML details</summary>

Raw HTML works, but it's sanitized — scripts and iframes are stripped, GitHub-style.

</details>

## Local images

![demo](assets/demo.png)

---

That's the tour. Open your own files now — right-click any `.md` → **Open with** → **Marq**. ✌️
