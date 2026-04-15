# Heart Flux Prompt Pack: iOS Onboarding

This prompt pack is for generating the new iOS onboarding art on Heart.

Goal:

- one visual world
- one companion
- one mission
- feels like a 2D narrative game, not a tutorial

The app should open like an invitation.
The user writes for 8 minutes.
That wakes Anky up.

## Recommendation

Start by testing 3 directions in parallel, then commit hard to one.

My recommendation:

- default winner to test first: `Dialogue Game`
- safest premium backup: `Ember Shrine`

If the team wants the strongest final result, use:

- `Dialogue Game` composition and character readability
- `Ember Shrine` color palette and emotional tone

That hybrid is likely the canonical direction.

## Production Rules

All directions should follow these rules:

- aspect ratio: `vertical 9:19.5`
- mobile-safe
- reserve bottom 26 to 32 percent for dialogue box or writing HUD
- no text in the artwork
- no literal app UI in the artwork
- no watermark
- no extra characters
- no clutter
- intimate, quiet, inviting
- loop-friendly

Shared avoid list:

- neon cyberpunk
- generic fantasy clutter
- childish mascot look
- chibi proportions
- mobile game ad energy
- loud sci-fi interface elements
- hard-saturated rainbow gradients

Suggested output:

- stills first
- PNG or lossless WebP masters
- animated WebP or short MP4 loops only after the still style is approved

Use GIF only for quick internal previews.
Do not make GIF the production format if you can avoid it.

## Iteration Workflow

1. Generate `Scene 1` across all 3 directions.
2. Review only for silhouette, mood, readability, and invitation quality.
3. Pick the best direction.
4. Lock that style and character shape.
5. Generate `Scenes 2 to 4` using the winning direction.
6. Generate motion plates or loop candidates only after the stills feel right.

Suggested first pass:

- 6 to 8 variants of `Scene 1` per direction
- pick top 2
- do one refinement round
- choose 1 style lock

## Shared Base Prompt

Use this as the common scaffold:

```text
Use case: illustration-story
Asset type: mobile onboarding scene
Primary request: a simplified Anky onboarding scene that feels like an invitation and a 2D narrative game, not a tutorial
Scene/backdrop: one dark intimate world with a small ember companion, strong center focus, clean bottom-safe area for dialogue UI
Subject: a living ember-being that feels ancient, kind, intimate, and slightly mysterious
Style/medium: hand-painted digital illustration, premium visual novel / story game quality
Composition/framing: vertical 9:19.5, mobile-safe, centered silhouette, clean readable space near the bottom
Lighting/mood: near-black and charcoal with warm ember orange and muted gold glow
Color palette: black, charcoal, ember orange, muted gold, restrained edge bloom only
Constraints: no text, no watermark, no extra characters, no clutter, no literal app UI
Avoid: neon cyberpunk, cartoon mascot style, generic fantasy clutter, mobile ad aesthetic, chibi proportions
```

## Direction 1: Ember Shrine

Art bible:

- painterly
- sacred
- storybook-like
- mythic but minimal
- premium and quiet

This is the most emotional direction.

### Scene 1: Dormant Ember

```text
Use case: illustration-story
Asset type: mobile onboarding opening scene
Primary request: a tiny living ember floating in a black void, soft orange glow, faint dust motes, intimate and sacred, waiting to be awakened
Scene/backdrop: near-black depth with minimal atmospheric texture
Subject: one small ember-being, centered, simple silhouette
Style/medium: hand-painted digital illustration, storybook, subtle magical realism
Composition/framing: vertical 9:19.5, large negative space, clean bottom-safe area
Lighting/mood: quiet, sacred, warm ember glow in darkness
Constraints: no text, no extra characters, no clutter, no UI
Avoid: neon, hard sci-fi, childish proportions, busy background
```

### Scene 2: Invitation

```text
Use case: illustration-story
Asset type: mobile onboarding invitation scene
Primary request: the ember brightens and starts to feel like a small presence speaking directly to the viewer
Scene/backdrop: same sacred dark world, slightly warmer
Subject: ember-being with a faint face-like suggestion, intimate and kind
Style/medium: hand-painted digital illustration, premium storybook feel
Composition/framing: vertical 9:19.5, dialogue-box-friendly lower area, centered character
Lighting/mood: quiet anticipation, ember orange and charcoal
Constraints: no text, no extra characters, no clutter, no UI
Avoid: exaggerated facial features, cartoon mascot look, busy scene dressing
```

### Scene 3: Awakened Writing World

```text
Use case: illustration-story
Asset type: mobile onboarding writing scene
Primary request: the void warms into a living chamber as if the world is responding to the user's writing
Scene/backdrop: soft gradients, flowing light, subtle word-like particles at the edges
Subject: ember-being now brighter, clearly affecting the world
Style/medium: hand-painted digital illustration
Composition/framing: vertical 9:19.5, clean center focus, bottom-safe for live writing HUD
Lighting/mood: warm, responsive, intimate, gently alive
Constraints: no text, no extra characters, no literal UI
Avoid: loud rainbow effects, clutter, heavy fantasy architecture
```

### Scene 4: Bonded Anky

```text
Use case: illustration-story
Asset type: mobile onboarding completion scene
Primary request: the ember has awakened into a small companion presence, calm and luminous, the bond is real and earned
Scene/backdrop: same world, now gently alive with warm edges and soft drift
Subject: bonded Anky, still intimate and simple
Style/medium: hand-painted digital illustration, tender storybook tone
Composition/framing: vertical 9:19.5, centered, strong silhouette, room for subtle overlay text later
Lighting/mood: warm, victorious without spectacle, emotionally resonant
Constraints: no text, no extra characters, no UI
Avoid: explosive celebration, fireworks, game reward screen energy
```

### Motion Notes

- ember pulse at 72 bpm
- very slow dust drift
- breathing glow
- soft edge bloom during writing

## Direction 2: Dialogue Game

Art bible:

- 2D narrative game
- visual novel / JRPG dialogue language
- clear silhouette
- playful but not childish
- easiest to understand instantly

This is the best fit if the goal is "a character talks to you".

### Scene 1: Dormant Ember

```text
Use case: illustration-story
Asset type: mobile onboarding opening scene
Primary request: a 2D narrative game scene with a small ember character in a dark hand-painted world, centered and readable
Scene/backdrop: moody dark world with soft shadowed depth and a clean dialogue-box-safe lower area
Subject: tiny ember character with a strong simple silhouette
Style/medium: hand-painted digital illustration, visual novel / narrative game
Composition/framing: vertical 9:19.5, centered, mobile-safe, character readability first
Lighting/mood: warm orange core glow against charcoal darkness, inviting not scary
Constraints: no text, no extra characters, no clutter, no UI
Avoid: over-detailed scenery, childish mascot style, neon game UI feel
```

### Scene 2: Invitation

```text
Use case: illustration-story
Asset type: mobile onboarding invitation scene
Primary request: the ember character leans forward slightly as if speaking to the player in a quiet 2D story game
Scene/backdrop: same narrative game world, slightly warmer, dialogue-box-friendly composition
Subject: ember companion, clear silhouette, alive and attentive
Style/medium: hand-painted visual novel style, premium and intimate
Composition/framing: vertical 9:19.5, eye-level feel, clean lower third, centered focal point
Lighting/mood: quiet anticipation, warm edge light, emotional closeness
Constraints: no text, no extra characters, no literal UI
Avoid: anime exaggeration, chibi, clutter, bright fantasy props
```

### Scene 3: Awakened Writing World

```text
Use case: illustration-story
Asset type: mobile onboarding writing scene
Primary request: the same 2D game world responding to writing, ember light spilling into the scene, floating glyph-like particles, subtle edge bloom
Scene/backdrop: handcrafted dark world becoming gently alive
Subject: ember companion brighter and more engaged
Style/medium: hand-painted digital illustration, elegant visual novel atmosphere
Composition/framing: vertical 9:19.5, center focus, bottom-safe for writing HUD, readable negative space
Lighting/mood: warm, responsive, concentrated, alive
Constraints: no text, no extra characters, no literal UI
Avoid: busy particle storms, over-saturated colors, game HUD baked into art
```

### Scene 4: Bonded Anky

```text
Use case: illustration-story
Asset type: mobile onboarding completion scene
Primary request: a small luminous companion fully awakened in a 2D narrative game world, the player and the being now clearly linked
Scene/backdrop: same world, warmer background gradients, calm and present
Subject: bonded Anky with a readable, iconic silhouette
Style/medium: premium visual novel / story game illustration
Composition/framing: vertical 9:19.5, centered, intimate scale, clean enough for UI overlay
Lighting/mood: gentle triumph, warmth, presence, emotional clarity
Constraints: no text, no extra characters, no literal UI
Avoid: boss-fight energy, flashy victory effects, childish cuteness
```

### Motion Notes

- slight ember bob
- one pulse or blink before dialogue
- dialogue box soft fade-in
- gentle 2-second push-in
- particle drift that can scale with typing speed

## Direction 3: Minimal Ritual

Art bible:

- restrained
- premium
- abstract
- elegant
- least risky

This is the cleanest direction, but also the least game-like.

### Scene 1: Dormant Ember

```text
Use case: illustration-story
Asset type: mobile onboarding opening scene
Primary request: a single ember suspended in near-black space, refined and minimal, premium and quiet
Scene/backdrop: abstract near-black field with subtle radial depth
Subject: one ember point with a soft halo
Style/medium: atmospheric digital illustration, high-end product mood
Composition/framing: vertical 9:19.5, large negative space, centered
Lighting/mood: calm, minimal, intimate
Constraints: no text, no extra characters, no clutter
Avoid: decorative fantasy elements, neon, hard sci-fi
```

### Scene 2: Invitation

```text
Use case: illustration-story
Asset type: mobile onboarding invitation scene
Primary request: the ember expands into a soft halo that suggests a presence speaking
Scene/backdrop: minimal abstract darkness with controlled glow
Subject: luminous halo around a central ember
Style/medium: premium atmospheric digital illustration
Composition/framing: vertical 9:19.5, one focal point, bottom-safe space
Lighting/mood: calm, elegant, gently intimate
Constraints: no text, no extra elements, no clutter
Avoid: strong figurative face, loud effects, fantasy scenery
```

### Scene 3: Awakened Writing World

```text
Use case: illustration-story
Asset type: mobile onboarding writing scene
Primary request: black space opening into layered warm gradients and faint luminous currents, as if thought is becoming visible
Scene/backdrop: abstract layered field, atmospheric depth
Subject: ember now stronger inside the field of light
Style/medium: minimalist atmospheric illustration
Composition/framing: vertical 9:19.5, elegant negative space, bottom-safe for writing HUD
Lighting/mood: refined, concentrated, warm, inward
Constraints: no text, no extra characters, no UI
Avoid: busy particles, narrative clutter, high-saturation rainbow effects
```

### Scene 4: Bonded Anky

```text
Use case: illustration-story
Asset type: mobile onboarding completion scene
Primary request: a completed luminous form emerging from the ember, still minimal but now clearly alive
Scene/backdrop: abstract warm field with subtle gold accents
Subject: bonded Anky as a calm luminous form
Style/medium: premium atmospheric illustration
Composition/framing: vertical 9:19.5, centered, restrained, elegant
Lighting/mood: quiet victory, premium, intimate
Constraints: no text, no extra elements, no UI
Avoid: explosive completion effects, theatrical fantasy imagery
```

### Motion Notes

- slow halo expansion
- subtle grain shimmer
- radial glow breathing
- minimal spark on completion

## Decision Rubric

Score each direction from 1 to 5 on:

1. immediate comprehension
2. emotional pull
3. dialogue readability
4. consistency across onboarding and post-write surfaces
5. motion safety

If there is a tie:

- choose the direction with the strongest character silhouette
- choose the direction that still looks good with no motion at all

## Loop Strategy

V1 recommendation:

- ship still plates plus procedural animation in Swift
- do not block on generated loops

V2 recommendation:

- generate short loops only for:
  - ember idle
  - invitation breathing
  - writing particle overlay
  - bonded glow

If Heart can generate loopable motion directly, use these as motion prompts:

```text
Create a subtle loopable onboarding motion plate for a small ember companion breathing softly in darkness, no camera shake, no dramatic movement, premium narrative game tone, seamless 2 to 4 second loop, no text, no UI
```

```text
Create a subtle loopable particle overlay for a dark intimate writing scene, tiny warm particles drifting gently outward from an ember light source, seamless 2 to 4 second loop, no text, transparent or near-black background
```

```text
Create a subtle loopable bonded glow plate for a small luminous companion in a dark world, warm edges, soft breathing light, premium story-game tone, seamless 2 to 4 second loop, no text, no UI
```

## Final Call

If the product goal is:

- clearest gameplay-like invitation: choose `Dialogue Game`
- strongest mythic feeling: choose `Ember Shrine`
- most minimal premium feel: choose `Minimal Ritual`

My recommendation for Anky right now is:

- `Dialogue Game` as the base
- with `Ember Shrine` warmth and seriousness

That gets you the clearest story, the strongest invitation, and the best chance of feeling memorable instead of generic.
