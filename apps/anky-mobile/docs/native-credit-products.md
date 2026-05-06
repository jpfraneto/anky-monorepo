# Native Credit Products

Credits are mobile consumables. Production iOS and Android builds sell them through RevenueCat, not Stripe Checkout.

Required App Store Connect consumables:

- `inc.anky.credits.22`
- `inc.anky.credits.88_bonus_11`
- `inc.anky.credits.333_bonus_88`

Required Google Play Console one-time products / consumables:

- `credits_22`
- `credits_88_bonus_11`
- `credits_333_bonus_88`

RevenueCat offering:

- identifier: `credits`
- packages: `starter`, `regular`, `sojourner`
- virtual currency code: `CREDITS`
- grants: 22, 99, and 421 credits respectively

Expo Go cannot run native purchases. Test with EAS development builds or production builds. iOS uses `EXPO_PUBLIC_REVENUECAT_IOS_API_KEY`; Android can use `EXPO_PUBLIC_REVENUECAT_ANDROID_API_KEY` when configured.
