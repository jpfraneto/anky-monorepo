(() => {
  const DB_NAME = 'anky-write-protocol';
  const DB_VERSION = 1;
  const STORE_NAME = 'completed_sessions';
  const REJECTED_KEYS = new Set([
    'Backspace',
    'Delete',
    'ArrowLeft',
    'ArrowRight',
    'ArrowUp',
    'ArrowDown',
    'Enter',
    'Tab',
  ]);
  const BASE58_ALPHABET =
    '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
  const ANKYVERSE_START_MS = 1691658000000;
  const DAY_MS = 86400000;
  const KINGDOMS = [
    'primordia',
    'emblazion',
    'chryseos',
    'eleasis',
    'voxlumis',
    'insightia',
    'claridium',
    'poiesis',
  ];

  function normalizeChar(char) {
    if (char === ' ') return 'SPACE';
    return String(char).normalize('NFC');
  }

  function materializeChar(token) {
    return token === 'SPACE' ? ' ' : token;
  }

  function reconstructText(entries) {
    return (entries || []).map((entry) => materializeChar(entry.char)).join('');
  }

  function wordCount(text) {
    return text.trim() ? text.trim().split(/\s+/).length : 0;
  }

  function deriveKingdom(epochMs) {
    const dayIndex = Math.floor((epochMs - ANKYVERSE_START_MS) / DAY_MS);
    return KINGDOMS[((dayIndex % KINGDOMS.length) + KINGDOMS.length) % KINGDOMS.length];
  }

  function isRejectedKeydown(event) {
    return (
      !!event.ctrlKey ||
      !!event.metaKey ||
      REJECTED_KEYS.has(event.key) ||
      event.key.length !== 1
    );
  }

  function createCapture() {
    return {
      entries: [],
      lastKeystrokeAt: null,
      startedAtMs: null,
    };
  }

  function captureKeydown(capture, event, now) {
    const timestamp = typeof now === 'number' ? now : Date.now();
    if (!capture || isRejectedKeydown(event)) {
      return { accepted: false, now: timestamp };
    }

    const entry = capture.entries.length === 0
      ? { ms: 0, char: normalizeChar(event.key), startedAtMs: timestamp }
      : { ms: timestamp - capture.lastKeystrokeAt, char: normalizeChar(event.key) };

    capture.entries.push(entry);
    if (capture.startedAtMs == null) {
      capture.startedAtMs = timestamp;
    }
    capture.lastKeystrokeAt = timestamp;

    return {
      accepted: true,
      entry,
      first: capture.entries.length === 1,
      now: timestamp,
    };
  }

  function buildAnkyString(entries) {
    return entries.map((entry) => `${entry.ms} ${entry.char}`).join('\n');
  }

  async function sha256Hex(ankyString) {
    const bytes = new TextEncoder().encode(ankyString);
    const hashBuffer = await crypto.subtle.digest('SHA-256', bytes);
    return Array.from(new Uint8Array(hashBuffer))
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('');
  }

  async function verifyHash(ankyString, hash) {
    return (await sha256Hex(ankyString)) === hash;
  }

  async function buildFinalizedSession(entries) {
    if (!entries || entries.length === 0) {
      throw new Error('No accepted keystrokes captured.');
    }

    const content = buildAnkyString(entries);
    const hash = await sha256Hex(content);
    const verified = await verifyHash(content, hash);
    if (!verified) {
      throw new Error('SHA-256 verification failed.');
    }

    const text = reconstructText(entries);
    const createdAt = new Date().toISOString();
    const activeDurationMs = entries.slice(1).reduce((total, entry) => total + entry.ms, 0);
    const epochMs =
      typeof entries[0].startedAtMs === 'number'
        ? entries[0].startedAtMs
        : Date.now();

    return {
      id: hash,
      hash,
      content,
      created_at: createdAt,
      submit_state: 'pending',
      started_at: new Date(epochMs).toISOString(),
      active_duration_ms: activeDurationMs,
      duration_seconds: Math.round(activeDurationMs / 1000),
      word_count: wordCount(text),
      epoch_ms: epochMs,
      text,
      entries: entries.map((entry) => ({ ...entry })),
      kingdom: deriveKingdom(epochMs),
    };
  }

  function openDb() {
    return new Promise((resolve, reject) => {
      if (!window.indexedDB) {
        reject(new Error('IndexedDB is not available in this browser.'));
        return;
      }

      const request = window.indexedDB.open(DB_NAME, DB_VERSION);
      request.onerror = () => reject(request.error || new Error('IndexedDB open failed.'));
      request.onupgradeneeded = () => {
        const db = request.result;
        if (!db.objectStoreNames.contains(STORE_NAME)) {
          db.createObjectStore(STORE_NAME, { keyPath: 'id' });
        }
      };
      request.onsuccess = () => resolve(request.result);
    });
  }

  async function getStoredSession(hash) {
    const db = await openDb();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const request = store.get(hash);
      request.onerror = () => reject(request.error || new Error('IndexedDB read failed.'));
      request.onsuccess = () => resolve(request.result || null);
      tx.oncomplete = () => db.close();
      tx.onerror = () => reject(tx.error || new Error('IndexedDB transaction failed.'));
    });
  }

  async function storeCompletedSession(session) {
    const existing = await getStoredSession(session.hash);
    if (existing) {
      return existing;
    }

    const db = await openDb();
    const record = {
      id: session.hash,
      content: session.content,
      hash: session.hash,
      created_at: session.created_at,
      submit_state: session.submit_state,
    };

    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readwrite');
      tx.objectStore(STORE_NAME).put(record);
      tx.oncomplete = () => {
        db.close();
        resolve(record);
      };
      tx.onerror = () => reject(tx.error || new Error('IndexedDB write failed.'));
    });
  }

  async function updateSubmitState(hash, submitState) {
    const existing = await getStoredSession(hash);
    if (!existing) return null;

    const db = await openDb();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readwrite');
      tx.objectStore(STORE_NAME).put({
        ...existing,
        submit_state: submitState,
      });
      tx.oncomplete = () => {
        db.close();
        resolve({
          ...existing,
          submit_state: submitState,
        });
      };
      tx.onerror = () => reject(tx.error || new Error('IndexedDB update failed.'));
    });
  }

  function ensureDownloadBanner() {
    let banner = document.getElementById('anky-protocol-download-banner');
    if (banner) return banner;

    banner = document.createElement('div');
    banner.id = 'anky-protocol-download-banner';
    banner.style.position = 'fixed';
    banner.style.right = '16px';
    banner.style.bottom = '16px';
    banner.style.zIndex = '9999';
    banner.style.maxWidth = 'calc(100vw - 32px)';
    banner.style.padding = '10px 12px';
    banner.style.border = '1px solid rgba(255,255,255,0.12)';
    banner.style.borderRadius = '10px';
    banner.style.background = 'rgba(8, 8, 8, 0.94)';
    banner.style.boxShadow = '0 12px 30px rgba(0,0,0,0.35)';
    banner.style.fontFamily = 'system-ui, -apple-system, sans-serif';
    banner.style.fontSize = '13px';
    banner.style.color = '#e0e0e0';
    banner.style.display = 'none';
    document.body.appendChild(banner);
    return banner;
  }

  function offerDownload(session) {
    const filename = `${session.hash}.anky`;
    const blob = new Blob([session.content], { type: 'text/plain;charset=utf-8' });
    const objectUrl = URL.createObjectURL(blob);
    const banner = ensureDownloadBanner();

    banner.innerHTML = '';
    const label = document.createElement('div');
    label.textContent = 'your .anky file';
    label.style.marginBottom = '6px';

    const link = document.createElement('a');
    link.href = objectUrl;
    link.download = filename;
    link.textContent = `download ${filename}`;
    link.style.color = '#b366ff';
    link.style.textDecoration = 'none';
    link.style.wordBreak = 'break-all';

    banner.appendChild(label);
    banner.appendChild(link);
    banner.style.display = 'block';

    const clicker = document.createElement('a');
    clicker.href = objectUrl;
    clicker.download = filename;
    clicker.style.display = 'none';
    document.body.appendChild(clicker);
    clicker.click();
    clicker.remove();

    window.setTimeout(() => URL.revokeObjectURL(objectUrl), 60000);

    return { filename, object_url: objectUrl };
  }

  function showVisibleError(message) {
    let banner = document.getElementById('anky-protocol-error-banner');
    if (!banner) {
      banner = document.createElement('div');
      banner.id = 'anky-protocol-error-banner';
      banner.style.position = 'fixed';
      banner.style.left = '16px';
      banner.style.right = '16px';
      banner.style.top = '16px';
      banner.style.zIndex = '10000';
      banner.style.padding = '12px 14px';
      banner.style.border = '1px solid rgba(255,68,68,0.45)';
      banner.style.borderRadius = '10px';
      banner.style.background = 'rgba(62, 10, 10, 0.96)';
      banner.style.color = '#ffd7d7';
      banner.style.fontFamily = 'system-ui, -apple-system, sans-serif';
      banner.style.fontSize = '13px';
      banner.style.boxShadow = '0 12px 30px rgba(0,0,0,0.35)';
      document.body.appendChild(banner);
    }
    banner.textContent = message;
    console.error('[anky protocol]', message);
  }

  function base58Encode(value) {
    const bytes = value instanceof Uint8Array ? value : new Uint8Array(value);
    if (bytes.length === 0) return '';

    const digits = [0];
    for (let i = 0; i < bytes.length; i += 1) {
      let carry = bytes[i];
      for (let j = 0; j < digits.length; j += 1) {
        const next = digits[j] * 256 + carry;
        digits[j] = next % 58;
        carry = Math.floor(next / 58);
      }
      while (carry > 0) {
        digits.push(carry % 58);
        carry = Math.floor(carry / 58);
      }
    }

    let leadingZeroCount = 0;
    while (leadingZeroCount < bytes.length && bytes[leadingZeroCount] === 0) {
      leadingZeroCount += 1;
    }

    let encoded = '1'.repeat(leadingZeroCount);
    for (let i = digits.length - 1; i >= 0; i -= 1) {
      encoded += BASE58_ALPHABET[digits[i]];
    }
    return encoded;
  }

  window.AnkyWriteProtocol = {
    base58Encode,
    buildAnkyString,
    buildFinalizedSession,
    captureKeydown,
    createCapture,
    deriveKingdom,
    getStoredSession,
    isRejectedKeydown,
    materializeChar,
    normalizeChar,
    offerDownload,
    reconstructText,
    sha256Hex,
    showVisibleError,
    storeCompletedSession,
    updateSubmitState,
    verifyHash,
    wordCount,
  };
})();
