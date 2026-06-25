import init, { run_interactive, stop_interactive } from "../pkg/sb_platform_wasm.js";

const DEFAULT_CODE = `PRINT "hello from sb-player!"
PRINT "edit this code, or load a file"
`;

const els = {
  run: document.getElementById("btn-run"),
  stop: document.getElementById("btn-stop"),
  load: document.getElementById("btn-load"),
  settings: document.getElementById("btn-settings"),
  closeSettings: document.getElementById("btn-close-settings"),
  clearStorage: document.getElementById("btn-clear-storage"),
  editor: document.getElementById("editor"),
  screen: document.getElementById("screen"),
  screenFrame: document.getElementById("screen-frame"),
  touchOverlay: document.getElementById("touch-overlay"),
  settingsPanel: document.getElementById("settings"),
  fileInput: document.getElementById("file-input"),
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

function applyScale() {
  const canvas = els.screen;
  canvas.classList.remove("fit");
  canvas.style.width = "";
  canvas.style.height = "";

  if (settings.scale === "fit") {
    canvas.classList.add("fit");
    return;
  }

  const scale = parseInt(settings.scale, 10);
  const baseWidth = canvas.width || 400;
  const baseHeight = canvas.height || 240;
  canvas.style.width = `${baseWidth * scale}px`;
  canvas.style.height = `${baseHeight * scale}px`;
}

function applyPixelated() {
  els.screen.classList.toggle("pixelated", settings.pixelated);
}

function applyTouchOverlay() {
  els.touchOverlay.classList.toggle("hidden", !settings.touchOverlay);
}

function setRunning(value) {
  running = value;
  els.run.disabled = running;
  els.stop.disabled = !running;
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
  const src = els.editor.value;
  localStorage.setItem("sb-player:draft", src);

  try {
    setRunning(true);
    run_interactive("screen", src);
  } catch (e) {
    console.error(e);
    alert("Failed to run program: " + e);
    setRunning(false);
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

// When the wasm host changes the canvas's intrinsic resolution (e.g. after an XSCREEN
// switch), keep the CSS display size in sync with the selected scale.
const canvasAttrObserver = new MutationObserver(() => {
  if (settings.scale !== "fit") {
    applyScale();
  }
});
canvasAttrObserver.observe(els.screen, {
  attributes: true,
  attributeFilter: ["width", "height"],
});
