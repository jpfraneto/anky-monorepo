# Anky Chrome Extension

## How It Works

The extension monitors all textareas and contenteditable elements on every page.

### Flow
1. Start typing in any textarea
2. After **8 seconds** of continuous typing, "anky mode" activates
3. A thin purple **life bar** appears at the top of the textarea
4. A subtle **sandclock** animation appears (canvas particles, very low opacity)
5. Keep writing for **8 minutes** without stopping for more than 8 seconds
6. If you stop for 8 seconds → session fails, visuals fade away
7. Complete 8 minutes → modal appears with your writing
8. Optionally enter a transformation prompt
9. Click "transform" → sends to anky.app API → shows AI-transformed result + cost

### Visual Indicators
- **Life bar**: Purple bar that fills as you progress through 8 minutes. Turns red when approaching silence timeout.
- **Sandclock**: Subtle canvas animation (opacity 0.15) showing particles flowing downward in an hourglass shape. Bottom fills as time progresses.

## Development Setup

1. Open Chrome → `chrome://extensions`
2. Enable "Developer mode" (top right)
3. Click "Load unpacked"
4. Select the `extension/` directory

### Files
- `manifest.json` — Extension configuration (Manifest V3)
- `content.js` — Main content script (textarea detection, session management)
- `styles.css` — Injected styles (life bar, modal, transformation display)
- `background.js` — Service worker (API calls to anky.app)
- `popup.html/js` — Extension popup (API key input, balance display)
- `icons/` — Purple circle icons (16/48/128px)

## API Key Setup

1. Go to https://anky.app/credits
2. Create an API key
3. Send USDC on Base to the treasury address
4. Verify the payment
5. Paste your API key in the extension popup

## Chrome Web Store Publishing

1. Create a [Chrome Web Store developer account](https://chrome.google.com/webstore/devconsole/) ($5 one-time fee)
2. Zip the extension directory: `cd ~/anky && zip -r anky-extension.zip extension/`
3. Upload to the developer dashboard
4. Fill in listing details, screenshots, description
5. Submit for review

## API Endpoints Used

- `POST /api/v1/transform` — Transform writing (requires X-API-Key header)
  - Body: `{ "writing": "...", "prompt": "optional prompt" }`
  - Response: `{ "transformed": "...", "cost_usd": 0.03, "balance_remaining": 4.97 }`
- `GET /api/v1/balance` — Check balance (requires X-API-Key header)
  - Response: `{ "balance_usd": 5.0, "total_spent_usd": 0.15, "total_transforms": 5, "recent_transforms": [...] }`
