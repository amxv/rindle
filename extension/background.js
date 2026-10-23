import {
  ALARM_PERIOD_MINUTES,
  HOST,
  MAX_PAGES_PER_CYCLE,
  PAGE_SIZE,
  chunks,
  nextCursor,
  toNativeVisits,
} from "./utils.mjs";

const PERIODIC_ALARM = "rindle-periodic";
const SOON_ALARM = "rindle-soon";
let activeSync = null;

function scheduleSoon() {
  chrome.alarms.create(SOON_ALARM, { delayInMinutes: 0.5 });
}

// Browser identity is user-selectable in the extension popup. The host wrapper
// independently checks it against the browser that launched the helper.
async function browserName() {
  const current = await chrome.storage.local.get("rindleBrowser");
  return current.rindleBrowser ?? "dia";
}

async function nativeExchange(visits = [], ack = []) {
  const browser = await browserName();
  const response = await chrome.runtime.sendNativeMessage(HOST, { version: 1, browser, visits, ack });
  if (!response?.ok || !Array.isArray(response.import)) {
    throw new Error("Rindle's native helper is not responding. Run rindle setup, then reload the extension.");
  }
  return response;
}

async function importPending(response) {
  let imported = 0;
  let received = response;
  for (let page = 0; page < 10; page++) {
    const acknowledged = [];
    for (const item of received.import) {
      if (typeof item.url !== "string") continue;
      try {
        const known = await chrome.history.getVisits({ url: item.url });
        if (known.length === 0) {
          await chrome.history.addUrl({ url: item.url });
          imported++;
        }
        acknowledged.push(item.url);
      } catch {
        // Leave the queue item pending. A future pass will retry it.
      }
    }
    if (acknowledged.length === 0) break;
    received = await nativeExchange([], acknowledged);
    if (received.import.length < 100) {
      // A second pass acknowledges this final page if there was one.
      if (received.import.length === 0) break;
    }
  }
  return imported;
}

async function exportRecent() {
  const local = await chrome.storage.local.get(["rindleScan", "rindleBackfillCursor", "rindleLastScan"]);
  // Persist the *whole* scan, including its original upper bound. Otherwise a
  // large burst repeatedly scans its newest 2,500 URLs and can starve older
  // unexported visits whenever the service worker is suspended between runs.
  const scan = local.rindleScan ?? {
    startedAt: Date.now(),
    lowerBound: local.rindleLastScan == null ? 0 : Math.max(0, local.rindleLastScan - 10 * 60 * 1000),
    endTime: local.rindleLastScan == null && local.rindleBackfillCursor != null
      ? local.rindleBackfillCursor
      : Date.now() + 1,
  };
  let exported = 0;
  let complete = false;
  for (let page = 0; page < MAX_PAGES_PER_CYCLE; page++) {
    const items = await chrome.history.search({
      text: "", startTime: scan.lowerBound, endTime: scan.endTime, maxResults: PAGE_SIZE,
    });
    if (items.length === 0) {
      complete = true;
      break;
    }
    const valid = toNativeVisits(items);
    for (const batch of chunks(valid)) {
      const reply = await nativeExchange(batch);
      exported += reply.received;
      await importPending(reply);
    }
    const candidate = nextCursor(items, scan.endTime);
    if (candidate < scan.lowerBound || candidate >= scan.endTime || items.length < PAGE_SIZE) {
      complete = true;
      break;
    }
    scan.endTime = candidate;
    // Commit the cursor only after every batch was accepted by the native host.
    await chrome.storage.local.set({ rindleScan: scan });
  }
  if (complete) {
    await chrome.storage.local.set({ rindleLastScan: scan.startedAt });
    await chrome.storage.local.remove(["rindleScan", "rindleBackfillCursor"]);
  } else {
    // A suspended or restarted extension resumes exactly where this pass ended.
    scheduleSoon();
  }
  return exported;
}

async function doSync() {
  const response = await nativeExchange();
  let imported = await importPending(response);
  const exported = await exportRecent();
  const latest = await nativeExchange();
  imported += await importPending(latest);
  await chrome.storage.local.set({ rindleLastSuccess: Date.now(), rindleLastError: "", rindleLastCount: { imported, exported } });
  return { imported, exported };
}

function runSync() {
  if (activeSync) return activeSync;
  activeSync = doSync().catch(async (error) => {
    await chrome.storage.local.set({ rindleLastError: String(error?.message ?? error) });
    throw error;
  }).finally(() => { activeSync = null; });
  return activeSync;
}

chrome.runtime.onInstalled.addListener(() => {
  chrome.alarms.create(PERIODIC_ALARM, { periodInMinutes: ALARM_PERIOD_MINUTES });
  scheduleSoon();
});
chrome.runtime.onStartup.addListener(() => {
  chrome.alarms.create(PERIODIC_ALARM, { periodInMinutes: ALARM_PERIOD_MINUTES });
  runSync().catch(() => {});
});
chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === PERIODIC_ALARM || alarm.name === SOON_ALARM) runSync().catch(() => {});
});
chrome.history.onVisited.addListener(scheduleSoon);
chrome.runtime.onMessage.addListener((message, _sender, reply) => {
  if (message?.type !== "rindle-sync-now") return;
  runSync().then((counts) => reply({ ok: true, ...counts })).catch((error) => reply({ ok: false, error: String(error.message ?? error) }));
  return true;
});
// The periodic alarm persists while Chromium terminates inactive service workers.
chrome.alarms.create(PERIODIC_ALARM, { periodInMinutes: ALARM_PERIOD_MINUTES });
