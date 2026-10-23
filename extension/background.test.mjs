import test from "node:test";
import assert from "node:assert/strict";

test("MV3 service worker imports queued Safari links and exports Chrome links without navigating", async () => {
  const hooks = {};
  const saved = { rindleBrowser: "chrome" };
  const chromeHistory = [{ url: "https://desktop.example/page", title: "Desktop Page", lastVisitTime: 2000 }];
  const visited = new Set(chromeHistory.map((item) => item.url));
  const injected = [];
  const nativeMessages = [];
  const pending = new Set(["https://phone.example/interesting"]);
  globalThis.chrome = {
    alarms: {
      create: () => {},
      onAlarm: { addListener: (handler) => { hooks.alarm = handler; } },
    },
    history: {
      search: async () => chromeHistory,
      getVisits: async ({ url }) => visited.has(url) ? [{ visitId: "existing" }] : [],
      addUrl: async ({ url }) => { injected.push(url); visited.add(url); },
      onVisited: { addListener: (handler) => { hooks.visited = handler; } },
    },
    runtime: {
      onStartup: { addListener: (handler) => { hooks.startup = handler; } },
      onInstalled: { addListener: (handler) => { hooks.installed = handler; } },
      onMessage: { addListener: (handler) => { hooks.message = handler; } },
      sendNativeMessage: async (host, data) => {
        assert.equal(host, "xyz.rindle.history");
        nativeMessages.push(data);
        assert.equal(data.browser, "chrome");
        data.ack.forEach((url) => pending.delete(url));
        return {
          ok: true,
          received: data.visits.length,
          acknowledged: data.ack.length,
          import: [...pending].map((url) => ({ url })),
        };
      },
    },
    storage: { local: {
      get: async (keys) => Object.fromEntries((Array.isArray(keys) ? keys : [keys]).map((key) => [key, saved[key]])),
      set: async (data) => Object.assign(saved, data),
      remove: async (key) => { delete saved[key]; },
    } },
  };

  await import("./background.js");
  assert.equal(typeof hooks.message, "function");
  assert.equal(typeof hooks.startup, "function");
  assert.equal(typeof hooks.alarm, "function");
  const syncNow = () => new Promise((resolve) => {
    const retained = hooks.message({ type: "rindle-sync-now" }, {}, resolve);
    assert.equal(retained, true);
  });

  const first = await syncNow();
  assert.deepEqual(first, { ok: true, imported: 1, exported: 1 });
  assert.deepEqual(injected, ["https://phone.example/interesting"]);
  assert.equal(pending.size, 0);
  assert.equal(saved.rindleLastError, "");
  assert.ok(nativeMessages.some(({ visits }) => visits.some(({ url }) => url === "https://desktop.example/page")));
  assert.ok(nativeMessages.some(({ ack }) => ack.includes("https://phone.example/interesting")));

  const again = await syncNow();
  assert.equal(again.imported, 0);
  assert.equal(injected.length, 1, "retries must not add another history visit");
  assert.ok(saved.rindleLastSuccess);
  delete globalThis.chrome;
});

test("a large backfill resumes across multiple short service-worker runs", async () => {
  const hooks = {};
  const storage = { rindleBrowser: "dia" };
  const sent = new Set();
  const created = Date.now();
  const history = Array.from({ length: 2_601 }, (_, index) => ({
    url: `https://example.org/history-${index}`,
    lastVisitTime: created - 10_000 - index,
    title: `History ${index}`,
  }));
  let alarmCount = 0;
  const searches = [];
  globalThis.chrome = {
    alarms: {
      create: (name) => { if (name === "rindle-soon") alarmCount++; },
      onAlarm: { addListener: (handler) => { hooks.alarm = handler; } },
    },
    history: {
      search: async ({ startTime, endTime, maxResults }) => {
        searches.push({ startTime, endTime });
        return history.filter((item) => item.lastVisitTime >= startTime && item.lastVisitTime <= endTime)
          .sort((a, b) => b.lastVisitTime - a.lastVisitTime).slice(0, maxResults);
      },
      getVisits: async () => [],
      addUrl: async () => { throw new Error("There are no queued incoming URLs"); },
      onVisited: { addListener: () => {} },
    },
    runtime: {
      onStartup: { addListener: (handler) => { hooks.startup = handler; } },
      onInstalled: { addListener: () => {} },
      onMessage: { addListener: (handler) => { hooks.message = handler; } },
      sendNativeMessage: async (_host, request) => {
        for (const visit of request.visits) sent.add(visit.url);
        return { ok: true, received: request.visits.length, acknowledged: 0, import: [] };
      },
    },
    storage: { local: {
      get: async (keys) => Object.fromEntries((Array.isArray(keys) ? keys : [keys]).map((key) => [key, storage[key]])),
      set: async (data) => Object.assign(storage, data),
      remove: async (keys) => { for (const key of Array.isArray(keys) ? keys : [keys]) delete storage[key]; },
    } },
  };
  try {
    await import("./background.js?case=large-backfill");
    const run = () => new Promise((resolve) => hooks.message({ type: "rindle-sync-now" }, {}, resolve));
    const first = await run();
    assert.equal(first.ok, true);
    assert.equal(first.exported, 2_500);
    assert.equal(sent.size, 2_500);
    assert.ok(storage.rindleScan, "unfinished history must retain an exact resume cursor");
    assert.equal(storage.rindleLastScan, undefined, "do not mark an unfinished history scan as complete");
    assert.ok(alarmCount > 0, "schedule the next backfill chunk");
    const previousCursor = storage.rindleScan.endTime;
    const second = await run();
    assert.equal(second.exported, 101);
    assert.equal(sent.size, 2_601);
    assert.equal(storage.rindleScan, undefined);
    assert.ok(storage.rindleLastScan >= created);
    assert.ok(searches.some(({ endTime }) => endTime === previousCursor), "resumed scan must use its saved cursor");
  } finally {
    delete globalThis.chrome;
  }
});
