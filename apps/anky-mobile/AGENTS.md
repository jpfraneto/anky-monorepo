# AGENTS.md — Anky Mobile

This file is for coding agents working inside the Anky Expo React Native app.

Read this before changing code.

Anky is not a normal journaling app.
Anky is not a notes app.
Anky is not a chatbot.
Anky is not a productivity dashboard.
Anky is not a crypto minting app, even though Solana is part of the infrastructure.

Anky is a private 8-minute writing ritual.

The user opens the app, writes what is alive, sees only the latest glyph while writing, a subtle background with the full text and receives a trace of themselves as they write. There is an optional AI reflection by Anky afterwards.

The app should feel quiet, alive, local-first, sacred, and practical.

There is deep lore behind Anky — the loom, the thread, the witness, the map, the trace — but the app should not force users to understand the lore before they can write. The lore should appear as atmosphere, not homework.

---

## 1. Binding invariants

These are non-negotiable unless the user explicitly asks to change them.

### The app opens to Write

The app must always open to the Write surface.

Do not make onboarding, auth, map, credits, loom, or any other screen the initial required experience.

### Writing is always available

A user must be able to write without:

- logging in
- creating an account
- connecting a wallet
- minting a Loom
- buying credits
- understanding Solana
- completing onboarding
- configuring settings

Writing is the core action. Everything else orbits it.

### `.anky` is canonical

A writing session is serialized as a `.anky` file internally.

The `.anky` file is the canonical storage artifact of the session.

Do not modify the `.anky` protocol or storage format, ever.

User-facing language is stricter than storage language:

- 8-minute sessions are ankys.
- Sessions that end before 8 minutes are fragments or incomplete ankys.
- Fragments may be stored as `.anky` files internally, but the UI should not say they were "saved as an anky" or imply they are full ankys.

Other artifacts may exist as sidecars or metadata:

- reconstructed text
- reflection
- generated image
- keep-writing conversation
- seal receipt
- map/day status
- export metadata

But the `.anky` file remains the source of truth.

### Local-first by default

Writing stays on the user’s device by default.

Plaintext may leave the device only after the user explicitly chooses a processing action, such as:

- reflect
- keep writing with Anky
- future sync/export behavior, if implemented with clear consent

Do not silently upload plaintext writing.

### No plaintext onchain

Solana may be used for optional proof/sealing flows.

Onchain sealing must only publish or verify the session hash or proof data.

Never put plaintext writing onchain.

### The writing chamber has no app chrome

During active writing, the app should not feel like an app screen.

Do not add:

- header
- visible textarea
- regular timer text
- bottom navigation
- settings buttons
- explanatory labels
- generic app controls
- pressable UI that distracts from writing

The active chamber may show only what belongs to the ritual:

- native keyboard
- latest accepted glyph
- circular vessel / progress ring
- Anky as a quiet witness

### No global chatbot

Do not add a global Chat tab.

Do not add a global AI assistant.

Any “talk to Anky" or conversational flow must be anchored to a specific `.anky` session or fragment.

Anky is a witness, not an assistant.

### Loom and Solana are optional

The app must work fully as a writing practice without a Loom.

The absence of a Loom must never make the app feel broken.

Solana is part of Anky’s proof infrastructure, not the center of the user experience.

### Simplicity wins

Do not add complexity just because the lore supports it.

If a change adds friction before writing, it is probably wrong.

If a change makes the app feel like a dashboard, chatbot, productivity tool, crypto app, or social app, it is probably wrong.

---

## 2. Ideal product names

Use these as the user-facing product names.

Internal route names and file names may differ if changing them is unnecessary or risky.

Root areas:

```text
Write
Map
You

Core child surfaces:

Writing Chamber
Reveal
Day Chamber
Entry
Reflection
Keep Writing

Preferred action names:

begin
reflect
talk to Anky
copy
export
try again
go back
swipe to seal
back to entry

Avoid user-facing names like:

Chat
AI Assistant
Dashboard
Track
Thread
Seal Hash
Mint Page
XP
Leaderboard
```

Internal code may still use legacy route or type names when appropriate, but user-facing copy should follow the product language above.

## 3. Core product loop

The core loop is:

write → reveal → remember → reflect / keep writing → return to life
Write

The threshold into the ritual.

The app opens here.

The user should immediately understand:

i am here to write.
Writing Chamber

The active 8-minute writing ritual.

The chamber captures a .anky trace while showing only the latest glyph and the visual consequences of typing.

Reveal

The moment after the rite ends.

The user sees what came through and decides what to do with it.

Fragment sessions are valid artifacts, not failures, but they are not full ankys.

When Reveal is showing a fragment:

- make the incomplete status explicit
- avoid "your anky" and "saved as an anky" copy
- offer a clear way back out of Reveal
- make try again the primary action
- keep try again distinct from go back; try again starts a fresh chamber
- do not make reflection the primary action

Complete 8-minute sessions may be kept, reflected, continued, or optionally sealed.

Map

The user’s memory through time.

A temporal surface for revisiting days, entries, fragments, reflections, seals, and keep-writing conversations.

The Map may feel like a path, trail, or living calendar, but it should not become noisy gamification.

Entry

The readable detail view of a past .anky.

Reading is the main act.

Reflection

A mirror artifact generated from a specific .anky.

Usually consists of:

title
image
markdown reflection letter

The reflection should feel like Anky wrote the user a personal letter, not like a generic AI response.

Keep Writing

A contextual conversation/writing continuation with Anky.

It must always belong to a specific session hash.

It should help the user metabolize one trace and return to life, not create an infinite chatbot loop.

You

The user’s control room.

Summary, settings, privacy, credits, export, account, and optional Loom/seal state belong here.

You is important, but it is not the center of the product.

## 4. Navigation model

The app has three root areas:

Write
Map
You

Do not add more root tabs unless explicitly asked.

Bottom navigation is visible only on root screens:

Write
Map
You

Bottom navigation is hidden on immersive or child surfaces:

Writing Chamber
Reveal
Day Chamber
Entry
Reflection
Keep Writing
Auth sheets
You detail screens such as Account, Privacy, Export Data, Credits, and Loom Info

You detail screens should be pushed above the You root with native stack navigation, no bottom tab bar, and a subtle back control. They may start as placeholders while their final designs are still being shaped, but their routes should be real and typed.

There are two modes:

root mode = navigating the app
chamber mode = inside one specific artifact

Bottom navigation belongs only to root mode.

## 5. Writing chamber invariants

The Writing Chamber is the sacred surface of the app.

Behavioral invariants:

first accepted character starts the rite
accepted characters are written into .anky format
only the latest accepted glyph is visible while writing
the user cannot edit previous text during the rite
silence can end the rite
the 8-minute duration is represented visually, not as ordinary timer text
when the rite ends, the writing is reconstructed/revealed from the hidden session

The chamber should answer only:

what key was just pressed?
where did it go?
how did it change the field?
how much time has passed?
is Anky still witnessing me?

Anything else is noise.

Before the first accepted character, the chamber may show tiny native-feeling controls for leaving the chamber or surfacing a prompt.

Those controls must disappear once writing begins.

Prompt sparks should be one-line starts, not instructions or coaching.

Do not make the chamber decorative. Every visual element should belong to the ritual.

## 6. Reflection principles

Reflections are optional.

A reflection is not a summary.
A reflection is not therapy.
A reflection is not a productivity plan.
A reflection is not a generic LLM answer.

A reflection is Anky reading one .anky as a living trace.

A good reflection may notice:

emotional undercurrents
repeated patterns
contradictions
protective narratives
hidden longing
product/life connections
spiritual or existential themes
the deeper thing beneath scattered thoughts

Typos, jumps, repetitions, and emotional spikes are signal.

The output is usually:

title
image
markdown reflection

The markdown reflection should be allowed to breathe.

It may have headings.
It may be long.
It should feel personal, casual, profound, non-clinical, and precise.

If additional structured metadata is added, it must support the letter, not dominate it.

Reflection UI should be reading-first.

Avoid making Reflection feel like:

dashboard cards
chatbot bubbles
expandable content widgets
generic AI insight screens

The user should feel:

anky saw something in what i wrote.
## 7. Keep Writing principles

Keep Writing is not a global chat.

It is a contextual continuation of one .anky.

It should always be anchored to a sessionHash.

Good framing:

keep writing
keep writing with anky
i’m here with what you wrote. what still feels alive?

Bad framing:

chat with AI
ask anky anything
how can i help you today?

Keep Writing should feel closer to writing than chatting.

It should be intimate, bounded, and specific.

The goal is not endless engagement. The goal is metabolizing the trace.

## 8. Map principles

Map is memory made visible.

It may use a 96-day path, trail, or calendar-like progression.

The current mobile Map direction is the simplified Sojourn IX vertical map:

- days ascend from bottom to top
- day nodes sit on a simple center line
- each day uses the 8-color day cycle rather than long kingdom bands
- complete ankys create small glowing count dots on the left
- fragments are valid local artifacts, but they are not counted as complete ankys on the Map
- tapping a day selects it in the map panel
- tapping an anky row opens the local Entry route when a local `.anky` file exists
- the root Map keeps the shared bottom navigation; the map component itself does not own bottom nav

It should feel:

temporal
alive
spatial
quiet
practical
beautiful

It should not feel:

childish
noisy
competitive
gamified for its own sake
like a streak/XP product

The Map can show states such as:

future
empty
today
fragment
complete
reflected
sealed
keep-writing exists

But the exact visuals are directional, not binding.

The binding rule is:

Map helps the user revisit their traces through time.
## 9. Entry principles

Entry is where the user reads something they wrote.

It should prioritize:

readability
emotional respect
clear artifact status
practical actions

Possible actions:

copy
reflect
keep writing
export
view raw .anky
swipe to seal, if complete and sealable

Do not make Entry feel like a social post.

Do not make Entry push sharing.

Do not make raw protocol data more prominent than the writing itself.

## 10. Solana, Loom, and sealing

Solana is part of Anky’s optional proof layer.

The user does not need to understand Solana to write.

A Loom may allow the user to seal a daily hash or participate in a proof flow.

A Loom must never be required for writing.

Sealing should be framed as:

swipe to seal
hash only • local writing stays private

The writing flow must not become a Solana onboarding flow.

Avoid:

crypto-first language
mint pressure
wallet walls before writing
separate seal pages in the core ritual flow
implying that unsealed writing is incomplete

If sealing is unavailable, the app should still feel whole.

## 11. Privacy and consent

Default privacy posture:

your writing stays on this device

Plaintext may leave the device only after explicit action and clear consent.

Reflection and Keep Writing may require processing plaintext.

Before processing, the user should understand what is happening.

Good privacy copy:

your writing stays on this device unless you ask anky to reflect or keep writing with you.
sealing publishes only the hash. your writing remains private.

Do not use fear-based privacy copy.

Be clear, calm, and human.

## 12. Visual and copy taste

The visual language should feel like Anky:

dark indigo / obsidian
soft gold thread
violet glow
manuscript texture
circular vessels
woven paths
quiet cosmic atmosphere
Anky as witness, not mascot overload

Use magic as atmosphere, not clutter.

Avoid:

bright SaaS colors
generic dashboards
productivity charts
social feeds
loud gamification
heavy crypto visuals
generic chatbot UI
too many cards
too much ornament competing with readability

Copy should usually be lowercase, direct, warm, and spare.

Good copy:

write what is alive
quiet the mind. open the heart.
a fragment arrived
an incomplete anky arrived
saved as a fragment
your anky is complete
keep writing
reflect
try again
go back
swipe to seal
hash only • local writing stays private
what anky noticed
one thing to try
question to carry
your writing belongs to you

Avoid copy like:

unlock premium
connect wallet to continue
maximize your streak
saved as an anky, when the session ended before 8 minutes
share with followers
AI assistant
ask me anything
mint now
## 13. Technical surfaces to treat carefully

Before editing any of these areas, inspect current implementation and tests.

High-care surfaces:

.anky protocol generation/parsing
session hashing
local storage paths
active draft autosave
pending reveal state
sidecar metadata
reflection processing
keep-writing storage
seal receipts
Solana seal/mint flows
navigation initial route
Writing Chamber input capture

Do not rewrite these casually.

Prefer small, incremental changes.

Preserve working flows.

## 14. Testing commands

Common commands:

npm run test:protocol
npm test
npm run typecheck

Also inspect package.json for additional lint/check/test scripts.

Do not claim tests pass unless they were actually run.

If simulator/device verification is not possible, say so honestly.

## 15. Before finishing a task

Before reporting completion, check:

Does the app still open to Write?
Can the user still write without auth, wallet, Loom, or credits?
Did .anky behavior remain intact?
Did plaintext privacy remain intact?
Did any child screen accidentally show bottom navigation?
Did any root screen accidentally hide required navigation?
Did any copy turn Anky into a chatbot, dashboard, or crypto app?
Did any change add friction before writing?
Were relevant tests run?
## 16. Final taste test

Ask:

does this help the user return to the practice?

If yes, it probably belongs.

If it explains lore but does not help the practice, remove it or move it into a quieter surface.

If it adds friction before writing, remove it.

If it makes the chamber feel like an app screen, remove it.

If it makes the user feel managed instead of accompanied, rethink it.

Anky should feel like a small being holding a lantern at the edge of the user’s own mind.

Practical enough to save, revisit, export, reflect, and seal.

Magical enough that opening it feels like entering a room that was waiting for them.
