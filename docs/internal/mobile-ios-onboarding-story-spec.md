# iOS Onboarding Story Spec

This replaces the old 8-card intro and the longer 7-beat awakening sequence.

The new onboarding should feel like a short playable rite:

- one character
- one world
- one mission

The user should not learn Anky through explanation.
They should meet it, accept the invitation, and write.

## Core Rule

The mission is:

`write for 8 minutes and wake anky up`

That is the onboarding.

Everything else is secondary.

## What Changes

Remove:

- the 8 chakra tutorial cards
- long explanatory copy
- separate tutorial energy before writing
- account-shaped UI before the first session
- lore that appears before the ritual has been felt

Keep:

- silent on-device identity
- the 8-minute writing rule
- the 8-second idle rule
- the feeling that the phone is the sacred place where identity lives

## Canonical Flow

The new first-run journey is:

1. the ember
2. the invitation
3. the trial
4. the arrival
5. the unlock

## 1. The Ember

Purpose:

- open like a story, not an app
- create attention before instruction

Visual:

- black screen
- one ember in the center
- soft pulse
- no chrome

Copy:

- `i've been waiting for you.`

Interaction:

- tap anywhere to continue
- auto-advance after 2 to 3 seconds

Notes:

- this should feel quiet, not dramatic
- no button yet
- no explanation yet

## 2. The Invitation

Purpose:

- state the mission in the clearest possible way

Visual:

- same world
- ember feels slightly more alive
- bottom dialogue box appears like a 2D narrative game

Recommended copy reveal:

1. `write for 8 minutes.`
2. `don't stop for 8 seconds.`
3. `reach the end and i wake up.`

CTA:

- `begin`

Notes:

- only one primary action
- no skip unless product insists
- if there is a skip, it still goes straight into writing

## 3. The Trial

Purpose:

- turn the invitation directly into the real writing session

Visual:

- same scene, now becoming the writing world
- dialogue box fades into the live writing HUD
- the screen should still feel like a world, not a blank editor

Rules taught by play:

- first printable character starts the session
- timer is visible
- lives are visible only when meaningful
- delete remains blocked
- after 3 seconds of inactivity, a life starts draining
- at 8 seconds, that life is lost

Recommended in-world reinforcement:

- after first life loss: `stay with it.`
- after resuming: `good. keep going.`
- after deeper momentum: `you're here. keep going.`

Notes:

- teach through consequence, not tooltips
- do not open any shell, profile, or story tab yet
- do not mention chakras or kingdoms here

## 4. The Arrival

Condition:

- only after the first real anky is persisted

Purpose:

- make the bond feel earned

Visual:

- ember is now clearly alive
- world warms
- subtle color at the edges

Copy:

- `you found me.`

Then immediately hand off into the post-writing response flow.

The first reflection should still begin with:

- `hey, thanks for being who you are. my thoughts:`

That lives in the reflection itself, not as onboarding chrome.

## 5. The Unlock

Condition:

- after the first real anky exists

Purpose:

- reveal that the app has opened because the ritual was completed

Visual:

- no giant reward screen
- short reveal, then transition into the normal app shell

Copy:

- `your first anky is alive.`

What opens now:

- profile
- written ankys
- reflection and conversation surfaces
- reminders
- later story surfaces

What stays hidden until now:

- broader profile UI
- archived history
- conversation threads
- anything that makes the app feel like a dashboard before it has been earned

## Recovery Phrase Recommendation

The current product can simplify dramatically by moving the backup ceremony later.

Recommended approach:

- generate identity silently on first open
- authenticate silently in the background
- do not interrupt the first writing mission with the recovery phrase
- present the recovery phrase ceremony after the first real anky, or on the next calm return

Why:

- the user first needs to feel why the app matters
- once they have a real anky, backup feels meaningful instead of bureaucratic

If the team wants a safer compromise:

- show the recovery ceremony immediately after the first successful reflection
- do not show it before the first writing session

## Copy Style

All onboarding copy should be:

- lowercase
- short
- intimate
- slightly mythical
- direct enough to act on

Avoid:

- therapy language
- product tutorial language
- crypto wallet language
- spiritual jargon
- long explanations

## The Real Shift

Old onboarding said:

- here are the rules
- here is the system
- now begin

New onboarding says:

- i am here
- come find me
- write

That is simpler, smoother, and much more memorable.
