const { test } = require("node:test");
const assert = require("node:assert/strict");
const { describeTerm, describeRoll } = require("../.cache/adapter-tests/dice-format.js");

test("advantage display shows which face was discarded and which was kept", () => {
  const text = describeTerm({kind:"dice",sign:1,count:1,sides:20,rolls:[4,17],kept:[false,true],subtotal:17});
  assert.ok(text.includes("4 discarded"));
  assert.ok(text.includes("17 kept"));
  assert.ok(text.endsWith("= 17"));
});
test("negative dice and signed modifiers remain visible", () => {
  assert.equal(describeTerm({kind:"modifier",value:-2}),"-2");
  assert.equal(describeTerm({kind:"modifier",value:5}),"+5");
  assert.equal(describeTerm({kind:"dice",sign:-1,count:1,sides:4,rolls:[3],kept:[true],subtotal:-3}),"-1d4: 3 kept = -3");
});
test("roll history labels retain the mode and recorded total", () => {
  const roll = {id:"test",campaignId:"campaign",characterId:null,actorName:null,label:"Check",expression:"1d20+5",
    mode:"advantage",terms:[],total:22,createdAt:"2026-11-07T16:45:00Z"};
  assert.equal(describeRoll(roll),"1d20+5 (advantage) = 22");
  assert.equal(describeRoll({...roll,mode:"normal"}),"1d20+5 = 22");
});
