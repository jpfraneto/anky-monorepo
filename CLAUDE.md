# Anky — Claude Code Instructions

## Changelog Protocol

Every conversation that results in code changes MUST update the changelog:

1. **Save each user prompt** as a txt file in `static/changelog/` with the naming convention:
   `YYYY-MM-DD-NNN-slug.txt` (e.g. `2026-02-14-001-video-studio.txt`)
   - NNN is a zero-padded sequence number per day (001, 002, 003...)
   - slug is a short kebab-case description
   - File contains the user's raw prompt text, exactly as they wrote it

2. **Add an entry to `templates/changelog.html`** at the TOP of the entries list (newest first), using this format:
   ```html
   <div class="changelog-entry" id="YYYY-MM-DD-slug">
     <div class="changelog-date">YYYY-MM-DD</div>
     <h2 class="changelog-title">short title</h2>
     <p class="changelog-desc">1-2 sentence summary of what changed.</p>
     <a class="changelog-prompt-link" href="/static/changelog/YYYY-MM-DD-NNN-slug.txt">read the prompt</a>
     <a class="changelog-permalink" href="/changelog#YYYY-MM-DD-slug">#</a>
   </div>
   ```
   - The `id` attribute enables direct linking: `anky.app/changelog#2026-02-14-video-studio`
   - Keep descriptions concise but specific about what shipped

3. **Do this at the end of every session**, right before the final build + deploy.

## Deployment

- Build: `cargo build --release`
- Restart: `systemctl --user restart anky.service`
- Always build and restart after changes unless told otherwise.

## Payments

- All paid features use x402 wallet payments (USDC on Base). No API key payment paths.
- Treasury address comes from config. Users send USDC, pass tx hash as `payment-signature` header.
