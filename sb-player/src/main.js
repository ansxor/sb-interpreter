import init, { run_interactive, stop_interactive } from "../pkg/sb_platform_wasm.js";
// SmileBASIC 3.6.0 built-in "EXAMPLE8 / Technical Demo" (TECHDEMO), decoded from the
// Azahar extdata slot TTECHDEMO (in-SB name TECHDEMO, TXT). Provenance: harvested via the
// sb-oracle extdata reader (.claude/skills/sb-oracle/tools/sb_extdata.py read_result).
import DEFAULT_CODE from "./techdemo.sb?raw";

const els = {
  run: document.getElementById("btn-run"),
  stop: document.getElementById("btn-stop"),
  load: document.getElementById("btn-load"),
  settings: document.getElementById("btn-settings"),
  closeSettings: document.getElementById("btn-close-settings"),
  clearStorage: document.getElementById("btn-clear-storage"),
  editor: document.getElementById("editor"),
  screen: document.getElementById("screen"),
  screenBottom: document.getElementById("screen-bottom"),
  screenFrame: document.getElementById("screen-frame"),
  touchOverlay: document.getElementById("touch-overlay"),
  settingsPanel: document.getElementById("settings"),
  fileInput: document.getElementById("file-input"),
  errorBanner: document.getElementById("error-banner"),
  errorMessage: document.getElementById("error-message"),
  errorClose: document.getElementById("btn-error-close"),
  scale: document.getElementById("setting-scale"),
  pixelated: document.getElementById("setting-pixelated"),
  touchOverlayToggle: document.getElementById("setting-touch-overlay"),
};

let wasmReady = false;
let running = false;

const settings = loadSettings();

function loadSettings() {
  const raw = localStorage.getItem("sb-player:settings");
  const defaults = {
    scale: "2",
    pixelated: true,
    touchOverlay: false,
  };
  if (!raw) return defaults;
  try {
    return { ...defaults, ...JSON.parse(raw) };
  } catch {
    return defaults;
  }
}

function saveSettings() {
  localStorage.setItem("sb-player:settings", JSON.stringify(settings));
}

function applySettings() {
  els.scale.value = settings.scale;
  els.pixelated.checked = settings.pixelated;
  els.touchOverlayToggle.checked = settings.touchOverlay;

  applyScale();
  applyPixelated();
  applyTouchOverlay();
}

// Both physical screens scale by the same factor so they stay 1:1 with each other.
const canvases = [els.screen, els.screenBottom];

function scaleCanvas(canvas) {
  canvas.classList.remove("fit");
  canvas.style.width = "";
  canvas.style.height = "";

  if (settings.scale === "fit") {
    canvas.classList.add("fit");
    return;
  }

  const scale = parseInt(settings.scale, 10);
  const baseWidth = canvas.width || (canvas === els.screenBottom ? 320 : 400);
  const baseHeight = canvas.height || 240;
  canvas.style.width = `${baseWidth * scale}px`;
  canvas.style.height = `${baseHeight * scale}px`;
}

function applyScale() {
  canvases.forEach(scaleCanvas);
}

function applyPixelated() {
  canvases.forEach((c) => c.classList.toggle("pixelated", settings.pixelated));
}

function applyTouchOverlay() {
  els.touchOverlay.classList.toggle("hidden", !settings.touchOverlay);
}

function setRunning(value) {
  running = value;
  els.run.disabled = running;
  els.stop.disabled = !running;
}

function showError(message) {
  els.errorMessage.textContent = String(message);
  els.errorBanner.classList.remove("hidden");
  setRunning(false);
  console.error(message);
}

function hideError() {
  els.errorBanner.classList.add("hidden");
}

async function boot() {
  await init();
  wasmReady = true;
  els.editor.value = localStorage.getItem("sb-player:draft") || DEFAULT_CODE;
  applySettings();
  setRunning(false);
}

function doRun() {
  if (!wasmReady || running) return;
  hideError();
  const src = els.editor.value;
  localStorage.setItem("sb-player:draft", src);

  try {
    setRunning(true);
    run_interactive("screen", "screen-bottom", src, showError);
  } catch (e) {
    console.error(e);
    showError("Failed to run program: " + e);
  }
}

function doStop() {
  if (!running) return;
  stop_interactive();
  setRunning(false);
}

function doLoad() {
  els.fileInput.click();
}

function onFileSelected(event) {
  const file = event.target.files[0];
  if (!file) return;
  hideError();
  const reader = new FileReader();
  reader.onload = () => {
    els.editor.value = String(reader.result);
    localStorage.setItem("sb-player:draft", els.editor.value);
  };
  reader.readAsText(file);
  event.target.value = "";
}

function toggleSettings() {
  els.settingsPanel.classList.toggle("hidden");
}

async function clearIndexedDB() {
  try {
    const databases = await indexedDB.databases?.();
    if (databases) {
      for (const db of databases) {
        if (db.name) indexedDB.deleteDatabase(db.name);
      }
    } else {
      indexedDB.deleteDatabase("smilebasic");
    }
    alert("IndexedDB storage cleared.");
  } catch (e) {
    console.error(e);
    alert("Could not clear storage: " + e);
  }
}

els.run.addEventListener("click", doRun);
els.stop.addEventListener("click", doStop);
els.load.addEventListener("click", doLoad);
els.settings.addEventListener("click", toggleSettings);
els.closeSettings.addEventListener("click", toggleSettings);
els.fileInput.addEventListener("change", onFileSelected);
els.errorClose.addEventListener("click", hideError);
els.clearStorage.addEventListener("click", clearIndexedDB);

els.editor.addEventListener("keydown", (e) => {
  if (e.ctrlKey && e.key === "Enter") {
    e.preventDefault();
    doRun();
  }
  if (e.key === "Tab") {
    e.preventDefault();
    const start = els.editor.selectionStart;
    const end = els.editor.selectionEnd;
    els.editor.value = els.editor.value.substring(0, start) + "  " + els.editor.value.substring(end);
    els.editor.selectionStart = els.editor.selectionEnd = start + 2;
  }
});

els.scale.addEventListener("change", () => {
  settings.scale = els.scale.value;
  saveSettings();
  applyScale();
});

els.pixelated.addEventListener("change", () => {
  settings.pixelated = els.pixelated.checked;
  saveSettings();
  applyPixelated();
});

els.touchOverlayToggle.addEventListener("change", () => {
  settings.touchOverlay = els.touchOverlayToggle.checked;
  saveSettings();
  applyTouchOverlay();
});

boot();

// When the wasm host changes a canvas's intrinsic resolution (e.g. after an XSCREEN
// switch), keep the CSS display size in sync with the selected scale. Watches both screens
// since either can resize independently across modes.
const canvasAttrObserver = new MutationObserver(() => {
  if (settings.scale !== "fit") {
    applyScale();
  }
});
for (const canvas of canvases) {
  canvasAttrObserver.observe(canvas, {
    attributes: true,
    attributeFilter: ["width", "height"],
  });
}
