const apiKeyInput = document.getElementById('apiKey');
const saveBtn = document.getElementById('saveBtn');
const keyStatus = document.getElementById('keyStatus');
const balanceSection = document.getElementById('balanceSection');
const balanceAmount = document.getElementById('balanceAmount');
const totalSpent = document.getElementById('totalSpent');
const totalTransforms = document.getElementById('totalTransforms');

// Load saved key
chrome.storage.local.get(['apiKey'], (result) => {
  if (result.apiKey) {
    apiKeyInput.value = result.apiKey;
    fetchBalance();
  }
});

saveBtn.addEventListener('click', () => {
  const key = apiKeyInput.value.trim();
  if (!key.startsWith('anky_') || key.length !== 37) {
    keyStatus.textContent = 'invalid key format (anky_ + 32 hex chars)';
    keyStatus.className = 'status error';
    return;
  }

  chrome.storage.local.set({ apiKey: key }, () => {
    keyStatus.textContent = 'key saved';
    keyStatus.className = 'status ok';
    fetchBalance();
  });
});

function fetchBalance() {
  chrome.runtime.sendMessage({ type: 'getBalance' }, (resp) => {
    if (resp && !resp.error) {
      balanceSection.style.display = 'block';
      balanceAmount.textContent = `$${resp.balance_usd.toFixed(4)}`;
      totalSpent.textContent = `$${resp.total_spent_usd.toFixed(4)}`;
      totalTransforms.textContent = resp.total_transforms;
      keyStatus.textContent = 'connected';
      keyStatus.className = 'status ok';
    } else {
      keyStatus.textContent = resp ? resp.error : 'connection failed';
      keyStatus.className = 'status error';
    }
  });
}
