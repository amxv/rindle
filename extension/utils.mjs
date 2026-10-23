export const HOST = "xyz.rindle.history";
export const ALARM_PERIOD_MINUTES = 5;
export const PAGE_SIZE = 500;
export const MAX_PAGES_PER_CYCLE = 5;
export const HOST_BATCH_SIZE = 100;

export function eligibleVisit(item) {
  if (!item || typeof item.url !== "string" || item.url.length > 8192) return false;
  try {
    const url = new URL(item.url);
    if (!["http:", "https:"].includes(url.protocol) || url.username || url.password) return false;
    if (["localhost", "127.0.0.1"].includes(url.hostname)) return false;
    return true;
  } catch {
    return false;
  }
}

export function toNativeVisits(historyItems) {
  return historyItems.filter(eligibleVisit).map((item) => ({
    url: item.url,
    title: String(item.title ?? "").slice(0, 512),
    lastVisitTime: Number.isFinite(item.lastVisitTime) ? Math.max(0, Math.floor(item.lastVisitTime)) : undefined,
  }));
}

export function chunks(items, size = HOST_BATCH_SIZE) {
  const batches = [];
  for (let i = 0; i < items.length; i += size) batches.push(items.slice(i, i + size));
  return batches;
}

export function nextCursor(items, previous) {
  const times = items.map((item) => item.lastVisitTime).filter(Number.isFinite);
  if (times.length === 0) return -1;
  return Math.min(previous - 1, ...times.map((v) => Math.floor(v) - 1));
}
