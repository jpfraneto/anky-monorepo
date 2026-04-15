# iOS Narrative-Game Onboarding Implementation Spec

This document extends:

- [mobile-ios-now-seed-spec.md](/home/kithkui/anky/docs/internal/mobile-ios-now-seed-spec.md)
- [IOS_PROMPT_POST_WRITING_FLOW.md](/home/kithkui/anky/IOS_PROMPT_POST_WRITING_FLOW.md)

It defines the simplified first-run flow that turns onboarding into one playable narrative scene.

## Product Goal

Make the app feel like:

- a character is speaking to you
- the phone is quietly holding your identity
- writing for 8 minutes is the rite that opens the rest of the app

Do not make first run feel like:

- a signup funnel
- a tutorial stack
- a wallet setup flow
- a normal text editor

## Product Rules

- the first real anky unlocks the full shell
- short sessions stay local and do not unlock the shell
- recovery phrase ceremony should not interrupt the first writing mission
- the same visual world should carry through intro, writing, and completion

## Recommended Architecture

Use SwiftUI for v1.

Do not build a separate game engine for this.

Preferred stack:

- `ZStack` scene composition
- `TimelineView`, `PhaseAnimator`, or `KeyframeAnimator` for subtle loops
- layered background plates from Heart
- lightweight particles in Swift, not baked into every asset
- static image plates first, optional animated WebP or short MP4 loops later

Avoid spending the first pass on GIF-heavy implementation.
For production, animated WebP or MP4 is better than GIF.
GIF is acceptable only for quick internal previews.

## App Gate

The root app flow should decide between:

```swift
enum RootFlow {
  case boot
  case onboarding(OnboardingPhase)
  case unlocked
}

enum OnboardingPhase {
  case ember
  case invitation
  case writing
  case shortSessionEnd
  case completionReveal
  case recoveryCeremony
}
```

Recommended behavior:

- if `firstMintComplete == true`, route directly to `.unlocked`
- otherwise route into `.onboarding(.ember)`
- if the user already started a first-run writing session, restore `.onboarding(.writing)`

## Local State

Store in `UserDefaults`:

- `onboardingVersion`
- `hasSeenGameIntro`
- `firstMintComplete`
- `lastOnboardingPhase`
- `pendingOnboardingSessionId`
- `hasBackedUpPhrase`
- `hasLocalIdentity`
- `hasAskedForNotifications`

Store in Keychain:

- private key / seed-derived signing material
- backend bearer session if app policy keeps it secure there

## Silent Identity

On first open:

1. create local identity
2. store it in Keychain
3. silently authenticate against `/swift/v2/auth/*`
4. move directly into onboarding

Do not block on profile creation.
Do not block on email.
Do not block on the recovery phrase ceremony.

## View Tree

Suggested top-level structure:

- `OnboardingCoordinatorView`
- `OnboardingSceneView`
- `DialogueOverlayView`
- `OnboardingWriteContainerView`
- `CompletionRevealView`
- `RecoveryCeremonyView`

### `OnboardingCoordinatorView`

Responsibilities:

- reads local flags
- restores interrupted first-run state
- routes between onboarding phases and unlocked shell
- listens for first persisted anky success

### `OnboardingSceneView`

One reusable scene renderer for all onboarding phases.

Inputs:

- background plate
- ember state
- edge bloom intensity
- particle intensity
- dialogue visibility

This is important: do not rebuild the whole world for every step.
One world changing state is what makes it feel like a game.

### `DialogueOverlayView`

Responsibilities:

- dialogue box layout
- typewriter text reveal
- primary CTA
- tap-to-advance behavior

Dialogue should support localization and dynamic type.

### `OnboardingWriteContainerView`

Responsibilities:

- wraps the existing 8-minute writing engine
- keeps the world art in place
- overlays timer, lives, idle drain, and writing UI
- starts the session on the first printable character

Do not fork the existing writing logic more than necessary.
Reuse the same writing model already specified in the NOW spec.

### `CompletionRevealView`

Shown only after `persisted: true` for the first real anky.

Responsibilities:

- brief bond moment
- visual warm-up
- short copy reveal
- route into the post-writing reflection flow

Keep this short. It should feel earned, not gamified.

## Asset Contract

Ask Heart for a small reusable asset set, not dozens of onboarding screens.

Required scene assets:

- `onboarding_void_bg`
- `onboarding_invitation_bg`
- `onboarding_writing_bg`
- `onboarding_bonded_bg`

Required character assets:

- `ember_idle`
- `ember_speaking`
- `ember_awake`

Optional overlays:

- `dust_overlay`
- `particle_overlay`
- `edge_bloom_mask`
- `glyph_crack_mask`

Safe-area rule:

- bottom 26 to 32 percent of the screen must stay readable under a dialogue box or input HUD

Delivery recommendation:

- stills as PNG or lossless WebP
- loops as animated WebP or short MP4 if motion is truly needed
- keep an `@2x` export for asset review and a production export for the app

## Motion Contract

Allowed motion:

- ember pulse
- slow drift of dust or particles
- very subtle parallax between background and ember
- dialogue box fade/slide
- gentle camera push during the invitation
- color bloom as writing deepens
- glyph fracture during idle drain

Reduce Motion:

- replace pulse with opacity change
- replace parallax with cross-dissolve
- disable particle drift
- keep timing identical so the flow still works

## Narrative Timing

### Phase 1: Ember

- black scene
- one ember
- one line: `i've been waiting for you.`
- advance on tap or after 2 to 3 seconds

### Phase 2: Invitation

Typewriter reveal:

1. `write for 8 minutes.`
2. `don't stop for 8 seconds.`
3. `reach the end and i wake up.`

Primary CTA:

- `begin`

### Phase 3: Writing

Rules:

- first printable character starts the timer
- first character must never be dropped
- delete stays blocked
- idle drain starts after 3 seconds
- life is lost at 8 seconds

In-world reinforcement lines are optional:

- `stay with it.`
- `good. keep going.`
- `you're here. keep going.`

### Phase 4: Short Session End

If the user fails to complete a real anky:

- stay in the locked world
- do not show reflection
- do not show chat
- do not show unlocked shell

Only show:

- `not yet.`
- subtle `try again`
- small `(8 minutes)` label

### Phase 5: Completion Reveal

If the user completes a real anky and backend returns `persisted: true`:

- show bonded ember state
- line: `you found me.`
- brief pause
- enter the existing post-writing response flow

After that:

- set `firstMintComplete = true`
- route to unlocked shell after dismissal

### Phase 6: Recovery Ceremony

Recommended timing:

- after the first successful reflection, or
- on the next calm app open

Do not place this before the first writing mission.

## API Integration

Reuse existing endpoints and behavior:

- silent auth: `/swift/v2/auth/*`
- writing submit: `/swift/v2/write`
- post-write polling: `/swift/v2/writing/{sessionId}/status`

Unlock condition:

- only when the first real session returns `persisted: true`

If outcome is short session:

- keep everything local
- remain in onboarding shell

## Localization Keys

At minimum:

- `onboarding.ember.waiting`
- `onboarding.invitation.line1`
- `onboarding.invitation.line2`
- `onboarding.invitation.line3`
- `onboarding.begin`
- `onboarding.short.notYet`
- `onboarding.short.tryAgain`
- `onboarding.short.durationHint`
- `onboarding.complete.foundYou`
- `onboarding.complete.firstAnkyAlive`

All copy should stay lowercase in the source strings unless a given language truly needs otherwise.

## Accessibility

- all tap targets 44x44 pt or larger
- VoiceOver order: scene -> dialogue -> CTA
- dialogue box must survive Dynamic Type expansion
- contrast must remain readable over dark art
- motion must respect Reduce Motion
- haptics must be optional and quiet

## QA Checklist

- first open creates identity silently
- first run shows ember, not a tutorial stack
- `begin` lands in the real writing scene
- first typed character starts the session and is never dropped
- short session does not unlock the shell
- short session shows only try-again state
- first persisted real anky triggers completion reveal
- completion reveal routes into the response flow
- `firstMintComplete` opens the full shell on next app launch
- recovery ceremony can be shown later without blocking writing
- device language localizes the onboarding copy

## Implementation Order

1. build `RootFlow` and `OnboardingPhase`
2. replace intro cards with the ember and invitation phases
3. bridge invitation directly into the existing writing engine
4. wire `persisted: true` into `completionReveal`
5. gate the unlocked shell behind `firstMintComplete`
6. add delayed recovery phrase ceremony
7. swap placeholder art for Heart-generated assets
8. add motion polish only after the state machine is stable
