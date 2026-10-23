import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { eligibleVisit, toNativeVisits, chunks, nextCursor } from "./utils.mjs";

test("only public web history is offered to the native helper", () => {
  const visits = [
    { url: "https://example.com/a?search=hi", title: "A" },
    { url: "chrome://settings", title: "internal" },
    { url: "file:///private/private.txt" },
    { url: "http://localhost:3000" },
  ];
  assert.deepEqual(toNativeVisits(visits), [{ url: "https://example.com/a?search=hi", title: "A", lastVisitTime: undefined }]);
  assert.equal(eligibleVisit({ url: "https://example.org" }), true);
});

test("batches and cursor never skip to a newer timestamp", () => {
  assert.deepEqual(chunks([1, 2, 3, 4, 5], 2), [[1, 2], [3, 4], [5]]);
  assert.equal(nextCursor([{ lastVisitTime: 100 }, { lastVisitTime: 95 }], 200), 94);
  assert.equal(nextCursor([], 200), -1);
});

test("the unpacked extension has a stable public-key-derived ID", () => {
  const manifest = JSON.parse(readFileSync(new URL("./manifest.json", import.meta.url), "utf8"));
  const digest = createHash("sha256").update(Buffer.from(manifest.key, "base64")).digest().subarray(0, 16);
  const id = [...digest].map((byte) => "abcdefghijklmnop"[byte >> 4] + "abcdefghijklmnop"[byte & 15]).join("");
  assert.equal(id, "jlpbjckajdeinkbdbegabmfieclibihf");
  assert.equal(manifest.background.type, "module");
});
