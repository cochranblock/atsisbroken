// SPDX-License-Identifier: Unlicense
// background.js — service worker. Periodically drains
// chrome.storage.local.atsisbroken_observations into the native
// atsisbroken binary via Native Messaging. On Ack, clears synced events.

const STORAGE_KEY = 'atsisbroken_observations';
const NATIVE_HOST = 'org.cochranblock.atsisbroken';
const SYNC_INTERVAL_MIN = 5; // minutes

function frameIsAck(msg) {
  return msg && msg.kind === 'ack';
}

async function drainOnce() {
  const { [STORAGE_KEY]: queue = [] } = await chrome.storage.local.get([STORAGE_KEY]);
  if (queue.length === 0) return;

  const port = chrome.runtime.connectNative(NATIVE_HOST);
  let acked = 0;

  port.onMessage.addListener((msg) => {
    if (msg.kind === 'hello_ack') {
      port.postMessage({ kind: 'push_observations', events: queue });
    } else if (frameIsAck(msg)) {
      acked = msg.accepted;
      port.disconnect();
    }
  });
  port.onDisconnect.addListener(async () => {
    if (acked > 0) {
      await chrome.storage.local.set({ [STORAGE_KEY]: queue.slice(acked) });
    }
  });

  port.postMessage({ kind: 'hello', extension_version: chrome.runtime.getManifest().version });
}

chrome.alarms.create('atsisbroken-sync', { periodInMinutes: SYNC_INTERVAL_MIN });
chrome.alarms.onAlarm.addListener((a) => {
  if (a.name === 'atsisbroken-sync') drainOnce().catch(() => {});
});

// Manual trigger from popup.
chrome.runtime.onMessage.addListener((req, _sender, sendResponse) => {
  if (req && req.kind === 'sync_now') {
    drainOnce().then(() => sendResponse({ ok: true })).catch((e) => sendResponse({ ok: false, error: String(e) }));
    return true; // async response
  }
});
