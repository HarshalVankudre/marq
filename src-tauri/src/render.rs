use std::path::Path;
use std::sync::OnceLock;

use comrak::plugins::syntect::{SyntectAdapter, SyntectAdapterBuilder};
use comrak::options::Plugins;
use comrak::{markdown_to_html_with_plugins, Options};
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct RenderResult {
    pub title: String,
    pub path: String,
    pub dir: String,
    pub html: String,
}

#[derive(Serialize)]
pub struct HighlightCss {
    pub light: String,
    pub dark: String,
}

const MAX_FILE_BYTES: usize = 20 * 1024 * 1024;

fn adapter() -> &'static SyntectAdapter {
    static ADAPTER: OnceLock<SyntectAdapter> = OnceLock::new();
    ADAPTER.get_or_init(|| {
        SyntectAdapterBuilder::new()
            .css_with_class_prefix("syntect-")
            .build()
    })
}

pub fn render_path(path: &Path) -> Result<RenderResult, String> {
    let raw = read_text(path)?;
    let html = md_to_html(&raw);
    Ok(RenderResult {
        title: path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into()),
        path: path.to_string_lossy().into_owned(),
        dir: path
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        html,
    })
}

pub fn md_to_html(md: &str) -> String {
    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(adapter());
    let dirty = markdown_to_html_with_plugins(md, &options(), &plugins);
    sanitize(&dirty)
}

fn options() -> Options<'static> {
    let mut o = Options::default();
    o.extension.strikethrough = true;
    o.extension.tagfilter = false; // ammonia handles dangerous HTML
    o.extension.table = true;
    o.extension.autolink = true;
    o.extension.tasklist = true;
    o.extension.footnotes = true;
    o.extension.description_lists = true;
    o.extension.multiline_block_quotes = true;
    o.extension.alerts = true; // GitHub-style [!NOTE] etc.
    o.extension.shortcodes = true; // :tada: -> 🎉
    o.extension.header_id_prefix = Some(String::new());
    o.extension.front_matter_delimiter = Some("---".into());
    o.parse.smart = true;
    o.parse.relaxed_tasklist_matching = true;
    o.render.r#unsafe = true; // raw HTML passes through to the sanitizer below
    o.render.tasklist_classes = true;
    o
}

/// GitHub-parity sanitization: raw HTML in documents is allowed but scripts,
/// event handlers, iframes etc. are stripped.
fn sanitize(html: &str) -> String {
    use std::collections::HashSet;
    let mut b = ammonia::Builder::default();
    b.add_tags(["input", "section", "details", "summary"])
        .add_generic_attributes(["class", "id", "align", "dir"])
        .add_tag_attributes("input", ["type", "checked", "disabled"])
        .add_tag_attributes("img", ["src", "alt", "title", "width", "height", "loading"])
        .add_tag_attributes("a", ["href", "title", "target"])
        .add_tag_attributes("th", ["style", "colspan", "rowspan", "scope"])
        .add_tag_attributes("td", ["style", "colspan", "rowspan"])
        .add_tag_attributes("ol", ["start", "type"])
        .add_tag_attributes("li", ["value"])
        .add_tag_attributes("details", ["open"])
        .url_schemes(HashSet::from(["http", "https", "mailto", "data", "asset"]))
        .url_relative(ammonia::UrlRelative::PassThrough)
        .link_rel(Some("noopener noreferrer"));
    b.clean(html).to_string()
}

fn read_text(path: &Path) -> Result<String, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(format!(
            "{} is larger than 20 MB — too big for a Markdown viewer.",
            path.display()
        ));
    }
    Ok(decode(bytes))
}

/// Handles the encodings Windows editors actually produce: UTF-8 (with or
/// without BOM) and UTF-16 LE/BE (Notepad's "Unicode").
fn decode(b: Vec<u8>) -> String {
    if b.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> = b[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else if b.starts_with(&[0xFE, 0xFF]) {
        let units: Vec<u16> = b[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else if b.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&b[3..]).into_owned()
    } else {
        String::from_utf8_lossy(&b).into_owned()
    }
}

pub fn highlight_css() -> HighlightCss {
    use syntect::highlighting::ThemeSet;
    use syntect::html::{css_for_theme_with_class_style, ClassStyle};
    let ts = ThemeSet::load_defaults();
    // Must match the class style comrak's SyntectAdapterBuilder::css() emits.
    let style = ClassStyle::SpacedPrefixed { prefix: "syntect-" };
    HighlightCss {
        light: css_for_theme_with_class_style(&ts.themes["InspiredGitHub"], style)
            .unwrap_or_default(),
        dark: css_for_theme_with_class_style(&ts.themes["base16-eighties.dark"], style)
            .unwrap_or_default(),
    }
}
