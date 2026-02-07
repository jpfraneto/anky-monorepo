const API_BASE = 'https://anky.app';

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'transform') {
    handleTransform(message.data).then(sendResponse).catch(err => {
      sendResponse({ error: err.message });
    });
    return true; // async response
  }

  if (message.type === 'getBalance') {
    handleGetBalance().then(sendResponse).catch(err => {
      sendResponse({ error: err.message });
    });
    return true;
  }

  if (message.type === 'getApiKey') {
    chrome.storage.local.get(['apiKey'], (result) => {
      sendResponse({ apiKey: result.apiKey || null });
    });
    return true;
  }
});

async function getApiKey() {
  return new Promise((resolve) => {
    chrome.storage.local.get(['apiKey'], (result) => {
      resolve(result.apiKey || null);
    });
  });
}

async function handleTransform(data) {
  const apiKey = await getApiKey();
  if (!apiKey) {
    throw new Error('No API key set. Open the extension popup to add one.');
  }

  const resp = await fetch(`${API_BASE}/api/v1/transform`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-API-Key': apiKey,
    },
    body: JSON.stringify({
      writing: data.writing,
      prompt: data.prompt || null,
    }),
  });

  if (!resp.ok) {
    const err = await resp.json().catch(() => ({ error: resp.statusText }));
    throw new Error(err.error || `API error: ${resp.status}`);
  }

  return await resp.json();
}

async function handleGetBalance() {
  const apiKey = await getApiKey();
  if (!apiKey) {
    throw new Error('No API key set.');
  }

  const resp = await fetch(`${API_BASE}/api/v1/balance`, {
    headers: { 'X-API-Key': apiKey },
  });

  if (!resp.ok) {
    const err = await resp.json().catch(() => ({ error: resp.statusText }));
    throw new Error(err.error || `API error: ${resp.status}`);
  }

  return await resp.json();
}
