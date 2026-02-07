(() => {
  'use strict';

  const ACTIVATION_DELAY = 8000;  // 8 seconds of typing to activate
  const SESSION_DURATION = 480000; // 8 minutes
  const SILENCE_TIMEOUT = 8000;    // 8 seconds of silence = fail

  const sessions = new WeakMap();

  function getEditableElements() {
    const textareas = document.querySelectorAll('textarea');
    const contentEditables = document.querySelectorAll('[contenteditable="true"]');
    return [...textareas, ...contentEditables];
  }

  function initElement(el) {
    if (sessions.has(el)) return;

    const state = {
      phase: 'idle', // idle | activating | writing | completed | failed
      typingStart: null,
      lastKeystroke: null,
      sessionStart: null,
      silenceTimer: null,
      sessionTimer: null,
      activationTimer: null,
      lifebar: null,
      sandclock: null,
      modal: null,
      writtenContent: '',
    };
    sessions.set(el, state);

    el.addEventListener('keydown', (e) => handleKeystroke(el, state, e));
    el.addEventListener('blur', () => {
      if (state.phase === 'activating') {
        resetState(el, state);
      }
    });
  }

  function handleKeystroke(el, state, e) {
    // Ignore modifier-only keys
    if (['Shift', 'Control', 'Alt', 'Meta', 'Tab', 'Escape'].includes(e.key)) return;

    const now = Date.now();
    state.lastKeystroke = now;

    if (state.phase === 'idle') {
      state.phase = 'activating';
      state.typingStart = now;
      state.activationTimer = setTimeout(() => {
        activateSession(el, state);
      }, ACTIVATION_DELAY);
      return;
    }

    if (state.phase === 'activating') {
      // Reset silence detection during activation
      clearTimeout(state.activationTimer);
      const elapsed = now - state.typingStart;
      const remaining = ACTIVATION_DELAY - elapsed;
      if (remaining <= 0) {
        activateSession(el, state);
      } else {
        state.activationTimer = setTimeout(() => {
          activateSession(el, state);
        }, remaining);
      }
      // Check for silence during activation
      clearTimeout(state.silenceTimer);
      state.silenceTimer = setTimeout(() => {
        resetState(el, state);
      }, SILENCE_TIMEOUT);
      return;
    }

    if (state.phase === 'writing') {
      // Reset silence timer
      clearTimeout(state.silenceTimer);
      state.silenceTimer = setTimeout(() => {
        failSession(el, state);
      }, SILENCE_TIMEOUT);
      updateLifebar(el, state);
      return;
    }
  }

  function activateSession(el, state) {
    state.phase = 'writing';
    state.sessionStart = Date.now();

    // Create life bar
    state.lifebar = createLifebar(el);

    // Create sandclock canvas
    state.sandclock = createSandclock(el);

    // Set silence timer
    state.silenceTimer = setTimeout(() => {
      failSession(el, state);
    }, SILENCE_TIMEOUT);

    // Set session complete timer
    state.sessionTimer = setTimeout(() => {
      completeSession(el, state);
    }, SESSION_DURATION);

    // Start animation loop
    animateSession(el, state);
  }

  function createLifebar(el) {
    const bar = document.createElement('div');
    bar.className = 'anky-lifebar';
    // Position relative to the element
    const rect = el.getBoundingClientRect();
    bar.style.position = 'fixed';
    bar.style.top = `${rect.top}px`;
    bar.style.left = `${rect.left}px`;
    bar.style.width = `${rect.width}px`;
    document.body.appendChild(bar);

    const fill = document.createElement('div');
    fill.className = 'anky-lifebar-fill';
    bar.appendChild(fill);

    return bar;
  }

  function createSandclock(el) {
    const canvas = document.createElement('canvas');
    canvas.className = 'anky-sandclock';
    canvas.width = 60;
    canvas.height = 80;
    const rect = el.getBoundingClientRect();
    canvas.style.position = 'fixed';
    canvas.style.top = `${rect.top + 4}px`;
    canvas.style.right = `${window.innerWidth - rect.right + 4}px`;
    canvas.style.pointerEvents = 'none';
    canvas.style.opacity = '0.15';
    canvas.style.zIndex = '10000';
    document.body.appendChild(canvas);

    canvas._particles = [];
    for (let i = 0; i < 30; i++) {
      canvas._particles.push({
        x: 25 + Math.random() * 10,
        y: 5 + Math.random() * 15,
        vy: 0.2 + Math.random() * 0.3,
        size: 1 + Math.random() * 1.5,
        opacity: 0.5 + Math.random() * 0.5,
      });
    }

    return canvas;
  }

  function animateSession(el, state) {
    if (state.phase !== 'writing') return;

    const now = Date.now();
    const elapsed = now - state.sessionStart;
    const progress = Math.min(elapsed / SESSION_DURATION, 1);

    // Update lifebar position (element might scroll)
    if (state.lifebar) {
      const rect = el.getBoundingClientRect();
      state.lifebar.style.top = `${rect.top}px`;
      state.lifebar.style.left = `${rect.left}px`;
      state.lifebar.style.width = `${rect.width}px`;
      const fill = state.lifebar.querySelector('.anky-lifebar-fill');
      if (fill) {
        fill.style.width = `${progress * 100}%`;
      }
    }

    // Update sandclock position and animation
    if (state.sandclock) {
      const rect = el.getBoundingClientRect();
      state.sandclock.style.top = `${rect.top + 4}px`;
      state.sandclock.style.right = `${window.innerWidth - rect.right + 4}px`;
      drawSandclock(state.sandclock, progress);
    }

    // Update silence indicator (lifebar pulse)
    if (state.lifebar && state.lastKeystroke) {
      const silence = now - state.lastKeystroke;
      const silenceRatio = Math.min(silence / SILENCE_TIMEOUT, 1);
      const fill = state.lifebar.querySelector('.anky-lifebar-fill');
      if (fill) {
        // As silence grows, shift color from purple to red
        if (silenceRatio > 0.5) {
          const r = Math.round(124 + (239 - 124) * (silenceRatio - 0.5) * 2);
          const g = Math.round(77 * (1 - (silenceRatio - 0.5) * 2));
          const b = Math.round(255 * (1 - (silenceRatio - 0.5) * 2));
          fill.style.backgroundColor = `rgb(${r}, ${g}, ${b})`;
        } else {
          fill.style.backgroundColor = '#7c4dff';
        }
      }
    }

    requestAnimationFrame(() => animateSession(el, state));
  }

  function drawSandclock(canvas, progress) {
    const ctx = canvas.getContext('2d');
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Draw hourglass outline
    ctx.strokeStyle = '#7c4dff';
    ctx.lineWidth = 1;
    ctx.beginPath();
    // Top triangle
    ctx.moveTo(10, 5);
    ctx.lineTo(50, 5);
    ctx.lineTo(30, 38);
    ctx.closePath();
    ctx.stroke();
    // Bottom triangle
    ctx.beginPath();
    ctx.moveTo(10, 75);
    ctx.lineTo(50, 75);
    ctx.lineTo(30, 42);
    ctx.closePath();
    ctx.stroke();

    // Animate particles falling
    const particles = canvas._particles;
    for (const p of particles) {
      p.y += p.vy;
      // Reset particles that fall below
      if (p.y > 70) {
        p.y = 5 + Math.random() * 10;
        p.x = 25 + Math.random() * 10;
      }

      ctx.fillStyle = `rgba(124, 77, 255, ${p.opacity})`;
      ctx.beginPath();
      ctx.arc(p.x, p.y, p.size, 0, Math.PI * 2);
      ctx.fill();
    }

    // Fill bottom based on progress
    if (progress > 0) {
      const fillHeight = progress * 30;
      ctx.fillStyle = 'rgba(124, 77, 255, 0.3)';
      ctx.beginPath();
      ctx.moveTo(30, 75);
      const spread = progress * 18;
      ctx.lineTo(30 - spread, 75 - fillHeight);
      ctx.lineTo(30 + spread, 75 - fillHeight);
      ctx.closePath();
      ctx.fill();
    }
  }

  function updateLifebar(el, state) {
    // Lifebar updates happen in animateSession loop
  }

  function failSession(el, state) {
    state.phase = 'failed';
    clearTimeout(state.sessionTimer);
    clearTimeout(state.silenceTimer);

    // Fade out visuals
    if (state.lifebar) {
      state.lifebar.style.transition = 'opacity 1s';
      state.lifebar.style.opacity = '0';
      setTimeout(() => state.lifebar?.remove(), 1000);
    }
    if (state.sandclock) {
      state.sandclock.style.transition = 'opacity 1s';
      state.sandclock.style.opacity = '0';
      setTimeout(() => state.sandclock?.remove(), 1000);
    }

    // Reset after fade
    setTimeout(() => {
      state.phase = 'idle';
      state.typingStart = null;
      state.sessionStart = null;
      state.lifebar = null;
      state.sandclock = null;
    }, 1200);
  }

  function completeSession(el, state) {
    state.phase = 'completed';
    clearTimeout(state.silenceTimer);

    // Capture the writing
    const writing = el.value || el.innerText || el.textContent || '';

    // Remove session visuals
    state.lifebar?.remove();
    state.sandclock?.remove();

    // Show completion modal
    showCompletionModal(el, state, writing);
  }

  function showCompletionModal(el, state, writing) {
    const overlay = document.createElement('div');
    overlay.className = 'anky-modal-overlay';

    const modal = document.createElement('div');
    modal.className = 'anky-modal';

    const wordCount = writing.trim().split(/\s+/).length;

    modal.innerHTML = `
      <h2 class="anky-modal-title">8 minutes complete</h2>
      <p class="anky-modal-stats">${wordCount} words of raw consciousness</p>
      <div class="anky-modal-writing">${escapeHtml(writing.substring(0, 500))}${writing.length > 500 ? '...' : ''}</div>
      <div class="anky-modal-prompt-section">
        <label class="anky-modal-label">transformation prompt (optional)</label>
        <input type="text" class="anky-modal-input" id="anky-prompt" placeholder="e.g., turn this into a poem, extract key insights, find the core message..." />
      </div>
      <div class="anky-modal-actions">
        <button class="anky-btn anky-btn-primary" id="anky-transform-btn">transform</button>
        <button class="anky-btn anky-btn-secondary" id="anky-close-btn">close</button>
      </div>
      <div class="anky-modal-result" id="anky-result" style="display:none;"></div>
      <div class="anky-modal-cost" id="anky-cost" style="display:none;"></div>
    `;

    overlay.appendChild(modal);
    document.body.appendChild(overlay);
    state.modal = overlay;

    // Transform button
    modal.querySelector('#anky-transform-btn').addEventListener('click', async () => {
      const prompt = modal.querySelector('#anky-prompt').value;
      const btn = modal.querySelector('#anky-transform-btn');
      btn.textContent = 'transforming...';
      btn.disabled = true;

      chrome.runtime.sendMessage(
        { type: 'transform', data: { writing, prompt: prompt || null } },
        (resp) => {
          const resultEl = modal.querySelector('#anky-result');
          const costEl = modal.querySelector('#anky-cost');

          if (resp && !resp.error) {
            resultEl.style.display = 'block';
            resultEl.innerHTML = `<div class="anky-transformed">${escapeHtml(resp.transformed)}</div>`;
            costEl.style.display = 'block';
            costEl.textContent = `cost: $${resp.cost_usd.toFixed(4)} | balance: $${resp.balance_remaining.toFixed(4)}`;
            btn.textContent = 'transform again';
            btn.disabled = false;
          } else {
            resultEl.style.display = 'block';
            resultEl.innerHTML = `<div class="anky-error">${escapeHtml(resp?.error || 'transformation failed')}</div>`;
            btn.textContent = 'retry';
            btn.disabled = false;
          }
        }
      );
    });

    // Close button
    modal.querySelector('#anky-close-btn').addEventListener('click', () => {
      overlay.remove();
      resetState(el, state);
    });

    // Click overlay to close
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) {
        overlay.remove();
        resetState(el, state);
      }
    });
  }

  function resetState(el, state) {
    clearTimeout(state.activationTimer);
    clearTimeout(state.silenceTimer);
    clearTimeout(state.sessionTimer);
    state.lifebar?.remove();
    state.sandclock?.remove();
    state.modal?.remove();
    state.phase = 'idle';
    state.typingStart = null;
    state.sessionStart = null;
    state.lastKeystroke = null;
    state.lifebar = null;
    state.sandclock = null;
    state.modal = null;
  }

  function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  // Initialize on all existing textareas
  function scanAndInit() {
    getEditableElements().forEach(initElement);
  }

  // Watch for dynamically added textareas
  const observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      for (const node of mutation.addedNodes) {
        if (node.nodeType !== 1) continue;
        if (node.matches && (node.matches('textarea') || node.matches('[contenteditable="true"]'))) {
          initElement(node);
        }
        if (node.querySelectorAll) {
          node.querySelectorAll('textarea, [contenteditable="true"]').forEach(initElement);
        }
      }
    }
  });

  observer.observe(document.body, { childList: true, subtree: true });
  scanAndInit();

  // Re-scan periodically for SPAs that mutate existing elements
  setInterval(scanAndInit, 5000);
})();
