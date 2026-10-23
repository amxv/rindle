const status = document.getElementById("status");
const sync = document.getElementById("sync");
const browser = document.getElementById("browser");

async function refresh() {
  const data = await chrome.storage.local.get(["rindleLastSuccess", "rindleLastError", "rindleLastCount"]);
  if (data.rindleLastError) status.textContent = `Connection issue: ${data.rindleLastError}`;
  else if (data.rindleLastSuccess) {
    status.textContent = `Last sync: ${new Date(data.rindleLastSuccess).toLocaleString()}\n${data.rindleLastCount?.imported ?? 0} new links imported`;
    status.style.whiteSpace = "pre-line";
  } else status.textContent = "Not synced yet. Click Sync now to check the local helper.";
}

sync.addEventListener("click", async () => {
  sync.disabled = true;
  status.textContent = "Syncing with Rindle…";
  try {
    const response = await chrome.runtime.sendMessage({ type: "rindle-sync-now" });
    if (!response?.ok) throw new Error(response?.error || "The extension worker was unavailable");
    await refresh();
  } catch (error) { status.textContent = `Could not sync: ${error.message}`; }
  sync.disabled = false;
});

browser.addEventListener("change", async () => {
  await chrome.storage.local.set({ rindleBrowser: browser.value });
  status.textContent = `Selected ${browser.selectedOptions[0]?.textContent}. Click Sync now to connect.`;
});

chrome.storage.local.get("rindleBrowser").then((value) => {
  browser.value = value.rindleBrowser ?? "dia";
}).catch(() => {});

refresh().catch((error) => { status.textContent = error.message; });
