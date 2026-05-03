// SPDX-License-Identifier: Unlicense
const STORAGE_KEY = 'atsisbroken_observations';

async function refresh() {
  const { [STORAGE_KEY]: queue = [] } = await chrome.storage.local.get([STORAGE_KEY]);
  document.getElementById('count').textContent = String(queue.length);
}

document.getElementById('sync').addEventListener('click', async () => {
  await chrome.runtime.sendMessage({ kind: 'sync_now' });
  setTimeout(refresh, 500);
});

refresh();
