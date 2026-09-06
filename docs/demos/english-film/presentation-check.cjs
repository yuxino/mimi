const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { test } = require('node:test');
const vm = require('node:vm');

// Test the documentation boundary, not native OS shortcut registration.
function harness() {
  const elements = new Map();
  const events = [];
  const create = () => ({ style: {}, contentWindow: { __emit: (event, payload) => events.push({ event, payload }) }, addEventListener() {}, remove() { elements.delete(this.id); } });
  for (const id of ['movie', 'mimi-menu', 'clock', 'pointer']) elements.set(id, create());
  elements.get('movie').currentTime = 0;
  const clock = { now: 1000 };
  const context = {
    window: {}, document: { getElementById: id => elements.get(id), createElement: create,
      querySelectorAll: () => [...elements.values()].filter(e => e.src), addEventListener() {},
      body: { append: e => elements.set(e.id, e) } },
    performance: { now: () => clock.now, timeOrigin: 0 }, Date,
    structuredClone, setInterval() {}, fetch() { throw Error('No network in presentation regression'); },
  };
  context.window = context;
  vm.createContext(context);
  vm.runInContext(readFileSync(join(__dirname, 'stage.js'), 'utf8'), context);
  vm.runInContext("showWindow('overlay');showWindow('overlay-control');session.isActive=true;snapshot()", context);
  function shortcut(extra = {}) {
    const event = { code: 'KeyM', ctrlKey: true, metaKey: false, shiftKey: true, altKey: false, repeat: false,
      preventDefault() { this.prevented = true; }, ...extra };
    context.handleShortcut(event);
    return event;
  }
  return { context, elements, clock, shortcut, snapshot: () => vm.runInContext('snapshot()', context) };
}

test('Immersive shortcut hides the entire control window and makes the canvas click-through', async () => {
  const h = harness();
  assert.equal(h.context.demoState.settings.fontSize, 16);
  assert.equal(h.context.demoState.controlMode, 'island');
  assert.equal(h.shortcut().prevented, true);
  assert.equal(h.context.demoState.settings.subtitleBlendsWithBackground, true);
  assert.equal(h.context.demoState.controlMode, 'hidden');
  assert.equal(h.elements.get('overlay-control').style.display, 'none');
  assert.equal(h.elements.get('overlay').style.pointerEvents, 'none');
  await h.context.invoke('overlay-control', 'overlay_popover_hide');
  await h.context.invoke('overlay-control', 'overlay_popover_toggle');
  h.snapshot();
  assert.equal(h.context.demoState.controlMode, 'hidden');
  assert.equal(h.elements.get('overlay-control').style.display, 'none');
  h.clock.now += 600;
  h.shortcut({ ctrlKey: false, metaKey: true });
  assert.equal(h.context.demoState.controlMode, 'island');
  assert.equal(h.elements.get('overlay-control').style.display, '');
  assert.equal(h.elements.get('overlay').style.pointerEvents, 'auto');
});

test('Key repeats, wrong combinations and debounce cannot accidentally toggle twice', () => {
  const h = harness();
  h.shortcut({ shiftKey: false });
  h.shortcut({ altKey: true });
  h.shortcut({ ctrlKey: false });
  assert.equal(h.context.demoState.settings.subtitleBlendsWithBackground, false);
  h.shortcut();h.shortcut();
  assert.equal(h.context.demoState.settings.subtitleBlendsWithBackground, true);
  h.clock.now += 600;h.shortcut({ repeat: true });
  assert.equal(h.context.demoState.settings.subtitleBlendsWithBackground, true);
  assert.equal(h.context.demoState.calls.filter(x => x.command === 'immersive_shortcut').length, 1);
});
