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

Backend ledger:

- `credit_ledger_entries` stores UI history only.
- RevenueCat `CREDITS` remains the source of truth for the balance.
- Welcome gifts use `POST /api/v1/credits/welcome-gift` and require bearer auth.
- Purchase history sync uses `POST /api/v1/credits/history/sync-purchase` after RevenueCat purchase success.
- Server-side RevenueCat virtual currency adjustments require `ANKY_REVENUECAT_PROJECT_ID` and `ANKY_REVENUECAT_SECRET_KEY`.

Expo Go cannot run native purchases. Test with EAS development builds or production builds. iOS uses `EXPO_PUBLIC_REVENUECAT_IOS_API_KEY`; Android can use `EXPO_PUBLIC_REVENUECAT_ANDROID_API_KEY` when configured.
