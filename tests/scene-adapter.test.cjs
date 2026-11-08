const { test } = require("node:test");
const assert = require("node:assert/strict");
const { makeScene, makeSceneCard, copyScene } = require("../.cache/adapter-tests/fari/scene.js");
const { escapeHtml } = require("../.cache/adapter-tests/html.js");

test("editing a draft cannot change saved public or GM notes", () => {
  const saved = makeScene();
  saved.indexCards.public.push(makeSceneCard());
  saved.indexCards.private.push(makeSceneCard());
  saved.indexCards.public[0].blocks[0].value = "Public description";
  saved.indexCards.private[0].blocks[0].value = "Hidden clue";
  const draft = copyScene(saved);
  draft.indexCards.public[0].blocks[0].value = "Unsaved edit";
  draft.indexCards.private[0].blocks[0].value = "Unsaved GM edit";
  draft.indexCards.public.splice(0, 1);
  assert.equal(saved.indexCards.public.length, 1);
  assert.equal(saved.indexCards.public[0].blocks[0].value, "Public description");
  assert.equal(saved.indexCards.private[0].blocks[0].value, "Hidden clue");
});
test("independent scenes never share card collections or identifiers", () => {
  const a = makeScene(), b = makeScene();
  const card = makeSceneCard();
  a.indexCards.public.push(card);
  assert.equal(b.indexCards.public.length, 0);
  assert.notEqual(a.id, b.id);
  assert.notEqual(card.id, card.blocks[0].id);
});
test("scene, card, and token IDs work on HTTP previews without randomUUID", () => {
  const { webcrypto } = require("node:crypto");
  const { makeToken } = require("../.cache/adapter-tests/board-model.js");
  const originalCrypto = Object.getOwnPropertyDescriptor(globalThis, "crypto");
  let randomRequests = 0;
  Object.defineProperty(globalThis, "crypto", {
    configurable: true,
    value: { getRandomValues(bytes) { randomRequests++; return webcrypto.getRandomValues(bytes); } }
  });
  try {
    const scene = makeScene(), card = makeSceneCard(), token = makeToken("player", 1);
    const ids = [scene.id, card.id, card.blocks[0].id, token.id];
    for (const id of ids) assert.match(id, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
    assert.equal(new Set(ids).size, ids.length);
    assert.equal(randomRequests, ids.length);
  } finally {
    if (originalCrypto) Object.defineProperty(globalThis, "crypto", originalCrypto);
    else delete globalThis.crypto;
  }
});
test("card text is escaped before entering the HTML template", () => {
  const input = '<img src=x onerror="window.injected=true"> & <script>alert(1)</script>';
  const result = escapeHtml(input);
  assert.ok(!result.includes("<"));
  assert.ok(!result.includes(">"));
  assert.ok(!result.includes('"'));
  assert.ok(result.includes("&lt;img"));
  assert.ok(result.includes("&amp;"));
});
