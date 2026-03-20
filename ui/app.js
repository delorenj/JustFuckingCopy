const invoke = window.__TAURI__.core.invoke;

const elements = {
  captureButton: document.querySelector("#capture-button"),
  copyButton: document.querySelector("#copy-button"),
  undoButton: document.querySelector("#undo-button"),
  resetButton: document.querySelector("#reset-button"),
  clearSelectionButton: document.querySelector("#clear-selection-button"),
  commitSelectionButton: document.querySelector("#commit-selection-button"),
  snapshotStatus: document.querySelector("#snapshot-status"),
  segmentCount: document.querySelector("#segment-count"),
  clipboardStatus: document.querySelector("#clipboard-status"),
  mergedOutput: document.querySelector("#merged-output"),
  segmentList: document.querySelector("#segment-list"),
  selectionStage: document.querySelector("#selection-stage"),
  selectionPlaceholder: document.querySelector("#selection-placeholder"),
  canvas: document.querySelector("#snapshot-canvas"),
  flashBanner: document.querySelector("#flash-banner"),
};

const context = {
  snapshot: null,
  image: null,
  selection: null,
  isDragging: false,
  dragStart: null,
  appState: {
    mergedText: "",
    segments: [],
    currentSnapshot: null,
  },
};

const canvasContext = elements.canvas.getContext("2d");

async function main() {
  bindEvents();
  await refreshState();
}

function bindEvents() {
  elements.captureButton.addEventListener("click", captureSnapshot);
  elements.copyButton.addEventListener("click", copyMergedText);
  elements.undoButton.addEventListener("click", undoLastSegment);
  elements.resetButton.addEventListener("click", resetSession);
  elements.clearSelectionButton.addEventListener("click", clearSelection);
  elements.commitSelectionButton.addEventListener("click", commitSelection);

  elements.canvas.addEventListener("pointerdown", onPointerDown);
  elements.canvas.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp);
}

async function refreshState() {
  context.appState = await invoke("get_app_state");
  render();
}

async function captureSnapshot() {
  setBusy(elements.captureButton, true, "Capturing...");

  try {
    const snapshot = await invoke("capture_snapshot");
    context.snapshot = snapshot;
    context.selection = null;
    await loadSnapshotImage(snapshot.dataUrl);
    context.appState.currentSnapshot = {
      id: snapshot.id,
      width: snapshot.width,
      height: snapshot.height,
    };
    renderCanvas();
    render();
    flash(`Snapshot ${snapshot.id} ready. Drag a marquee and commit it.`);
  } catch (error) {
    flash(String(error), true);
  } finally {
    setBusy(elements.captureButton, false, "Capture Snapshot");
  }
}

async function commitSelection() {
  if (!context.snapshot || !context.selection) {
    flash("Draw a selection box before committing.", true);
    return;
  }

  setBusy(elements.commitSelectionButton, true, "Running OCR...");

  try {
    context.appState = await invoke("commit_selection", {
      request: {
        snapshotId: context.snapshot.id,
        selection: context.selection,
      },
    });
    context.selection = null;
    renderCanvas();
    render();
    flash("Selection committed and merged.");
  } catch (error) {
    flash(String(error), true);
  } finally {
    setBusy(elements.commitSelectionButton, false, "Commit Selection");
  }
}

async function copyMergedText() {
  setBusy(elements.copyButton, true, "Copying...");

  try {
    const mergedText = await invoke("copy_merged_text");
    context.appState.mergedText = mergedText;
    elements.clipboardStatus.textContent = "Copied";
    flash("Merged text copied to the native clipboard.");
  } catch (error) {
    flash(String(error), true);
  } finally {
    setBusy(elements.copyButton, false, "Copy Merged Text");
  }
}

async function undoLastSegment() {
  try {
    context.appState = await invoke("undo_last_segment");
    render();
    flash("Removed the latest OCR slice.");
  } catch (error) {
    flash(String(error), true);
  }
}

async function resetSession() {
  try {
    context.appState = await invoke("reset_session");
    context.snapshot = null;
    context.image = null;
    context.selection = null;
    renderCanvas();
    render();
    flash("Session reset.");
  } catch (error) {
    flash(String(error), true);
  }
}

function clearSelection() {
  context.selection = null;
  renderCanvas();
  render();
}

async function loadSnapshotImage(dataUrl) {
  const image = new Image();
  image.src = dataUrl;

  await new Promise((resolve, reject) => {
    image.onload = resolve;
    image.onerror = reject;
  });

  context.image = image;
  elements.canvas.width = image.naturalWidth || image.width;
  elements.canvas.height = image.naturalHeight || image.height;
  elements.selectionStage.classList.remove("empty");
}

function render() {
  const { segments, mergedText, currentSnapshot } = context.appState;

  elements.snapshotStatus.textContent = currentSnapshot
    ? `${currentSnapshot.width} x ${currentSnapshot.height}`
    : "None yet";
  elements.segmentCount.textContent = String(segments.length);
  elements.mergedOutput.value = mergedText || "";
  elements.selectionPlaceholder.hidden = Boolean(context.image);
  elements.commitSelectionButton.disabled = !context.selection;
  elements.clearSelectionButton.disabled = !context.selection;
  elements.undoButton.disabled = segments.length === 0;
  elements.copyButton.disabled = !mergedText.trim();

  if (!segments.length) {
    elements.segmentList.innerHTML = '<p class="empty-state">No OCR slices yet.</p>';
    return;
  }

  elements.segmentList.innerHTML = segments
    .map(
      (segment) => `
        <article class="segment-card">
          <div class="segment-meta">
            <span class="segment-badge">#${segment.order}</span>
            <span>${segment.mergeStrategy}</span>
            <span>${segment.overlapLines} overlap lines</span>
            <span>${segment.selection.width} x ${segment.selection.height}</span>
          </div>
          <pre class="segment-text">${escapeHtml(segment.recognizedText)}</pre>
        </article>
      `
    )
    .join("");
}

function renderCanvas() {
  const canvas = elements.canvas;
  const image = context.image;

  canvasContext.clearRect(0, 0, canvas.width, canvas.height);

  if (!image) {
    elements.selectionStage.classList.add("empty");
    return;
  }

  elements.selectionStage.classList.remove("empty");
  canvasContext.drawImage(image, 0, 0, canvas.width, canvas.height);

  if (context.selection) {
    const { x, y, width, height } = context.selection;
    canvasContext.fillStyle = "rgba(217, 101, 43, 0.18)";
    canvasContext.strokeStyle = "#d9652b";
    canvasContext.lineWidth = 2;
    canvasContext.fillRect(x, y, width, height);
    canvasContext.strokeRect(x, y, width, height);
  }
}

function onPointerDown(event) {
  if (!context.image) {
    return;
  }

  context.isDragging = true;
  context.dragStart = toCanvasPoint(event);
  context.selection = {
    x: context.dragStart.x,
    y: context.dragStart.y,
    width: 0,
    height: 0,
  };
  renderCanvas();
  render();
}

function onPointerMove(event) {
  if (!context.isDragging || !context.dragStart) {
    return;
  }

  const current = toCanvasPoint(event);
  context.selection = normalizedRect(context.dragStart, current);
  renderCanvas();
  render();
}

function onPointerUp() {
  if (!context.isDragging) {
    return;
  }

  context.isDragging = false;
  context.dragStart = null;

  if (context.selection && (context.selection.width < 8 || context.selection.height < 8)) {
    context.selection = null;
  }

  renderCanvas();
  render();
}

function toCanvasPoint(event) {
  const rect = elements.canvas.getBoundingClientRect();
  const scaleX = elements.canvas.width / rect.width;
  const scaleY = elements.canvas.height / rect.height;
  const x = Math.max(0, Math.min(elements.canvas.width, Math.round((event.clientX - rect.left) * scaleX)));
  const y = Math.max(0, Math.min(elements.canvas.height, Math.round((event.clientY - rect.top) * scaleY)));
  return { x, y };
}

function normalizedRect(start, end) {
  const x = Math.min(start.x, end.x);
  const y = Math.min(start.y, end.y);
  return {
    x,
    y,
    width: Math.abs(end.x - start.x),
    height: Math.abs(end.y - start.y),
  };
}

function setBusy(button, busy, label) {
  button.disabled = busy;
  button.textContent = label;
}

let flashTimer = null;
function flash(message, isError = false) {
  window.clearTimeout(flashTimer);
  elements.flashBanner.hidden = false;
  elements.flashBanner.textContent = message;
  elements.flashBanner.style.background = isError
    ? "rgba(122, 22, 22, 0.92)"
    : "rgba(31, 28, 23, 0.9)";

  flashTimer = window.setTimeout(() => {
    elements.flashBanner.hidden = true;
  }, 3200);
}

function escapeHtml(text) {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

main().catch((error) => {
  flash(String(error), true);
});
