const invoke = window.__TAURI__.core.invoke;

const elements = {
  processNowButton: document.querySelector("#process-now-button"),
  clearBatchButton: document.querySelector("#clear-batch-button"),
  pendingCount: document.querySelector("#pending-count"),
  segmentCount: document.querySelector("#segment-count"),
  clipboardStatus: document.querySelector("#clipboard-status"),
  mergedOutput: document.querySelector("#merged-output"),
  pendingList: document.querySelector("#pending-list"),
  segmentList: document.querySelector("#segment-list"),
  flashBanner: document.querySelector("#flash-banner"),
};

const context = {
  batchState: { pendingCount: 0, pendingFiles: [] },
  appState: { mergedText: "", segments: [] },
};

async function main() {
  bindEvents();
  await refreshAll();
  // Re-fetch state whenever the panel regains focus (user switches back from screenshots)
  window.addEventListener("focus", refreshAll);
}

function bindEvents() {
  elements.processNowButton.addEventListener("click", processNow);
  elements.clearBatchButton.addEventListener("click", clearBatch);
}

async function refreshAll() {
  try {
    const [batchState, appState] = await Promise.all([
      invoke("get_batch_state"),
      invoke("get_app_state"),
    ]);
    context.batchState = batchState;
    context.appState = appState;
    render();
  } catch (error) {
    flash(String(error), true);
  }
}

async function processNow() {
  setBusy(elements.processNowButton, true, "Processing...");
  try {
    await invoke("process_batch_now");
    flash("Batch processed — text copied to clipboard.");
    elements.clipboardStatus.textContent = "Copied";
    await refreshAll();
  } catch (error) {
    flash(String(error), true);
  } finally {
    setBusy(elements.processNowButton, false, "Process Now");
  }
}

async function clearBatch() {
  setBusy(elements.clearBatchButton, true, "Clearing...");
  try {
    await invoke("clear_batch");
    flash("Batch cleared.");
    await refreshAll();
  } catch (error) {
    flash(String(error), true);
  } finally {
    setBusy(elements.clearBatchButton, false, "Clear Batch");
  }
}

function render() {
  const { pendingCount, pendingFiles } = context.batchState;
  const { mergedText, segments } = context.appState;

  elements.pendingCount.textContent = String(pendingCount);
  elements.segmentCount.textContent = String(segments.length);
  elements.mergedOutput.value = mergedText || "";

  elements.processNowButton.disabled = pendingCount === 0;

  // Pending file list
  if (!pendingFiles.length) {
    elements.pendingList.innerHTML = '<p class="empty-state">No screenshots pending.</p>';
  } else {
    elements.pendingList.innerHTML = pendingFiles
      .map((filePath) => {
        const filename = filePath.split("/").pop() || filePath;
        return `
          <article class="segment-card pending-file-card">
            <div class="segment-meta">
              <span class="segment-badge pending-badge">queued</span>
              <span>${escapeHtml(filename)}</span>
            </div>
          </article>`;
      })
      .join("");
  }

  // Session timeline (processed segments)
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
          </div>
          <pre class="segment-text">${escapeHtml(segment.recognizedText)}</pre>
        </article>`
    )
    .join("");
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
