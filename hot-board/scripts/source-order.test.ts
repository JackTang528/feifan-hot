import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { mergeSourceOrder, moveItem } from "../src/sourceOrder.ts";

describe("mergeSourceOrder", () => {
  it("follows the catalog when nothing is saved", () => {
    const catalog = ["hackernews", "weibo", "bilibili"];
    assert.deepEqual(mergeSourceOrder(catalog, []), catalog);
  });

  it("keeps the saved order and appends new sources in catalog order", () => {
    const catalog = [
      "hackernews",
      "weibo",
      "bilibili",
      "toutiao",
      "google_trends",
      "google_news",
    ];
    const saved = ["toutiao", "hackernews", "bilibili", "removed"];
    assert.deepEqual(mergeSourceOrder(catalog, saved), [
      "toutiao",
      "hackernews",
      "bilibili",
      "weibo",
      "google_trends",
      "google_news",
    ]);
  });

  it("drops blanks and duplicates", () => {
    assert.deepEqual(mergeSourceOrder(["a", "b", "c"], [" b ", "b", "  ", "a"]), [
      "b",
      "a",
      "c",
    ]);
  });
});

describe("moveItem", () => {
  it("moves an item down into the target slot", () => {
    assert.deepEqual(moveItem(["a", "b", "c", "d"], 0, 2), ["b", "c", "a", "d"]);
  });

  it("moves an item up into the target slot", () => {
    assert.deepEqual(moveItem(["a", "b", "c", "d"], 2, 0), ["c", "a", "b", "d"]);
  });

  it("returns null when the move would not change anything", () => {
    assert.equal(moveItem(["a", "b"], 0, -1), null);
    assert.equal(moveItem(["a", "b"], 1, 1), null);
    assert.equal(moveItem(["a", "b"], 1, 4), null);
  });
});
