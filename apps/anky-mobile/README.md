# anky-mobile

Phase 1 is a local .anky capture client. It is not a journaling app: no title, mood, prompt, category, markdown, editing, cloud, social, blockchain, or AI.

## Run On A MacBook

```bash
cd ~/anky/apps/anky-mobile
npm install
npm run test:protocol
npm run typecheck
npx expo start
```

Do not launch a simulator automatically unless you explicitly want one. Let Expo print the QR code and local URLs.

## Run On Poiesis

```bash
cd ~/anky/apps/anky-mobile
npm install
npm run test:protocol
npm run typecheck
npx expo start --tunnel
```

Use `--tunnel` when the phone cannot reach the same LAN address as the Poiesis machine.

## Test With Expo Go

1. Install Expo Go on the iOS or Android device.
2. Run `npx expo start` from `apps/anky-mobile`.
3. Scan the QR code with Expo Go.
4. Open **Write 8 Minutes**, type normally, then stop for 8 seconds to seal the file.
5. Open **Past Threads** to inspect saved local `.anky` files.

## Local Files

The app writes only to Expo FileSystem local documents storage:

```text
FileSystem.documentDirectory/anky/
```

Active mutable draft:

```text
active.anky.draft
```

Sealed immutable sessions:

```text
{sha256_raw_utf8_bytes}.anky
```

With Expo Go, this directory is inside Expo Go's app sandbox. The draft is recoverable after a restart if it does not yet contain terminal `8000`, but it is not treated as a sealed session. A final `{hash}.anky` file is saved only after terminal `8000` has been appended.

## Logo And Splash

The raster source logo lives in:

```text
assets/anky-logo-source.png
```

Expo consumes the exported PNGs:

```text
assets/icon.png
assets/adaptive-icon.png
assets/splash-icon.png
assets/favicon.png
```

After replacing `anky-logo-source.png`, regenerate the PNGs with:

```bash
magick assets/anky-logo-source.png -resize 1024x1024^ -gravity center -extent 1024x1024 -strip assets/icon.png
magick -size 1024x1024 canvas:none \( assets/anky-logo-source.png -resize 850x850 \) -gravity center -compose over -composite -strip assets/adaptive-icon.png
magick -size 1024x1024 canvas:none \( assets/anky-logo-source.png -resize 720x720 \) -gravity center -compose over -composite -strip assets/splash-icon.png
magick assets/anky-logo-source.png -resize 48x48^ -gravity center -extent 48x48 -strip assets/favicon.png
```
