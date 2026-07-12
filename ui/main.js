"use strict";

const { invoke, convertFileSrc } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (s) => document.querySelector(s);
const docEl = $("#doc");
const welcomeEl = $("#welcome");
const fileLabel = $("#file-label");
const docMeta = $("#doc-meta");
const scrollEl = $("#scroll");
const syntectStyle = $("#syntect");

let cur = { path: null, dir: null };
let hlCss = { light: "", dark: "" };
let themePref = localStorage.getItem("theme") || "auto";
let zoom = parseFloat(localStorage.getItem("zoom") || "1") || 1;

// ---------------------------------------------------------------- theme

const ICONS = {
  auto: '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true"><circle cx="8" cy="8" r="6.2" stroke="currentColor" stroke-width="1.4"/><path d="M8 1.8A6.2 6.2 0 0 1 8 14.2Z" fill="currentColor"/></svg>',
  light:
    '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true"><circle cx="8" cy="8" r="3.4" stroke="currentColor" stroke-width="1.4"/><path d="M8 .8v2M8 13.2v2M15.2 8h-2M2.8 8h-2M13.1 2.9l-1.4 1.4M4.3 11.7l-1.4 1.4M13.1 13.1l-1.4-1.4M4.3 4.3 2.9 2.9" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>',
  dark: '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M13.8 9.6A6.2 6.2 0 1 1 6.4 2.2a5 5 0 0 0 7.4 7.4Z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/></svg>',
};

const sysDark = matchMedia("(prefers-color-scheme: dark)");
const effectiveTheme = () =>
  themePref === "auto" ? (sysDark.matches ? "dark" : "light") : themePref;

function applyTheme() {
  const t = effectiveTheme();
  document.documentElement.dataset.theme = t;
  syntectStyle.textContent = hlCss[t] || "";
  const btn = $("#btn-theme");
  btn.innerHTML = ICONS[themePref] || ICONS.auto;
  btn.title = `Theme: ${themePref} (click to cycle)`;
}
sysDark.addEventListener("change", () => {
  if (themePref === "auto") applyTheme();
});

function applyZoom() {
  docEl.style.zoom = zoom;
  localStorage.setItem("zoom", String(zoom));
}

// ---------------------------------------------------------------- toast

let toastTimer;
function toast(msg, ms = 3600) {
  const t = $("#toast");
  t.textContent = msg;
  t.hidden = false;
  requestAnimationFrame(() => t.classList.add("show"));
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    t.classList.remove("show");
    setTimeout(() => (t.hidden = true), 250);
  }, ms);
}

// ---------------------------------------------------------------- document

const isAbsWin = (p) => /^([a-zA-Z]:[\\/]|\\\\)/.test(p);
const hasScheme = (s) => /^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(s);

function resolveLocal(src) {
  let raw = src.split("#")[0].split("?")[0];
  try {
    raw = decodeURIComponent(raw);
  } catch {}
  raw = raw.replace(/\//g, "\\").replace(/^\.\\/, "");
  if (isAbsWin(raw)) return raw;
  if (!cur.dir) return null;
  return cur.dir.replace(/[\\/]+$/, "") + "\\" + raw;
}

function updateMeta() {
  const text = docEl.innerText || "";
  const words = text.trim() ? text.trim().split(/\s+/).length : 0;
  const mins = Math.max(1, Math.round(words / 220));
  docMeta.textContent = words
    ? `${words.toLocaleString()} words · ${mins} min read`
    : "";
}

// The WebView occasionally applies a stray native scroll while the window is
// first shown (async image/font reflow). Pin a fresh document to the top for a
// moment; any real user input disarms the pin immediately.
let pinDisarm = null;
function pinScrollTop() {
  if (pinDisarm) pinDisarm();
  let corrections = 0;
  const onScroll = () => {
    if (scrollEl.scrollTop !== 0 && corrections < 4) {
      corrections++;
      scrollEl.scrollTop = 0;
    }
  };
  const onUser = () => disarm();
  const events = ["wheel", "keydown", "pointerdown", "touchstart"];
  const disarm = () => {
    scrollEl.removeEventListener("scroll", onScroll);
    for (const ev of events) window.removeEventListener(ev, onUser, true);
    clearTimeout(timer);
    pinDisarm = null;
  };
  scrollEl.addEventListener("scroll", onScroll);
  for (const ev of events) window.addEventListener(ev, onUser, true);
  const timer = setTimeout(disarm, 1600);
  pinDisarm = disarm;
}

function setDoc(d) {
  const samePath = cur.path === d.path;
  const prevScroll = scrollEl.scrollTop;
  cur = { path: d.path, dir: d.dir };
  fileLabel.textContent = d.title;
  fileLabel.title = d.path;
  document.title = `${d.title} — Marq`;
  document.body.classList.add("has-doc");
  docEl.innerHTML = d.html;
  welcomeEl.hidden = true;
  docEl.hidden = false;
  postProcess();
  updateMeta();
  const target = samePath ? prevScroll : 0;
  scrollEl.scrollTop = target;
  requestAnimationFrame(() => {
    scrollEl.scrollTop = target;
  });
  if (!samePath) pinScrollTop();
  signalRendered();
}

function postProcess() {
  // Local images/media → asset protocol URLs the webview can load.
  for (const el of docEl.querySelectorAll("img[src], video[src], source[src]")) {
    const src = el.getAttribute("src");
    if (!src || hasScheme(src)) continue;
    const p = resolveLocal(src);
    if (p) el.setAttribute("src", convertFileSrc(p));
  }
  for (const pre of docEl.querySelectorAll("pre")) {
    // Language chip from the code element's language-* class.
    const code = pre.querySelector("code");
    const m = code && code.className.match(/language-([\w#+-]+)/);
    if (m) pre.dataset.lang = m[1];
    // Copy button.
    if (pre.querySelector(".copy-btn")) continue;
    const btn = document.createElement("button");
    btn.className = "copy-btn";
    btn.type = "button";
    btn.textContent = "Copy";
    btn.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(code ? code.innerText : pre.innerText);
        btn.textContent = "Copied";
      } catch {
        btn.textContent = "Failed";
      }
      setTimeout(() => (btn.textContent = "Copy"), 1400);
    });
    pre.appendChild(btn);
  }
}

// Tell the backend the doc has painted (drives headless --pdf export).
async function signalRendered() {
  const imgs = [...docEl.querySelectorAll("img")];
  await Promise.race([
    Promise.allSettled(imgs.map((i) => (i.decode ? i.decode().catch(() => {}) : 0))),
    new Promise((r) => setTimeout(r, 2500)),
  ]);
  requestAnimationFrame(() =>
    requestAnimationFrame(() => invoke("mark_rendered").catch(() => {}))
  );
}

// Link handling: anchors scroll, web links open in the browser,
// relative .md links open in Marq, other local files open in their app.
document.addEventListener("click", (e) => {
  const a = e.target.closest("a[href]");
  if (!a || !docEl.contains(a)) return;
  e.preventDefault();
  const href = a.getAttribute("href");
  if (!href) return;
  if (href.startsWith("#")) {
    let id = href.slice(1);
    try {
      id = decodeURIComponent(id);
    } catch {}
    const target = document.getElementById(id) || document.getElementsByName(id)[0];
    if (target) target.scrollIntoView({ behavior: "smooth", block: "start" });
  } else if (/^(https?|mailto):/i.test(href)) {
    invoke("open_external", { url: href }).catch((err) => toast(String(err)));
  } else if (!hasScheme(href)) {
    invoke("open_path", { path: href.split("#")[0], base: cur.dir }).catch((err) =>
      toast(String(err))
    );
  }
});

// ---------------------------------------------------------------- export

async function exportPdf() {
  const btn = $("#btn-pdf");
  if (btn.disabled) return;
  if (!cur.path) {
    toast("Open a Markdown document first");
    return;
  }
  btn.disabled = true;
  const prevPref = themePref;
  themePref = "light"; // PDFs export in the light theme — better on paper
  applyTheme();
  document.body.classList.add("exporting");
  try {
    const saved = await invoke("export_pdf");
    if (saved) toast(`Saved ${saved}`);
  } catch (err) {
    toast(String(err));
  } finally {
    document.body.classList.remove("exporting");
    themePref = prevPref;
    applyTheme();
    btn.disabled = false;
  }
}

// ---------------------------------------------------------------- chrome

$("#btn-pdf").addEventListener("click", exportPdf);
$("#btn-theme").addEventListener("click", () => {
  themePref = themePref === "auto" ? "light" : themePref === "light" ? "dark" : "auto";
  localStorage.setItem("theme", themePref);
  applyTheme();
});
$("#btn-open").addEventListener("click", () => invoke("open_file_dialog"));
$("#btn-default").addEventListener("click", async () => {
  try {
    await invoke("set_default_app");
    toast("In the Settings page that opened, set .md to Marq");
  } catch (err) {
    toast(String(err));
  }
});

document.addEventListener("keydown", (e) => {
  if (!e.ctrlKey || e.altKey) return;
  const k = e.key.toLowerCase();
  if (k === "e") {
    e.preventDefault();
    exportPdf();
  } else if (k === "o") {
    e.preventDefault();
    invoke("open_file_dialog");
  } else if (e.key === "=" || e.key === "+") {
    e.preventDefault();
    zoom = Math.min(3, Math.round((zoom + 0.1) * 10) / 10);
    applyZoom();
  } else if (e.key === "-") {
    e.preventDefault();
    zoom = Math.max(0.5, Math.round((zoom - 0.1) * 10) / 10);
    applyZoom();
  } else if (e.key === "0") {
    e.preventDefault();
    zoom = 1;
    applyZoom();
  }
});

window.addEventListener(
  "wheel",
  (e) => {
    if (!e.ctrlKey) return;
    e.preventDefault();
    zoom = Math.min(3, Math.max(0.5, Math.round((zoom + (e.deltaY < 0 ? 0.1 : -0.1)) * 10) / 10));
    applyZoom();
  },
  { passive: false }
);

// Drag & drop (native paths come through Tauri, not the DOM).
try {
  const win = window.__TAURI__.webviewWindow.getCurrentWebviewWindow();
  win.onDragDropEvent((ev) => {
    const t = ev.payload.type;
    if (t === "enter" || t === "over") $("#drop-overlay").hidden = false;
    else $("#drop-overlay").hidden = true;
    if (t === "drop") {
      $("#drop-overlay").hidden = true;
      const p = ev.payload.paths && ev.payload.paths[0];
      if (p) invoke("open_path", { path: p, base: null }).catch((err) => toast(String(err)));
    }
  });
} catch (err) {
  console.warn("drag-drop unavailable", err);
}

// ---------------------------------------------------------------- init

(async function init() {
  applyZoom();
  try {
    hlCss = await invoke("get_highlight_css");
  } catch {}
  applyTheme();
  await listen("doc", (ev) => setDoc(ev.payload));
  await listen("toast", (ev) => toast(String(ev.payload)));
  try {
    const d = await invoke("get_current_doc");
    if (d) setDoc(d);
  } catch {}
})();
