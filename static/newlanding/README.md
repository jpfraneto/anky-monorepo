# ANKY new landing

Vanilla HTML/CSS/JS landing page served by the existing Rust/Axum app.

## Files

- Landing source lives in `static/newlanding/`
- Main page route is `/newlanding`
- Host-based route is `https://newlanding.anky.app/`

## Assets

- Replace the images in `static/newlanding/assets/`
- Expected filenames:
  - `anky-hero.png`
  - `app-loom.png`
  - `app-write.png`
  - `app-past.png`
- If store URLs change, update the button `href` values in `static/newlanding/index.html`

## Local test

Run the Rust server from the repo root:

```bash
cargo run
```

Then open:

```txt
http://localhost:8889/newlanding
```

To test the host-based branch locally:

```bash
curl -H 'Host: newlanding.anky.app' http://127.0.0.1:8889/
```

## Cloudflare note

If `newlanding.anky.app` points at the same server as `anky.app`, the existing subdomain middleware will serve this landing page at `/`. If DNS or the tunnel is not updated yet, `/newlanding` remains the canonical path to test and preview it.
