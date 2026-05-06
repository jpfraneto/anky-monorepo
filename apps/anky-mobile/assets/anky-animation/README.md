# Anky sprite pack

This pack was extracted from the spritesheet you uploaded.

## Contents

- `assets/anky/anky-sprite.png` — original spritesheet
- `assets/anky/atlas.json` — detected source rectangles and suggested animation sequences
- `assets/anky/frames/raw` — exact transparent crops
- `assets/anky/frames/normalized` — 240x240 transparent frames, centered/baseline-aligned for React Native
- `assets/anky/frames/contact-sheet.jpg` — visual index of all extracted frames
- `src/presence/ankyFrameAssets.ts` — static React Native `require(...)` map
- `src/presence/AnkySprite.tsx` — frame-swapping sprite component
- `src/presence/AnkyPresenceOverlayExample.tsx` — tiny global overlay example

## Suggested sequences

{
  "idle_front": [
    1,
    2,
    3,
    4,
    5,
    6
  ],
  "walk_right": [
    7,
    8,
    9,
    10,
    11,
    12,
    13,
    14,
    15,
    16,
    17,
    18,
    19,
    20,
    21,
    22
  ],
  "wave_front": [
    23,
    24,
    25,
    26
  ],
  "celebrate": [
    27,
    28,
    29,
    30,
    31
  ],
  "soft_concern_to_sleep": [
    32,
    33,
    34,
    35,
    36,
    37,
    38,
    39
  ],
  "idle_blink": [
    40,
    41,
    42,
    43,
    44,
    45
  ],
  "seated": [
    46,
    47,
    48,
    49,
    50,
    51
  ],
  "shy_listening": [
    52,
    53,
    54,
    55,
    56,
    57
  ]
}

## Install into the Expo app

From the root of `apps/anky-mobile`:

```bash
cp -R /path/to/this-pack/assets/anky ./assets/
cp -R /path/to/this-pack/src/presence ./src/
```

Then render the example once near your app root:

```tsx
<NavigationContainer>
  <AppNavigator />
  <AnkyPresenceOverlayExample isWriting={false} />
</NavigationContainer>
```

In the real app, replace `isWriting={false}` with route/session state so companion mode is suppressed during the active writing ritual.

## Product rule

Anky is a witness, not a mascot. Keep motion subtle. The frame animation gives life; the UI behavior gives dignity.
