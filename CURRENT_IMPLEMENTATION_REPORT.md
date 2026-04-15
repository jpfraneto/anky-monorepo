# Current Implementation State Report

This report is based on the current source tree under `/home/kithkui/anky`, plus the live Postgres schema reachable from the local `.env` configuration. I used source as the ground truth for runtime behavior and `information_schema` as the ground truth for the current database shape.

High-level counts:
- Axum router entries: 268 method/path combinations, plus 13 static `nest_service` mounts.
- Live Postgres schema: 72 public tables, 665 columns, 33 declared foreign-key edges.
- Important caveat: some docs and comments do **not** match runtime behavior. `SOUL.md` is aspirational; the running system stores writings server-side and sends some of them to cloud APIs.

Primary source files:
- [routes/mod.rs](/home/kithkui/anky/src/routes/mod.rs)
- [routes/auth.rs](/home/kithkui/anky/src/routes/auth.rs)
- [routes/swift.rs](/home/kithkui/anky/src/routes/swift.rs)
- [routes/writing.rs](/home/kithkui/anky/src/routes/writing.rs)
- [routes/session.rs](/home/kithkui/anky/src/routes/session.rs)
- [services/claude.rs](/home/kithkui/anky/src/services/claude.rs)
- [pipeline/image_gen.rs](/home/kithkui/anky/src/pipeline/image_gen.rs)
- [services/redis_queue.rs](/home/kithkui/anky/src/services/redis_queue.rs)
- [config.rs](/home/kithkui/anky/src/config.rs)
- [solana/worker/src/index.ts](/home/kithkui/anky/solana/worker/src/index.ts)

## 1. Authentication

Current auth is provider-linked and session-token based. There is no password auth, no password hash column, and no first-party email/password login flow.

Current auth entrypoints:
- Web X OAuth: `GET /auth/x/login` and `GET /auth/x/callback` in [auth.rs](/home/kithkui/anky/src/routes/auth.rs). The server stores PKCE state in `oauth_states`, exchanges the code with X, upserts the linked X account into `x_users`, creates an `auth_sessions` row, and sets web cookies.
- Web Farcaster miniapp auth: `POST /auth/farcaster/verify` in [auth.rs](/home/kithkui/anky/src/routes/auth.rs). This trusts the supplied `fid` from miniapp context, links or creates a `users` row, and creates the same `auth_sessions` session token used elsewhere.
- Mobile Privy auth: `POST /swift/v1/auth/privy` in [swift.rs](/home/kithkui/anky/src/routes/swift.rs). The server verifies the Privy JWT locally when `PRIVY_VERIFICATION_KEY` exists, otherwise via Privy API, then links `users.privy_did`, `users.email`, and `users.wallet_address` when available.
- Mobile seed auth: `POST /swift/v2/auth/challenge` and `POST /swift/v2/auth/verify` in [swift.rs](/home/kithkui/anky/src/routes/swift.rs). This supports Solana Ed25519 signatures and legacy EVM `personal_sign` signatures. Challenges are stored in `auth_challenges`; successful verification creates or finds a user by wallet and then creates an `auth_sessions` row.
- QR-based seal auth: `POST /api/auth/qr`, `GET /api/auth/qr/{id}`, `POST /api/auth/qr/{id}/seal`, `POST /api/auth/qr/seal` in [qr_auth.rs](/home/kithkui/anky/src/routes/qr_auth.rs). This is not a general login flow; it supports phone-to-web sealing.

Web session/cookie behavior:
- Primary session cookie: `anky_session`.
- Anonymous visitor cookie: `anky_user_id`.
- Session lookup always resolves through `auth_sessions`.
- Mobile bearer tokens are the same `auth_sessions.token` values; mobile just sends them in `Authorization: Bearer <token>`.

What is stored about users in Postgres:
- Core identity is in `users`.
- Provider linkages and session state are spread across `x_users`, `auth_sessions`, `oauth_states`, `auth_challenges`, `qr_auth_challenges`, `farcaster_wallets`, `device_tokens`, and `farcaster_notification_tokens`.
- X OAuth tokens are stored server-side in `x_users.access_token`, `x_users.refresh_token`, and `x_users.token_expires_at`.
- Mobile/session tokens are stored in plaintext in `auth_sessions.token`.
- Web anonymous users can also get a generated custodial wallet; its secret is stored in `users.generated_wallet_secret`.

Current `users` schema:
- `id text primary key`
- `created_at text default anky_now()`
- `username text unique nullable`
- `wallet_address text unique nullable`
- `privy_did text nullable`
- `farcaster_fid integer nullable`
- `farcaster_username text nullable`
- `farcaster_pfp_url text nullable`
- `email text nullable`
- `is_premium integer default 0`
- `premium_since text nullable`
- `generated_wallet_secret text nullable`
- `wallet_generated_at text nullable`
- `is_pro integer default 0`

Auth-related tables that matter in practice:
- `x_users`: X user id, local user id, username/profile, access token, refresh token, token expiry.
- `auth_sessions`: bearer/web session token, local user id, optional X user id, expiry.
- `oauth_states`: OAuth state + PKCE verifier + optional redirect.
- `auth_challenges`: wallet auth challenge text + expiry + consumed timestamp.
- `qr_auth_challenges`: QR token, optional Solana address, sealed/session linkage, expiry.
- `farcaster_wallets`: miniapp wallet custody/onboarding state for Farcaster-specific flows.

Important implementation details:
- `users.is_premium` exists, but it is not tied to a billing system.
- `users.is_pro` exists and is used for GPU queue priority, not for auth.
- There is no RBAC framework or auth middleware layer. Auth is mostly enforced inside handlers.

Representative mobile seed auth flow:

```rust
pub async fn auth_seed_verify_inner(
    state: &AppState,
    req: &SeedAuthVerifyRequest,
) -> Result<AuthResponse, AppError> {
    let wallet_address = normalize_seed_wallet_address(&req.wallet_address)?;
    let challenge = queries::get_active_auth_challenge(&db, &req.challenge_id)?;
    verify_seed_auth_signature(&wallet_address, &challenge.challenge_text, &req.signature)?;
    let user_id = if let Some(existing) = queries::get_user_by_wallet(&db, &wallet_address)? {
        existing
    } else {
        let uid = uuid::Uuid::new_v4().to_string();
        queries::create_user_with_wallet(&db, &uid, &wallet_address)?;
        uid
    };
    queries::consume_auth_challenge(&db, &req.challenge_id)?;
    queries::create_auth_session(&db, &session_token, &user_id, None, &expires_at)?;
}
```

## 2. API Routes

Global Axum middleware on the main router in [routes/mod.rs](/home/kithkui/anky/src/routes/mod.rs):
- `CompressionLayer::new()`
- global `CorsLayer`
- `RequestBodyLimitLayer::new(256 * 1024)` for the default 256KB body limit
- `security_headers::security_headers`
- `honeypot::honeypot_and_attack_detection`
- `subdomain::pitch_subdomain` even though it also handles `mirror.anky.app` and `ankycoin.com`
- `.with_state(state)`

Route-group-specific middleware/layers:
- `/api/v1/generate`, `/api/v1/prompt*`, `/api/v1/transform`, `/api/v1/balance`: `optional_api_key`
- `/api/v1/studio/upload`: 512MB body limit
- `/api/v1/media-factory/*`: 20MB body limit
- `/swift/*` group: mobile CORS allowing any origin with `GET, POST, DELETE, OPTIONS`
- Static mounts `/data/images` and `/data/anky-images`: immutable cache header override

Handler-level auth patterns that are **not** Axum middleware:
- Web cookie auth: `anky_session` / `anky_user_id`
- Mobile bearer auth: `Authorization: Bearer <auth_sessions.token>`
- Agent auth: `X-API-Key`, validated inside handlers

Exhaustive route inventory derived from the current router and handler signatures:

```text
PATH | METHOD | HANDLER | REQUEST BODY | RESPONSE TYPE | EXTRA
--- | --- | --- | --- | --- | ---
/api/v1/generate | POST | api::generate_anky_paid | JSON PaidGenerateRequest | Result<Response, AppError> | optional_api_key middleware
/api/v1/prompt | POST | prompt::create_prompt_api | JSON CreatePromptRequest | Result<Response, AppError> | optional_api_key middleware
/api/v1/prompt/create | POST | prompt::create_prompt_api | JSON CreatePromptRequest | Result<Response, AppError> | optional_api_key middleware
/api/v1/prompt/quick | POST | prompt::create_prompt_quick | JSON CreatePromptRequest | Result<Json<serde_json::Value>, AppError> | optional_api_key middleware
/api/v1/studio/upload | POST | api::upload_studio_video | Multipart form-data | Result<Json<serde_json::Value>, AppError> | 512MB body limit
/api/v1/media-factory/video | POST | api::media_factory_video | JSON serde_json::Value | Result<Response, AppError> | 20MB body limit
/api/v1/media-factory/image | POST | api::media_factory_image | JSON serde_json::Value | Result<Response, AppError> | 20MB body limit
/api/v1/media-factory/flux | POST | api::media_factory_flux | JSON serde_json::Value | Result<Response, AppError> | 20MB body limit
/api/v1/transform | POST | extension_api::transform | JSON TransformRequest | Result<Json<TransformResponse>, AppError> | optional_api_key middleware
/api/v1/balance | GET | extension_api::balance | None | Result<Json<BalanceResponse>, AppError> | optional_api_key middleware
/swift/v1/auth/privy | POST | swift::auth_privy | JSON PrivyAuthRequest | Result<Json<AuthResponse>, AppError> | mobile CORS
/swift/v2/auth/challenge | POST | swift::auth_seed_challenge | JSON SeedAuthChallengeRequest | Result<Json<SeedAuthChallengeResponse>, AppError> | mobile CORS
/swift/v2/auth/verify | POST | swift::auth_seed_verify | JSON SeedAuthVerifyRequest | Result<Json<AuthResponse>, AppError> | mobile CORS
/swift/v1/auth/session | DELETE | swift::auth_logout | None | Result<Json<serde_json::Value>, AppError> | mobile CORS
/swift/v2/auth/session | DELETE | swift::auth_logout | None | Result<Json<serde_json::Value>, AppError> | mobile CORS
/swift/v1/me | GET | swift::get_me | None | Result<Json<MeResponse>, AppError> | mobile CORS
/swift/v2/me | GET | swift::get_me | None | Result<Json<MeResponse>, AppError> | mobile CORS
/swift/v1/writings | GET | swift::list_writings | None | Result<Json<Vec<WritingItem>>, AppError> | mobile CORS
/swift/v2/writings | GET | swift::list_writings | None | Result<Json<Vec<WritingItem>>, AppError> | mobile CORS
/swift/v1/write | POST | swift::submit_writing_unified | JSON MobileWriteRequest | Result<Json<MobileWriteResponse>, AppError> | mobile CORS
/swift/v2/write | POST | swift::submit_writing_unified | JSON MobileWriteRequest | Result<Json<MobileWriteResponse>, AppError> | mobile CORS
/swift/v2/writing/{sessionId}/status | GET | swift::get_writing_status | None | Result<Json<WritingStatusResponse>, AppError> | mobile CORS
/swift/v2/children | GET | swift::list_children | None | Result<Json<Vec<ChildProfileItem>>, AppError> | mobile CORS
/swift/v2/children | POST | swift::create_child_profile | JSON CreateChildProfileRequest | Result<Json<ChildProfileItem>, AppError> | mobile CORS
/swift/v2/children/{childId} | GET | swift::get_child_profile | None | Result<Json<ChildProfileItem>, AppError> | mobile CORS
/swift/v2/cuentacuentos/ready | GET | swift::cuentacuentos_ready | None | Result<Json<Option<CuentacuentosItem>>, AppError> | mobile CORS
/swift/v2/cuentacuentos/history | GET | swift::cuentacuentos_history | None | Result<Json<Vec<CuentacuentosItem>>, AppError> | mobile CORS
/swift/v2/cuentacuentos/{id}/complete | POST | swift::complete_cuentacuentos | None | Result<Json<serde_json::Value>, AppError> | mobile CORS
/swift/v2/cuentacuentos/{id}/assign | POST | swift::assign_cuentacuentos | JSON AssignCuentacuentosRequest | Result<Json<CuentacuentosItem>, AppError> | mobile CORS
/swift/v2/prompt/{id} | GET | swift::get_prompt_by_id | None | Result<Json<serde_json::Value>, AppError> | mobile CORS
/swift/v2/next-prompt | GET | swift::get_next_prompt | None | Result<Json<NextPromptResponse>, AppError> | mobile CORS
/swift/v2/chat/prompt | GET | swift::get_chat_prompt | None | Result<Json<ChatPromptResponse>, AppError> | mobile CORS
/swift/v2/you | GET | swift::get_you | None | Result<Json<YouResponse>, AppError> | mobile CORS
/swift/v2/you/ankys | GET | swift::get_you_ankys | None | Result<Json<Vec<YouAnkyItem>>, AppError> | mobile CORS
/swift/v2/you/items | GET | swift::get_you_items | None | Result<Json<serde_json::Value>, AppError> | mobile CORS
/swift/v2/mirror/mint | POST | swift::swift_mirror_mint | JSON serde_json::Value | Result<Json<serde_json::Value>, AppError> | mobile CORS
/swift/v2/device-token | POST | swift::register_device | JSON RegisterDeviceRequest | Result<Json<serde_json::Value>, AppError> | mobile CORS
/swift/v2/devices | POST | swift::register_device | JSON RegisterDeviceRequest | Result<Json<serde_json::Value>, AppError> | mobile CORS
/swift/v2/devices | DELETE | swift::delete_device | JSON DeleteDeviceRequest | Result<Json<serde_json::Value>, AppError> | mobile CORS
/swift/v2/settings | GET | swift::get_settings | None | Result<Json<MobileSettingsResponse>, AppError> | mobile CORS
/swift/v2/settings | PATCH | swift::patch_settings | JSON PatchSettingsRequest | Result<Json<MobileSettingsResponse>, AppError> | mobile CORS
/swift/v2/writing/{sessionId}/prepare-mint | POST | swift::prepare_mint | None | Result<Json<PrepareMintResponse>, AppError> | mobile CORS
/swift/v2/writing/{sessionId}/confirm-mint | POST | swift::confirm_mint | JSON ConfirmMintRequest | Result<Json<ConfirmMintResponse>, AppError> | mobile CORS
/swift/v2/mint-mirror | POST | swift::mint_raw_mirror | JSON serde_json::Value | Result<Json<serde_json::Value>, AppError> | mobile CORS
/mirror/mint | POST | swift::mint_raw_mirror | JSON serde_json::Value | Result<Json<serde_json::Value>, AppError> | mobile CORS
/swift/v2/sealed-sessions | GET | sealed::list_sealed_sessions | None | Result<Json<SealedSessionsListResponse>, AppError> | mobile CORS
/swift/v1/admin/premium | POST | swift::set_premium | JSON serde_json::Value | Result<Json<serde_json::Value>, AppError> | mobile CORS
/ | GET | pages::home | None | Result<(CookieJar, Html<String>), AppError> | 
/altar | GET | pages::altar_page | None | Html<String> | 
/write | GET | pages::write_page | None | Result<(CookieJar, Html<String>), AppError> | 
/stories | GET | pages::stories_page | None | Result<Html<String>, AppError> | 
/ankys | GET | pages::ankys_page | None | Result<Html<String>, AppError> | 
/you | GET | pages::you_page | None | Result<Html<String>, AppError> | 
/test | GET | pages::test_page | None | Result<Html<String>, AppError> | 
/gallery | GET | pages::gallery | None | Result<Html<String>, AppError> | 
/gallery/dataset-round-two | GET | pages::dataset_round_two | None | Result<Html<String>, AppError> | 
/gallery/dataset-round-two/og-image | GET | pages::dataset_og_image | None | axum::response::Response | 
/gallery/dataset-round-two/eliminate | POST | pages::dataset_eliminate | Form EliminateForm | axum::response::Response | 
/video-gallery | GET | pages::videos_gallery | None | Result<Html<String>, AppError> | 
/feed | GET | pages::feed_page | None | Result<Html<String>, AppError> | 
/help | GET | pages::help | None | Result<Html<String>, AppError> | 
/mobile | GET | pages::mobile | None | Result<Html<String>, AppError> | 
/dca | GET | pages::dca_dashboard | None | Result<Html<String>, AppError> | 
/dca-bot-code | GET | pages::dca_bot_code | None | Result<Html<String>, AppError> | 
/login | GET | pages::login_page | None | Html<String> | 
/seal | GET | pages::seal_bridge_page | None | Result<Html<String>, AppError> | 
/ankycoin | GET | pages::ankycoin_page | None | Result<Html<String>, AppError> | 
/leaderboard | GET | pages::leaderboard | None | Result<Html<String>, AppError> | 
/pitch | GET | pages::pitch | None | Result<Html<String>, AppError> | 
/generate | GET | pages::generate_page | None | Result<Html<String>, AppError> | 
/create-videos | GET | pages::create_videos_page | None | Result<Html<String>, AppError> | 
/generate/video | GET | pages::video_dashboard | None | Result<Html<String>, AppError> | 
/video/pipeline | GET | pages::video_pipeline_page | None | Result<Html<String>, AppError> | 
/video-dashboard | GET | pages::media_dashboard | None | Result<Html<String>, AppError> | 
/sleeping | GET | pages::sleeping | None | Result<Html<String>, AppError> | 
/feedback | GET | pages::feedback | None | Result<Html<String>, AppError> | 
/changelog | GET | pages::changelog | None | Result<Html<String>, AppError> | 
/easter | GET | pages::easter_gallery | None | Result<Html<String>, AppError> | 
/classes | GET | pages::classes_index | None | Result<Html<String>, AppError> | 
/classes/{number} | GET | pages::class_page | None | Result<Html<String>, AppError> | 
/simulations | GET | simulations::simulations_page | None | Result<Html<String>, AppError> | 
/api/simulations/slots | GET | simulations::slots_status | None | Json<SlotsResponse> | 
/api/simulations/slots/stream | GET | simulations::slots_stream | None | Sse<impl Stream<Item = Result<Event, Infallible>>> | 
/api/simulations/slots/demo | POST | simulations::slots_demo | None | Result<Json<DemoResponse>, AppError> | 
/llm | GET | pages::llm | None | Result<Html<String>, AppError> | 
/pitch-deck | GET | pages::pitch_deck | None | Result<Html<String>, AppError> | 
/pitch-deck.pdf | GET | pages::pitch_deck_pdf | None | Result<impl axum::response::IntoResponse, AppError> | 
/api/v1/llm/training-status | POST | api::llm_training_status | JSON serde_json::Value | Result<Json<serde_json::Value>, AppError> | 
/api/v1/classes/generate | POST | api::generate_class | JSON serde_json::Value | Response | 
/anky/{id} | GET | pages::anky_detail | None | Result<Html<String>, AppError> | 
/story/{story_id} | GET | voices::story_deep_link_page | None | Result<Response, AppError> | 
/api/og/write | GET | api::og_write_svg | None | axum::response::Response<axum::body::Body> | 
/prompt | GET | prompt::prompt_new_page | None | Result<Html<String>, AppError> | 
/prompt/create | GET | prompt::create_prompt_page | None | Result<Html<String>, AppError> | 
/prompt/{id} | GET | prompt::prompt_page | None | Result<Html<String>, AppError> | 
/api/v1/prompt/{id} | GET | prompt::get_prompt_api | None | Result<Json<serde_json::Value>, AppError> | 
/api/v1/prompt/{id}/write | POST | prompt::submit_prompt_writing | JSON SubmitWritingRequest | Result<Json<serde_json::Value>, AppError> | 
/api/v1/prompts | GET | prompt::list_prompts_api | None | Result<Json<serde_json::Value>, AppError> | 
/api/v1/prompts/random | GET | prompt::random_prompt_api | None | Result<Json<serde_json::Value>, AppError> | 
/settings | GET | settings::settings_page | None | Result<Html<String>, AppError> | 
/api/settings | POST | settings::save_settings | JSON SaveSettingsRequest | Result<Json<serde_json::Value>, AppError> | 
/api/claim-username | POST | settings::claim_username | JSON ClaimUsernameRequest | Result<Json<serde_json::Value>, AppError> | 
/auth/x/login | GET | auth::login | None | Result<Redirect, AppError> | 
/auth/x/callback | GET | auth::callback | None | Result<(CookieJar, Redirect), AppError> | 
/auth/x/logout | GET | auth::logout | None | Result<(CookieJar, Redirect), AppError> | 
/auth/logout | POST | auth::logout_json | None | Result<(CookieJar, Json<serde_json::Value>), AppError> | 
/auth/farcaster/verify | POST | auth::farcaster_verify | JSON FarcasterVerifyRequest | Result<(CookieJar, Json<serde_json::Value>), AppError> | 
/write | POST | writing::process_writing | JSON WriteRequest | Result<(CookieJar, Json<WriteResponse>), AppError> | 
/writings | GET | writing::get_writings | None | Result<Html<String>, AppError> | 
/writing/{id} | GET | writing::view_writing | None | Result<Html<String>, AppError> | 
/api/writing/{sessionId}/status | GET | writing::get_writing_status_web | None | Result<Json<serde_json::Value>, AppError> | 
/collection/create | POST | collection::create_collection | JSON CollectionCreateRequest | Result<Json<CollectionCreateResponse>, AppError> | 
/collection/{id} | GET | collection::get_collection | None | Result<Html<String>, AppError> | 
/payment/verify | POST | payment::verify_payment | JSON PaymentVerifyRequest | Result<Json<PaymentVerifyResponse>, AppError> | 
/notify/signup | POST | notification::signup | JSON NotifySignupRequest | Result<Json<SignupResponse>, AppError> | 
/api/ankys | GET | api::list_ankys | None | Result<Json<serde_json::Value>, AppError> | 
/api/v1/ankys | GET | api::list_ankys | None | Result<Json<serde_json::Value>, AppError> | 
/api/generate | POST | api::generate_anky | JSON GenerateAnkyRequest | Result<Json<serde_json::Value>, AppError> | 
/api/v1/anky/{id} | GET | api::get_anky | None | Result<Json<serde_json::Value>, AppError> | 
/api/v1/mind/status | GET | api::get_mind_status | None | Json<serde_json::Value> | 
/api/v1/anky/{id}/metadata | GET | swift::anky_metadata | None | Result<Json<serde_json::Value>, AppError> | 
/api/stream-reflection/{id} | GET | api::stream_reflection | None | impl IntoResponse | 
/api/warm-context | POST | api::warm_context | None | Json<serde_json::Value> | 
/api/me | GET | api::web_me | None | Json<serde_json::Value> | 
/api/my-ankys | GET | api::web_my_ankys | None | Result<Json<serde_json::Value>, AppError> | 
/api/chat-history | GET | api::web_chat_history | None | Json<serde_json::Value> | 
/api/anky-card/{id} | GET | api::anky_reflection_card_image | None | Result<Response, AppError> | 
/api/checkpoint | POST | api::save_checkpoint | JSON CheckpointRequest | Result<Json<serde_json::Value>, AppError> | 
/api/session/paused | GET | api::get_paused_writing_session | None | Result<Json<serde_json::Value>, AppError> | 
/api/session/pause | POST | api::pause_writing_session | JSON PauseWritingSessionRequest | Result<Json<serde_json::Value>, AppError> | 
/api/session/resume | POST | api::resume_writing_session | JSON ResumeWritingSessionRequest | Result<Json<serde_json::Value>, AppError> | 
/api/session/discard | POST | api::discard_paused_writing_session | JSON DiscardPausedSessionRequest | Result<Json<serde_json::Value>, AppError> | 
/api/prefetch-memory | POST | api::prefetch_memory | JSON PrefetchMemoryRequest | Json<serde_json::Value> | 
/api/cost-estimate | GET | api::cost_estimate | None | Result<Json<serde_json::Value>, AppError> | 
/api/treasury | GET | api::treasury_address | None | Json<serde_json::Value> | 
/api/mirror | GET | api::mirror | None | Result<Response, AppError> | 
/api/mirror/gallery | GET | api::mirror_gallery | None | Result<Response, AppError> | 
/api/mirror/chat | POST | api::mirror_chat | JSON serde_json::Value | Result<Json<serde_json::Value>, AppError> | 
/api/mirror/solana-mint | POST | api::solana_mint_mirror | JSON serde_json::Value | Result<Json<serde_json::Value>, AppError> | 
/api/mirror/raw-mint | POST | api::raw_mint_mirror | JSON serde_json::Value | Result<Json<serde_json::Value>, AppError> | 
/api/mirror/supply | GET | api::mirror_supply | None | Result<Json<serde_json::Value>, AppError> | 
/api/mirror/collection-metadata | GET | api::mirror_collection_metadata | None | Json<serde_json::Value> | 
/api/mirror/metadata/{id} | GET | api::mirror_metadata | None | Result<Json<serde_json::Value>, AppError> | 
/image.png | GET | api::mirror_latest_image | None | Result<Response, AppError> | 
/splash.png | GET | api::mirror_latest_image | None | Result<Response, AppError> | 
/api/miniapp/notifications | POST | api::save_notification_token | JSON SaveNotificationTokenRequest | Result<Response, AppError> | 
/api/miniapp/prompt | GET | api::get_farcaster_prompt | None | Response | 
/api/webhook | POST | api::farcaster_miniapp_webhook | JSON serde_json::Value | Response | 
/api/miniapp/onboarding | GET | api::miniapp_onboarding_status | None | impl IntoResponse | 
/api/miniapp/onboard | POST | api::miniapp_onboard | JSON serde_json::Value | impl IntoResponse | 
/api/miniapp/images | GET | api::miniapp_image_list | None | impl IntoResponse | 
/api/miniapp/stickers | GET | api::miniapp_sticker_list | None | impl IntoResponse | 
/api/altar | GET | altar::get_altar | None | Result<Json<serde_json::Value>, AppError> | 
/api/altar/burn | POST | altar::verify_burn | JSON BurnRequest | Result<impl IntoResponse, AppError> | 
/api/altar/checkout | POST | altar::create_checkout | JSON CheckoutRequest | Result<Json<serde_json::Value>, AppError> | 
/api/altar/stripe-success | GET | altar::stripe_success | None | Result<Json<serde_json::Value>, AppError> | 
/api/altar/payment-intent | POST | altar::create_payment_intent | JSON CreatePaymentIntentRequest | Result<Json<serde_json::Value>, AppError> | 
/api/altar/apple-pay | POST | altar::apple_pay_burn | JSON ApplePayRequest | Result<Json<serde_json::Value>, AppError> | 
/api/auth/qr | POST | qr_auth::create_challenge | None | Result<Json<serde_json::Value>, AppError> | 
/api/auth/qr/seal | POST | qr_auth::seal_by_token | JSON SealByTokenRequest | Result<Json<serde_json::Value>, AppError> | 
/api/auth/qr/{id} | GET | qr_auth::poll_challenge | None | Result<Json<serde_json::Value>, AppError> | 
/api/auth/qr/{id}/seal | POST | qr_auth::seal_challenge | JSON SealByIdRequest | Result<Json<serde_json::Value>, AppError> | 
/api/sessions/seal | POST | sealed::seal_session | JSON SealSessionRequest | Result<Json<SealSessionResponse>, AppError> | 
/api/verify/{session_hash} | GET | sealed::verify_sealed_session | None | Result<Json<VerifyResponse>, AppError> | 
/api/anky/public-key | GET | sealed::get_enclave_public_key | None | axum::response::Response | 
/api/feedback | POST | api::submit_feedback | JSON FeedbackRequest | Result<Json<serde_json::Value>, AppError> | 
/api/v1/feedback | POST | api::submit_feedback | JSON FeedbackRequest | Result<Json<serde_json::Value>, AppError> | 
/api/chat | POST | api::chat_with_anky | JSON ChatRequest | Result<Json<serde_json::Value>, AppError> | 
/api/chat-quick | POST | api::chat_quick | JSON QuickChatRequest | Result<Json<serde_json::Value>, AppError> | 
/api/suggest-replies | POST | api::suggest_replies | JSON SuggestRepliesRequest | Result<Json<serde_json::Value>, AppError> | 
/api/retry-failed | POST | api::retry_failed | None | Result<Json<serde_json::Value>, AppError> | 
/api/v1/generate/video-frame | POST | api::generate_video_frame | JSON VideoFrameRequest | Result<Response, AppError> | 
/api/v1/generate/video | POST | api::generate_video | JSON VideoGenerateRequest | Result<Response, AppError> | 
/api/v1/create-videos/{id} | GET | api::get_create_video_card | None | Result<Json<serde_json::Value>, AppError> | 
/api/v1/create-videos/image | POST | api::generate_create_video_image | JSON CreateVideoRequest | Result<Json<serde_json::Value>, AppError> | 
/api/v1/create-videos/video | POST | api::generate_create_video_clip | JSON CreateVideoRequest | Result<Json<serde_json::Value>, AppError> | 
/api/v1/video/{id} | GET | api::get_video_project | None | Result<Json<serde_json::Value>, AppError> | 
/api/v1/video/{id}/resume | POST | api::resume_video_project | None | Result<Json<serde_json::Value>, AppError> | 
/api/v1/video/pipeline/config | GET | api::get_video_pipeline_config | None | Result<Json<serde_json::Value>, AppError> | 
/api/v1/video/pipeline/config | POST | api::save_video_pipeline_config | JSON SaveVideoPipelineConfigRequest | Result<Json<serde_json::Value>, AppError> | 
/api/v1/purge-cache | POST | api::purge_cache | None | Result<Json<serde_json::Value>, AppError> | 
/og/video | GET | api::og_video_image | None | Result<Response, AppError> | 
/og/dca | GET | api::og_dca_image | None | Result<Response, AppError> | 
/api/v1/feed | GET | api::get_feed | None | Result<Json<serde_json::Value>, AppError> | 
/api/v1/anky/{id}/like | POST | api::toggle_like | None | Result<Json<serde_json::Value>, AppError> | 
/api/v1/story/test | POST | api::story_test | JSON StoryTestRequest | Result<Json<serde_json::Value>, AppError> | 
/admin/story-tester | GET | api::admin_story_tester | None | Result<Response, AppError> | 
/onboarding-lab | GET | api::onboarding_lab_page | None | axum::response::Html<&'static str> | 
/flux-lab | GET | api::flux_lab_page | None | axum::response::Html<&'static str> | 
/api/v1/flux-lab/experiments | GET | api::flux_lab_list_experiments | None | Result<Response, AppError> | 
/api/v1/flux-lab/experiments/{name} | GET | api::flux_lab_get_experiment | None | Result<Response, AppError> | 
/api/v1/flux-lab/generate | POST | api::flux_lab_generate | JSON serde_json::Value | Result<Response, AppError> | 
/api/v1/ankycoin/generate | POST | api::ankycoin_generate_image | JSON serde_json::Value | Result<Response, AppError> | 
/api/v1/ankycoin/latest | GET | api::ankycoin_latest_image | None | Result<Response, AppError> | 
/media-factory | GET | api::media_factory_page | None | impl IntoResponse | 
/api/v1/media-factory/list | GET | api::media_factory_list | None | Result<Response, AppError> | 
/api/v1/media-factory/video/{request_id} | GET | api::media_factory_video_poll | None | Result<Response, AppError> | 
/api/v1/check-prompt | POST | api::check_prompt | JSON CheckPromptRequest | Result<Json<serde_json::Value>, AppError> | 
/api/v1/og-embed | GET | api::og_embed_image | None | Result<Response, AppError> | 
/api/v1/stories | GET | swift::list_all_stories | None | Result<Json<Vec<CuentacuentosItem>>, AppError> | 
/api/v1/stories/{id} | GET | swift::get_story | None | Result<Json<CuentacuentosItem>, AppError> | 
/api/v1/stories/{story_id}/recordings | GET | voices::list_recordings | None | Result<Json<Vec<RecordingItem>>, AppError> | 
/api/v1/stories/{story_id}/recordings | POST | voices::create_recording | Multipart form-data | Result<Json<serde_json::Value>, AppError> | 
/api/v1/stories/{story_id}/voice | GET | voices::get_voice | None | Result<Json<serde_json::Value>, AppError> | 
/api/v1/stories/{story_id}/recordings/{recording_id}/complete | POST | voices::complete_listen | None | Result<Json<serde_json::Value>, AppError> | 
/api/v1/register | POST | extension_api::register | JSON RegisterRequest | Result<Json<RegisterResponse>, AppError> | 
/api/v1/session/start | POST | session::start_session | JSON StartRequest | Result<Json<StartResponse>, AppError> | 
/api/v1/session/chunk | POST | session::send_chunk | JSON ChunkRequest | Result<Json<ChunkResponse>, AppError> | 
/api/v1/session/{id}/events | GET | session::session_events | None | Result<Json<SessionEventsResponse>, AppError> | 
/api/v1/session/{id}/result | GET | session::session_result | None | Result<Json<SessionResultResponse>, AppError> | 
/api/v1/session/{id} | GET | session::session_status | None | Json<serde_json::Value> | 
/manifesto.md | GET | manifesto_md | None | ([(axum::http::HeaderName, &'static str); 1], &'static str) | 
/MANIFESTO.md | GET | manifesto_md | None | ([(axum::http::HeaderName, &'static str); 1], &'static str) | 
/PROMPT.md | GET | prompt_md | None | ([(axum::http::HeaderName, &'static str); 1], &'static str) | 
/SOUL.md | GET | soul_md | None | ([(axum::http::HeaderName, &'static str); 1], &'static str) | 
/prompts/{id} | GET | serve_prompt | None | Result<([(axum::http::HeaderName, String); 1], String), axum::http::StatusCode> | 
/skills | GET | skills | None | ([(axum::http::HeaderName, &'static str); 1], &'static str) | 
/skill.md | GET | skill_md | None | ([(axum::http::HeaderName, &'static str); 1], &'static str) | 
/skill | GET | skills_redirect | None | axum::response::Redirect | 
/skills.md | GET | skills_redirect | None | axum::response::Redirect | 
/agent-skills/anky | GET | anky_skill_bundle | None | ([(axum::http::HeaderName, &'static str); 1], &'static str) | 
/agent-skills/anky/ | GET | anky_skill_bundle | None | ([(axum::http::HeaderName, &'static str); 1], &'static str) | 
/agent-skills/anky/skill.md | GET | anky_skill_bundle_entry_redirect | None | axum::response::Redirect | 
/agent-skills/anky/skills.md | GET | anky_skill_bundle_entry_redirect | None | axum::response::Redirect | 
/agent-skills/anky/manifest.json | GET | anky_skill_bundle_manifest | None | ([(axum::http::HeaderName, &'static str); 1], &'static str) | 
/api/ankys/today | GET | live::todays_ankys | None | Json<serde_json::Value> | 
/api/live-status | GET | live::live_status_sse | None | Sse<impl futures::Stream<Item = Result<Event, Infallible>>> | 
/interview | GET | interview::interview_page | None | Result<Html<String>, AppError> | 
/ws/interview | GET | interview::ws_interview_proxy | None | impl IntoResponse | 
/api/interview/start | POST | interview::interview_start | JSON InterviewStartRequest | Result<Json<serde_json::Value>, AppError> | 
/api/interview/message | POST | interview::interview_message | JSON InterviewMessageRequest | Result<Json<serde_json::Value>, AppError> | 
/api/interview/end | POST | interview::interview_end | JSON InterviewEndRequest | Result<Json<serde_json::Value>, AppError> | 
/api/interview/history/{user_id} | GET | interview::interview_history | None | Result<Json<serde_json::Value>, AppError> | 
/api/interview/user-context/{user_id} | GET | interview::interview_user_context | None | Result<Json<serde_json::Value>, AppError> | 
/stream/overlay | GET | pages::stream_overlay | None | Result<Html<String>, AppError> | 
/generations | GET | generations::list_batches | None | Result<Html<String>, AppError> | 
/generations/{id} | GET | generations::review_batch | None | Result<Html<String>, AppError> | 
/generations/{id}/status | POST | generations::save_status | None | Result<axum::Json<serde_json::Value>, AppError> | 
/generations/{id}/dashboard | GET | generations::generation_dashboard | None | Result<Html<String>, AppError> | 
/generations/{id}/progress | GET | generations::generation_progress | None | Result<axum::Json<serde_json::Value>, AppError> | 
/generations/{id}/tinder | GET | generations::review_images | None | Result<Html<String>, AppError> | 
/generations/{id}/review | POST | generations::save_review | None | Result<axum::Json<serde_json::Value>, AppError> | 
/training | GET | training::training_page | None | Result<Html<String>, AppError> | 
/trainings | GET | training::trainings_list | None | Result<Html<String>, AppError> | 
/trainings/general-instructions | GET | training::general_instructions | None | Result<Html<String>, AppError> | 
/trainings/{date} | GET | training::training_run_detail | None | Result<Html<String>, AppError> | 
/api/training/next | GET | training::next_image | None | Result<Json<NextResponse>, AppError> | 
/api/training/vote | POST | training::vote | JSON VoteRequest | Result<Json<serde_json::Value>, AppError> | 
/api/training/heartbeat | POST | training::training_heartbeat | JSON TrainingHeartbeat | Result<Json<serde_json::Value>, AppError> | 
/api/training/state | GET | training::training_state | None | Result<Json<serde_json::Value>, AppError> | 
/training/live | GET | training::training_live | None | Result<Html<String>, AppError> | 
/training/live/samples/{filename} | GET | training::training_sample_image | None | Result<axum::response::Response<axum::body::Body>, AppError> | 
/api/memory/backfill | POST | api::memory_backfill | None | Result<Json<serde_json::Value>, AppError> | 
/evolve | GET | evolve::evolve_dashboard | None | Result<Html<String>, AppError> | 
/dashboard | GET | dashboard::dashboard | None | Result<Html<String>, AppError> | 
/dashboard/logs | GET | dashboard::dashboard_logs | None | Sse<impl Stream<Item = Result<Event, Infallible>>> | 
/dashboard/summaries | GET | dashboard::dashboard_summaries | None | Result<Json<Vec<serde_json::Value>>, AppError> | 
/.well-known/apple-app-site-association | GET | apple_app_site_association | None | ([(axum::http::HeaderName, &'static str); 1], &'static str) | 
/.well-known/farcaster.json | GET | farcaster_manifest | None | ([(axum::http::HeaderName, &'static str); 1], &'static str) | 
/.well-known/agent | GET | agent_manifest | None | ([(axum::http::HeaderName, &'static str); 1], &'static str) | 
/sw.js | GET | service_worker | None | ([(axum::http::HeaderName, &'static str); 2], &'static str) | 
/webhooks/x | GET | webhook_x::webhook_crc | None | Result<Json<serde_json::Value>, AppError> | 
/webhooks/x | POST | webhook_x::webhook_post | Raw request body bytes | impl IntoResponse | 
/webhooks/farcaster | POST | webhook_farcaster::webhook_post | Raw request body bytes | impl IntoResponse | 
/webhooks/logs | GET | webhook_x::webhook_logs_page | None | Html<String> | 
/webhooks/logs/stream | GET | webhook_x::webhook_logs_stream | None | Sse<impl Stream<Item = Result<Event, Infallible>>> | 
/health | GET | health::health_check | None | Result<Json<HealthResponse>, AppError> | 
/api/health | GET | health::health_check | None | Result<Json<HealthResponse>, AppError>
```

Static service mounts in the router:
- `GET/HEAD /agent-skills/*` -> `ServeDir("agent-skills")`
- `GET/HEAD /static/*` -> `ServeDir("static")`
- `GET/HEAD /data/images/*` -> `ServeDir("data/images")` with immutable cache header
- `GET/HEAD /data/anky-images/*` -> `ServeDir("data/anky-images")` with immutable cache header
- `GET/HEAD /flux/*` -> `ServeDir("flux")`
- `GET/HEAD /data/writings/*` -> `ServeDir("data/writings")`
- `GET/HEAD /videos/*` -> `ServeDir("videos")`
- `GET/HEAD /data/videos/*` -> `ServeDir("data/videos")`
- `GET/HEAD /gen-images/*` -> `ServeDir("data/generations")`
- `GET/HEAD /data/training-images/*` -> `ServeDir("data/training-images")`
- `GET/HEAD /data/training-runs/*` -> `ServeDir("data/training_runs")`
- `GET/HEAD /data/mirrors/*` -> `ServeDir("data/mirrors")`
- `GET/HEAD /data/classes/*` -> `ServeDir("data/classes")`

## 3. Session Handling

There are three materially different writing flows today:
- Web writing: `POST /write` in [writing.rs](/home/kithkui/anky/src/routes/writing.rs)
- Mobile writing: `POST /swift/v1/write` and `POST /swift/v2/write` in [swift.rs](/home/kithkui/anky/src/routes/swift.rs)
- Agent chunked writing: `POST /api/v1/session/start` + `POST /api/v1/session/chunk` in [session.rs](/home/kithkui/anky/src/routes/session.rs)

### Mobile unified flow

Input body:

```rust
pub struct MobileWriteRequest {
    pub text: String,
    pub duration: f64,
    pub session_id: Option<String>,
    pub keystroke_deltas: Option<Vec<f64>>,
    pub is_checkpoint: bool,
}
```

Decision tree in `submit_writing_unified`:
- If `is_checkpoint == true`: upsert active `writing_sessions` row, insert `writing_checkpoints` row, optionally send to Honcho, return immediately.
- If final submission has `< 10` words: nothing persisted, returns `outcome=short_session`, `persisted=false`.
- If not an anky (`duration < 480` or `< 300` words):
  - seed user (has wallet): returns local-only, no Postgres persistence.
  - Privy/non-seed user: persists `writing_sessions`, later fills `writing_sessions.response`, fires Honcho and next-prompt generation in background.
- If an anky (`duration >= 480 && words >= 300`): persists `writing_sessions`, inserts `ankys` row, optionally auto-mints Solana cNFT, archives raw writing to filesystem for seed users, enqueues Redis GPU job, optionally spawns cuentacuentos lifecycle, sends to Honcho, returns immediately with a polling URL.

Core anky branch:

```rust
queries::upsert_completed_writing_session_with_flow(... true ...)?;
queries::insert_anky(... "generating", "mobile", None)?;
crate::services::redis_queue::enqueue_job(
    &state.config.redis_url,
    &crate::state::GpuJob::AnkyImage {
        anky_id: anky_id.clone(),
        session_id: session_id.clone(),
        user_id: user_id.clone(),
        writing: req.text.clone(),
    },
    is_pro,
).await?;
```

### Web `/write` flow

Input body from [models/mod.rs](/home/kithkui/anky/src/models/mod.rs):

```rust
pub struct WriteRequest {
    pub text: String,
    pub duration: f64,
    pub session_id: Option<String>,
    pub session_token: Option<String>,
    pub keystroke_deltas: Option<Vec<f64>>,
    pub inquiry_id: Option<String>,
    pub prompt_id: Option<String>,
}
```

Behavior differences vs mobile:
- Anonymous web users are identified by `anky_user_id` visitor cookie.
- If the user has no wallet, the backend generates a custodial wallet and stores it on `users.generated_wallet_secret` / `users.wallet_address`.
- Non-anky writes always persist to `writing_sessions` and immediately get a quick response from OpenRouter or Claude Haiku, with fallback to a local nudge.
- Anky writes persist `writing_sessions`, insert `ankys`, enqueue a Redis image job, and return a placeholder response. Reflection is streamed later via `GET /api/stream-reflection/{anky_id}`.

### Agent chunked flow

This is different from both web and mobile:
- `POST /api/v1/session/start` authenticates an agent API key, creates an in-memory `ActiveSession`, and records timeline entries in `agent_session_events`.
- `POST /api/v1/session/chunk` appends chunks into memory only.
- The session ends when the agent is silent for 8 seconds. Finalization happens in-process, not via Redis.
- If the chunked session dies before the 8-minute threshold: `finalize_non_anky` writes a completed `writing_sessions` row and stores a Haiku feedback string in `writing_sessions.response`.
- If it crosses the threshold: `finalize_anky` writes a completed `writing_sessions` row, inserts an `ankys` row, stores a deep Haiku reflection in `writing_sessions.response`, and enqueues a Redis `GpuJob::AnkyImage`.

### What gets stored in Postgres

Always or often:
- `writing_sessions`: raw content, duration, word count, `is_anky`, `response`, `keystroke_deltas`, `flow_score`, `status`, pause/resume timestamps, `session_token`, `content_deleted_at`, `anky_response`, `anky_next_prompt`, `anky_mood`.
- `writing_checkpoints`: checkpoint text snapshots with elapsed duration and word count.
- `ankys`: anky lifecycle record, image/reflection/title/mint metadata/status/origin, plus later story/image fields.
- `cost_records`: model/provider cost entries when AI calls are saved.
- `agent_session_events`: chunked agent timeline.
- Optional downstream tables: `cuentacuentos`, `cuentacuentos_images`, `cuentacuentos_audio`, `story_training_pairs`, `story_recordings`, `user_memories`, `user_profiles`, `next_prompts`.

### What gets stored in Redis

Only queued GPU jobs and their processing metadata. Session state itself is **not** stored in Redis.

Keys and structures from [services/redis_queue.rs](/home/kithkui/anky/src/services/redis_queue.rs):

```rust
const PRO_QUEUE: &str = "anky:jobs:pro";
const FREE_QUEUE: &str = "anky:jobs:free";
const PROCESSING_SET: &str = "anky:jobs:processing";
const FAILED_SET: &str = "anky:jobs:failed";

pub struct QueuedGpuJob {
    pub id: String,
    pub job: GpuJob,
    pub is_pro: bool,
    pub retry_count: u32,
    pub created_at: i64,
}
```

Important nuance:
- The `GpuJob::AnkyImage` payload includes the full writing text. So the writing content is present in Redis **inside the job payload** until the job completes or fails.

### What gets discarded

Definitely discarded or not persisted:
- Final submissions under 10 words: discarded after response.
- Mobile seed-user short sessions: not persisted to Postgres.
- In-memory chunked sessions: no DB row until finalization.

Explicitly nullified later:
- The seed-user cuentacuentos lifecycle calls `queries::nullify_writing_content`, which sets `writing_sessions.content = NULL` and records `content_deleted_at` after story export.

Still retained elsewhere even after nullification:
- Filesystem archive for seed users under `data/writings/<wallet>/<timestamp>.txt`
- Sealed session envelopes under `data/sealed/...` if the user sealed the session
- Honcho, if `send_writing` already ran
- Redis job payloads until queue completion for in-flight jobs

## 4. AI Processing

### How reflections are generated right now

There are two main reflection paths:
- Primary web anky reflection: `GET /api/stream-reflection/{id}` in [api.rs](/home/kithkui/anky/src/routes/api.rs). This is the user-facing SSE path.
- Fallback reflection inside image generation: if the image pipeline finishes and `ankys.reflection` is still empty, [image_gen.rs](/home/kithkui/anky/src/pipeline/image_gen.rs) generates and saves title+reflection.

The current title/reflection system prompts are in [services/claude.rs](/home/kithkui/anky/src/services/claude.rs):

```rust
const TITLE_AND_REFLECTION_SYSTEM_KNOWN: &str = r#"...You know this person...
TITLE (first line of your response):
3-5 words. Lowercase.

Then a blank line. Then begin the reflection body with the natural equivalent...
hey, thanks for being who you are. my thoughts:
...Respond in the same language they wrote in."#;

const TITLE_AND_REFLECTION_SYSTEM_STRANGER: &str = r#"Someone just wrote for 8 unbroken minutes...
TITLE (first line of your response):
3-5 words. Lowercase.

Then a blank line. Then begin the reflection body with...
hey, thanks for being who you are. my thoughts:
...Respond in the same language they wrote in."#;
```

Streaming reflection routing:

```rust
pub async fn stream_title_and_reflection_best(...) -> Result<(String, i64, i64, String, String)> {
    if !config.openrouter_api_key.is_empty() && !config.openrouter_anky_model.is_empty() {
        // OpenRouter first
    }
    // else Anthropic using reflection_model with conversation_model fallback
}
```

Non-streaming fallback routing:
- `generate_title_and_reflection_with_memory_using_model` does `Mind -> Claude chosen model -> fallback Claude model -> OpenRouter`.
- `stream_reflection` itself has another fallback chain if the streaming path fails:
  - Claude/OpenRouter stream path
  - then Claude Haiku with `ollama::deep_reflection_prompt`
  - then OpenRouter `anthropic/claude-3.5-haiku` with system prompt `You are a contemplative writing mirror.`

The fallback non-title prompt used in those emergency branches is in [services/ollama.rs](/home/kithkui/anky/src/services/ollama.rs):

```rust
pub fn deep_reflection_prompt(text: &str) -> String {
    format!(
        r#"Read this writing. The person wrote for 8 unbroken minutes...
In the tradition of Ramana Maharshi and Jed McKenna: don't analyze. Point...
Keep it to 2-3 paragraphs. No softening, no framework.
Respond in their language."#,
        text
    )
}
```

### Memory/context inputs

Reflection generation can be conditioned on:
- prewarmed memory context from `state.memory_cache`
- Honcho peer context
- local memory recall/building from the memory pipeline

### Does it go to Poiesis, cloud API, or both?

Ground truth:
- There is **no** separate reflection service called “Poiesis”.
- `poiesis.rs` exists but is just an unwired HTML/SSE log viewer. It is not used in reflection routing.
- The running reflection path can be local **and** cloud-backed:
  - local: `Mind` (`MIND_URL`) first in some non-streaming paths
  - cloud: OpenRouter and Anthropic are the main streaming/user-facing providers
  - Honcho is consulted for memory context when configured

### Tier routing

I did **not** find reflection-model routing by subscription tier.
- `users.is_pro` affects Redis queue priority only.
- `users.is_premium` is surfaced in `/swift/v1/me` and can be toggled by an admin endpoint, but I found no model/provider switch based on `is_premium`.
- There is no Stripe-to-premium automation.

## 5. Image Generation

### What triggers image generation

An image job is triggered when an anky is created:
- Mobile: `submit_writing_unified` inserts an `ankys` row and enqueues `GpuJob::AnkyImage`.
- Web: `process_writing` inserts an `ankys` row and enqueues the same job.
- Agent chunked sessions: `finalize_anky` enqueues the same job after session timeout/finalization.
- Cuentacuentos images and audio are also queued as GPU jobs after story generation/translation.

### Current anky image pipeline

From [pipeline/image_gen.rs](/home/kithkui/anky/src/pipeline/image_gen.rs):

```rust
// 1. Generate image prompt: Mind first, then Claude Haiku, else raw writing
// 2. Generate image: Gemini first, fall back to local ComfyUI/Flux
// 3. If reflection missing, generate title+reflection fallback
// 4. Convert to WebP + thumbnail
// 5. Upload to R2 if configured
// 6. Save image and mark anky complete
// 7. Spawn writing formatting + memory extraction in background
```

The image prompt instruction is in [services/ollama.rs](/home/kithkui/anky/src/services/ollama.rs):

```rust
pub const IMAGE_PROMPT_SYSTEM: &str = r#"CONTEXT: You are generating an image prompt for Anky...
YOUR TASK: ...create a scene where Anky embodies the EMOTIONAL TRUTH...
OUTPUT: A single detailed image generation prompt, 2-3 sentences..."#;
```

Important implementation details:
- The pipeline requires both `ANTHROPIC_API_KEY` and `GEMINI_API_KEY`; if either is missing, it logs a warning and skips generation.
- Kingdom-specific flavor text is appended to the image prompt if `ankys.kingdom_id` is already set.
- Gemini is the primary renderer. Local ComfyUI/Flux is fallback for ankys; Flux is also used directly for some paid/free image endpoints.

### Where images are stored and served

Stored locally:
- Main anky images: `data/images/<...>`
- WebP derivatives: `ankys.image_webp`
- Thumbnails: `ankys.image_thumb`
- Mirror images: `data/mirrors/<mirror_id>.png`
- Training/media outputs in their respective `data/*` dirs

Served by the app:
- `GET/HEAD /data/images/*`
- `GET/HEAD /data/anky-images/*`
- `GET/HEAD /data/mirrors/*`
- Several other static mounts for generated media

Optional CDN copy:
- If R2 is configured, the pipeline uploads image bytes to Cloudflare R2 and stores an externally-served URL in the generated `.anky` story payload.

What is written back to Postgres:
- `ankys.image_prompt`
- `ankys.image_path`
- `ankys.caption`
- `ankys.image_model`
- `ankys.image_webp`
- `ankys.image_thumb`
- `ankys.anky_story`
- `ankys.status` transitions to complete via `update_anky_image_complete`

## 6. Queue System

Redis is used as a persistent job queue for GPU-ish background work. The actual worker is the Rust server process in worker/full mode.

Queue model from [state.rs](/home/kithkui/anky/src/state.rs):

```rust
pub enum GpuJob {
    AnkyImage { anky_id: String, session_id: String, user_id: String, writing: String },
    CuentacuentosImages { cuentacuentos_id: String },
    CuentacuentosAudio { cuentacuentos_id: String },
}
```

Redis behavior from [services/redis_queue.rs](/home/kithkui/anky/src/services/redis_queue.rs):
- Pro jobs go to `anky:jobs:pro`
- Free jobs go to `anky:jobs:free`
- When dequeued, the serialized payload is copied to `anky:jobs:processing:<job_id>` with a 1 hour TTL
- Failed jobs are copied to `anky:jobs:failed:<job_id>` with a 24 hour TTL
- Startup recovery scans `anky:jobs:processing:*` and requeues jobs up to 5 retries

Worker loop from [main.rs](/home/kithkui/anky/src/main.rs):

```rust
async fn gpu_job_worker(state: AppState) {
    loop {
        let Some(job) = redis_queue::dequeue_job(...).await? else { sleep(...); continue; };
        if let Err(e) = process_gpu_job(&state, &job.job).await {
            redis_queue::fail_job(...).await?;
            continue;
        }
        redis_queue::complete_job(...).await?;
    }
}
```

How “Poiesis” relates to the queue:
- It doesn’t. There is no separate Poiesis worker process picking up Redis jobs.
- `routes/poiesis.rs` is only an unwired page/stream for server logs.
- The same Rust binary drains Redis in `RunMode::Full` or `RunMode::Worker`.

Is there a webhook on completion?
- No Redis-job completion webhook exists.
- Web/mobile clients poll status endpoints (`/api/writing/{sessionId}/status`, `/swift/v2/writing/{sessionId}/status`, or anky endpoints).
- The only separate worker with HTTP completion semantics is the Solana mint Cloudflare worker, which returns the transaction result inline to the caller.

## 7. Notifications

Yes, there is push-notification infrastructure.

APNs:
- Implemented in [services/apns.rs](/home/kithkui/anky/src/services/apns.rs) using token-based auth from `.p8` key material.
- Config fields: `APNS_KEY_PATH`, `APNS_KEY_ID`, `APNS_TEAM_ID`, `APNS_BUNDLE_ID`, `APNS_ENVIRONMENT`.
- Device registration endpoints:
  - `POST /swift/v2/device-token`
  - `POST /swift/v2/devices`
  - `DELETE /swift/v2/devices`
- Tokens are stored in `device_tokens`.

Daily push scheduler:
- Implemented in [services/push_scheduler.rs](/home/kithkui/anky/src/services/push_scheduler.rs).
- Cron: `0 30 5 * * *` (5:30 AM UTC daily).
- It loads users with device tokens and at least one writing session, builds a short re-engagement message with Claude Haiku, and sends it via APNs.

Farcaster notifications also exist:
- Token storage table: `farcaster_notification_tokens`
- Save endpoint: `POST /api/miniapp/notifications`
- Miniapp webhook: `POST /api/webhook`
- Neynar webhook: `POST /webhooks/farcaster`
- Notifications are posted back to a saved Farcaster notification URL; this is separate from APNs.

What does **not** exist:
- No FCM/Android implementation found.
- No web push implementation found.
- No generic notification service abstraction; APNs and Farcaster notifications are separate codepaths.

## 8. Payments / Subscriptions

There is payment logic, but there is **not** a full recurring subscription system.

What exists:
- Base-chain USDC verification for collection/payment endpoints in [routes/payment.rs](/home/kithkui/anky/src/routes/payment.rs) and [services/payment.rs](/home/kithkui/anky/src/services/payment.rs).
- x402-compatible payment-required responses and facilitator verification in [middleware/x402.rs](/home/kithkui/anky/src/middleware/x402.rs).
- Paid prompt/generation endpoints use `payment_helper::validate_payment` and accept:
  - registered agent API keys
  - direct wallet tx hashes in headers
  - x402 facilitator payloads
- Stripe one-off payment logic exists for the altar in [routes/altar.rs](/home/kithkui/anky/src/routes/altar.rs): Checkout Sessions, PaymentIntents, and Apple Pay verification.
- Altar burns are stored in `altar_burns`.

What does **not** exist:
- No subscription table.
- No recurring Stripe subscription flow.
- No Stripe webhook handler for subscription lifecycle.
- No automatic bridge from payment completion to `users.is_premium`.

Premium/pro flags today:
- `users.is_premium` and `users.premium_since` exist.
- `POST /swift/v1/admin/premium` toggles `is_premium` directly.
- `users.is_pro` exists and is used for GPU queue priority, but I found no purchase or admin route that currently sets it in code.

## 9. Arweave

I found **no implemented Arweave integration**.

Ground truth:
- No source files reference `arweave`.
- No uploader, no SDK client, no background job, no completion handler.
- The only Arweave-related reference is a comment in [sealed.rs](/home/kithkui/anky/src/routes/sealed.rs):

```rust
// Store on disk for future Arweave upload
```

What actually happens today:
- Sealed session envelopes are stored in Postgres (`sealed_sessions`) and on local disk under `data/sealed/<user>/<hash>.sealed`.
- On-chain metadata for Base minting uses Pinata/IPFS, not Arweave.

## 10. Solana / Minting

Yes, there is real Solana cNFT minting logic and Bubblegum integration.

### Mirror cNFT minting

Current mirror mint paths:
- `POST /api/mirror/solana-mint`
- `POST /api/mirror/raw-mint`
- `POST /swift/v2/mint-mirror`
- `POST /mirror/mint` (alias)
- `POST /swift/v2/mirror/mint`

These call the separate Cloudflare worker at `SOLANA_MINT_WORKER_URL` with bearer auth `SOLANA_MINT_WORKER_SECRET`.

### Anky auto-minting

For ankys, [image_gen.rs](/home/kithkui/anky/src/pipeline/image_gen.rs) includes `mint_anky_cnft`:
- it requires the user to have a wallet
- it requires the user to already have a minted mirror (`get_user_existing_mint`)
- it POSTs to `{worker}/mint-anky`
- it stores the returned tx signature in `ankys.solana_mint_tx`

### Bubblegum worker

From [solana/worker/src/index.ts](/home/kithkui/anky/solana/worker/src/index.ts):
- `POST /mint` -> mirror cNFT mint
- `POST /mint-anky` -> anky cNFT mint
- `GET /supply` -> current minted count
- Uses `@metaplex-foundation/mpl-bubblegum`
- Calls `mintV1(...)`
- Uses a Merkle tree and collection mint for both mirrors and ankys

Representative worker code:

```ts
const builder = mintV1(umi, {
  leafOwner: publicKey(recipient),
  merkleTree,
  metadata: {
    name,
    symbol: symbol || "ANKY",
    uri,
    sellerFeeBasisPoints: 0,
    collection: { key: collectionMint, verified: false },
    creators: [{ address: umi.identity.publicKey, verified: false, share: 100 }],
  },
});
```

What gets stored for minted mirrors/ankys:
- Mirrors: `mirrors.solana_mint_tx`, `mirrors.solana_recipient`, `mirrors.solana_asset_id`, `mirrors.solana_minted_at`, `mirrors.writing_session_id`
- Ankys: `ankys.solana_mint_tx`
- Sojourn state: `sojourn_state`

Webhook/completion model:
- No separate completion webhook back into the Rust app.
- The Rust side updates DB state from the immediate worker response.

Separate non-Solana minting also exists:
- `prepare_mint` and `confirm_mint` in [swift.rs](/home/kithkui/anky/src/routes/swift.rs) implement a Base/EVM mint flow with Pinata/IPFS, gas funding, EIP-712 signing, and `SoulBorn` event verification.

## 11. Enclave

There is **no `/soul` directory** in the repository. The only matching artifact is [SOUL.md](/home/kithkui/anky/SOUL.md).

What `SOUL.md` is:
- A conceptual design document describing Anky as a timing-aware, local-first mirror.
- It says timing data is central, reflections should be short and observational, writing never leaves the machine unless exported, and timing data is never discarded.

What the actual code does instead:
- Writings are stored server-side in `writing_sessions.content`.
- Writings are sent to cloud providers in several flows.
- Keystroke timing is optional and not guaranteed.
- Raw writing can be deleted later in the cuentacuentos lifecycle.

Current enclave implementation:
- Config variable: `ANKY_ENCLAVE_URL`
- One live endpoint: `GET /api/anky/public-key` in [sealed.rs](/home/kithkui/anky/src/routes/sealed.rs)
- That endpoint simply proxies `GET {enclave_url}/public-key`
- Sealed sessions are opaque encrypted envelopes; the backend stores them but never decrypts them

Representative code:

```rust
/// The iOS app uses this to encrypt session data to the enclave.
pub async fn get_enclave_public_key(
    State(state): State<AppState>,
) -> axum::response::Response {
    if state.config.enclave_url.is_empty() { ... }
    match reqwest::get(format!("{}/public-key", state.config.enclave_url)).await { ... }
}
```

What I did **not** find:
- No enclave code directory in this repo
- No attestation flow
- No enclave-side decryption
- No enclave compute jobs
- No restore/decrypt endpoint

So today the enclave is only an external public-key source for client-side sealing.

## 12. Database Schema

This section is the **live Postgres schema**, not just the migration intent. It comes from `information_schema` against the configured database.

Important caveat:
- The codebase still contains SQLite-flavored SQL in places (`?1` placeholders, `datetime('now')`, `INSERT OR IGNORE`-style patterns), but the running database is Postgres and the live schema below reflects that reality.
- Many logical links are not enforced as foreign keys. For example, `device_tokens.user_id` and `sealed_sessions.user_id` are logical user references without declared FK constraints.

```text
_sqlx_migrations: version bigint, description text, installed_on timestamp with time zone default now(), success boolean, checksum bytea, execution_time bigint
agent_session_events: id bigint default nextval('agent_session_events_id_seq'::regclass), session_id text, user_id text, agent_id text, agent_name text, event_type text, chunk_index integer null, elapsed_seconds double precision default 0, words_total integer default 0, chunk_text text null, chunk_word_count integer null, detail_json text null, created_at text default anky_now_ms()
agents: id text, name text, description text null, model text null, api_key text, free_sessions_remaining integer default 4, total_sessions integer default 0, created_at text default anky_now() | fks: api_key->api_keys.key
altar_burns: id text default (gen_random_uuid())::text, user_identifier text, identifier_type text default 'wallet'::text, amount_usdc bigint, tx_hash text, display_name text null, avatar_url text null, fid bigint null, created_at timestamp with time zone default now()
anky_likes: user_id text, anky_id text, created_at text default anky_now()
ankys: id text, writing_session_id text, user_id text, image_prompt text null, reflection text null, title text null, image_path text null, caption text null, thinker_name text null, thinker_moment text null, is_minted integer default 0, mint_tx_hash text null, status text default 'pending'::text, created_at text default anky_now(), origin text default 'written'::text, image_webp text null, image_thumb text null, conversation_json text null, image_model text default 'gemini'::text, prompt_id text null, formatted_writing text null, gas_funded_at text null, session_cid text null, metadata_uri text null, token_id text null, anky_story text null, kingdom_id integer null, kingdom_name text null, kingdom_chakra text null, retry_count integer default 0, last_retry_at text null, solana_mint_tx text null | fks: user_id->users.id, writing_session_id->writing_sessions.id
api_keys: key text, label text null, balance_usd double precision default 0, total_spent_usd double precision default 0, total_transforms integer default 0, is_active integer default 1, created_at text default anky_now()
auth_challenges: id text, wallet_address text, challenge_text text, expires_at text, consumed_at text null, created_at text default anky_now()
auth_sessions: token text, user_id text, x_user_id text null, expires_at text, created_at text default anky_now() | fks: user_id->users.id
breathwork_completions: id text, user_id text, session_id text, completed_at text default anky_now(), notes text null | fks: session_id->breathwork_sessions.id, user_id->users.id
breathwork_sessions: id text, style text, duration_seconds integer default 480, script_json text, generated_at text default anky_now()
child_profiles: id text, parent_wallet_address text, derived_wallet_address text, name text, birthdate text, emoji_pattern text, created_at text default anky_now() | fks: parent_wallet_address->users.wallet_address
collections: id text, user_id text, mega_prompt text, beings_json text null, status text default 'pending'::text, payment_tx_hash text null, cost_estimate_usd double precision null, actual_cost_usd double precision null default 0, progress integer default 0, total integer default 88, created_at text default anky_now() | fks: user_id->users.id
cost_records: id bigint default nextval('cost_records_id_seq'::regclass), service text, model text, input_tokens integer default 0, output_tokens integer default 0, cost_usd double precision default 0, related_id text null, created_at text default anky_now()
credit_purchases: id text, api_key text, tx_hash text, amount_usdc double precision, amount_credited_usd double precision, verified integer default 0, created_at text default anky_now() | fks: api_key->api_keys.key
cuentacuentos: id text, writing_id text, parent_wallet_address text, child_wallet_address text null, title text, content text, guidance_phases text, played integer default 0, generated_at text default anky_now(), chakra integer null, kingdom text null, city text null, content_es text null, content_zh text null, content_hi text null, content_ar text null | fks: writing_id->writing_sessions.id
cuentacuentos_audio: id text, cuentacuentos_id text, language text, status text default 'pending'::text, r2_key text null, audio_url text null, duration_seconds double precision null, attempts integer default 0, error_message text null, generated_at text null, created_at text default anky_now() | fks: cuentacuentos_id->cuentacuentos.id
cuentacuentos_images: id text, cuentacuentos_id text, phase_index integer, image_prompt text, image_url text null, status text default 'pending'::text, attempts integer default 0, generated_at text null, created_at text default anky_now() | fks: cuentacuentos_id->cuentacuentos.id
device_tokens: id text, user_id text, device_token text, platform text default 'ios'::text, created_at text default anky_now(), updated_at text default anky_now()
facilitator_bookings: id text, facilitator_id text, user_id text, status text default 'pending'::text, payment_amount_usd double precision null, platform_fee_usd double precision null, payment_method text null, payment_tx_hash text null, stripe_payment_id text null, user_context_shared integer null default 0, shared_context_json text null, created_at text default anky_now() | fks: facilitator_id->facilitators.id, user_id->users.id
facilitator_reviews: id text, facilitator_id text, user_id text, rating integer, review_text text null, created_at text default anky_now() | fks: facilitator_id->facilitators.id, user_id->users.id
facilitators: id text, user_id text, name text, bio text, specialties text default '[]'::text, approach text null, session_rate_usd double precision, booking_url text null, contact_method text null, profile_image_url text null, location text null, languages text default '["en"]'::text, status text default 'pending'::text, avg_rating double precision null default 0, total_reviews integer null default 0, total_sessions integer null default 0, fee_paid integer default 0, fee_tx_hash text null, approved_at text null, created_at text default anky_now() | fks: user_id->users.id
farcaster_notification_tokens: fid bigint, token text, url text, created_at timestamp with time zone default now(), updated_at timestamp with time zone default now()
farcaster_prompts: fid bigint, prompt_text text, created_at timestamp with time zone default now()
farcaster_wallets: fid bigint, solana_address text, encrypted_keypair bytea, kingdom_id integer null, kingdom_name text null, onboarded boolean default false, mint_tx text null, created_at timestamp without time zone default now(), onboarded_at timestamp without time zone null
feedback: id text, source text, author text null, content text, status text default 'pending'::text, created_at text default anky_now()
generation_records: id text, anky_id text, api_key text null, agent_id text null, payment_method text, amount_usd double precision default 0, tx_hash text null, status text default 'pending'::text, created_at text default anky_now()
interview_messages: id bigint default nextval('interview_messages_id_seq'::regclass), interview_id text, role text, content text, created_at text default anky_now() | fks: interview_id->interviews.id
interviews: id text, user_id text null, guest_name text default 'guest'::text, is_anonymous integer default 1, started_at text default anky_now(), ended_at text null, summary text null, duration_seconds double precision null, message_count integer null default 0
llm_training_runs: id bigint default nextval('llm_training_runs_id_seq'::regclass), run_date text, val_bpb double precision, training_seconds double precision, peak_vram_mb double precision, mfu_percent double precision, total_tokens_m double precision, num_steps integer, num_params_m double precision, depth integer, corpus_sessions integer, corpus_words integer, corpus_tokens integer, epochs integer, status text default 'complete'::text, created_at text default anky_now()
meditation_sessions: id text, user_id text, duration_target integer, duration_actual integer null, completed integer default 0, created_at text default anky_now()
memory_embeddings: id text, user_id text, writing_session_id text null, source text, content text, embedding bytea, created_at text null default anky_now()
mirrors: id text, fid integer, username text, display_name text default ''::text, avatar_url text null, follower_count integer default 0, bio text default ''::text, public_mirror text, flux_descriptors_json text, image_path text null, created_at text default anky_now(), gap text default ''::text, solana_mint_tx text null, solana_recipient text null, solana_asset_id text null, solana_minted_at text null, kingdom integer null, kingdom_name text null, mirror_type text default 'public'::text, user_id text null, items_json text null, writing_session_id text null
next_prompts: user_id text, prompt_text text, generated_from_session text null, created_at text null default anky_now()
notification_signups: id bigint default nextval('notification_signups_id_seq'::regclass), email text null, telegram_chat_id text null, created_at text default anky_now()
oauth_states: state text, code_verifier text, redirect_to text null, created_at text default anky_now()
personalized_breathwork: id text, user_id text, writing_session_id text null, style text default 'calming'::text, script_json text null, status text default 'pending'::text, tier text default 'free'::text, created_at text default anky_now() | fks: user_id->users.id
personalized_meditations: id text, user_id text, writing_session_id text null, script_json text null, status text default 'pending'::text, tier text default 'free'::text, created_at text default anky_now() | fks: user_id->users.id
pipeline_prompts: key text, value text, updated_by text null, updated_at text default anky_now()
programming_classes: id integer, class_number integer, title text, description text default ''::text, concept text default ''::text, slides_json text default '[]'::text, changelog_slug text null, created_at text default anky_now()
prompt_sessions: id text, prompt_id text, user_id text null, content text null, keystroke_deltas text null, page_opened_at text null, first_keystroke_at text null, duration_seconds double precision null, word_count integer default 0, completed integer default 0, created_at text default anky_now() | fks: prompt_id->prompts.id
prompts: id text, creator_user_id text, prompt_text text, image_path text null, status text default 'pending'::text, payment_tx_hash text null, created_at text default anky_now(), created_by text null | fks: creator_user_id->users.id
qr_auth_challenges: id text default (gen_random_uuid())::text, token text, solana_address text null, sealed boolean default false, session_token text null, expires_at text, created_at text default (now())::text
sadhana_checkins: id text, commitment_id text, user_id text, date text, completed integer default 1, notes text null, created_at text default anky_now() | fks: commitment_id->sadhana_commitments.id
sadhana_commitments: id text, user_id text, title text, description text null, frequency text default 'daily'::text, duration_minutes integer default 10, target_days integer default 30, start_date text, is_active integer default 1, created_at text default anky_now() | fks: user_id->users.id
sealed_sessions: id text, user_id text, session_id text, ciphertext bytea, nonce bytea, tag bytea, user_encrypted_key bytea, anky_encrypted_key bytea, session_hash text, metadata_json text null, solana_tx_signature text null, sealed_at bigint default (EXTRACT(epoch FROM now()))::bigint, created_at timestamp without time zone null default now()
social_interactions: id text, platform text, post_id text, author_id text null, author_username text null, post_text text null, parent_id text null, status text default 'received'::text, classification text null, reply_text text null, reply_id text null, created_at text default anky_now(), updated_at text null
social_peers: id text, platform text, platform_user_id text, platform_username text null, honcho_peer_id text null, user_id text null, interaction_count integer default 0, first_seen_at text default anky_now(), last_seen_at text default anky_now()
sojourn_state: id integer default 1, sojourn_number integer default 9, max_supply integer default 3456, merkle_tree text null, collection_mint text null, started_at text null
story_listen_events: id text, story_id text, recording_id text, user_id text, listened_at text default anky_now() | fks: recording_id->story_recordings.id
story_recordings: id text, story_id text, user_id text, attempt_number integer, language text, status text default 'pending'::text, duration_seconds double precision, r2_key text null, audio_url text null, rejection_reason text null, full_listen_count integer default 0, created_at text default anky_now(), approved_at text null | fks: story_id->cuentacuentos.id, user_id->users.id
story_training_pairs: id text, cuentacuentos_id text, writing_id text, writing_input text, story_title text, story_content text, chakra integer null, kingdom text null, city text null, played integer default 0, parent_wrote_again_within_24h integer null, language text null, quality_score double precision null, exported_at text null, created_at text default anky_now() | fks: cuentacuentos_id->cuentacuentos.id
system_summaries: id text, created_at text default anky_now(), period_start text, period_end text, raw_stats text, summary text
training_labels: anky_id text, approved integer, created_at text default anky_now()
training_runs: id text, base_model text, dataset_size integer, steps integer, current_step integer default 0, loss double precision null, status text default 'pending'::text, lora_weights_path text null, started_at text null, completed_at text null, created_at text default anky_now()
transformations: id text, api_key text, input_text text, prompt text null, output_text text, input_tokens integer default 0, output_tokens integer default 0, cost_usd double precision default 0, created_at text default anky_now() | fks: api_key->api_keys.key
user_collections: user_id text, anky_id text, collected_at text default anky_now()
user_inquiries: id text, user_id text, question text, language text null default 'en'::text, response_text text null, response_session_id text null, answered_at text null, skipped integer null default 0, created_at text null default anky_now()
user_interactions: id text, user_id text, meditation_session_id text null, interaction_type text, question_text text null, response_text text null, metadata_json text null, created_at text default anky_now()
user_memories: id text, user_id text, writing_session_id text null, category text, content text, importance double precision null default 0.5, occurrence_count integer null default 1, first_seen_at text, last_seen_at text, embedding bytea null, created_at text null default anky_now()
user_profiles: user_id text, total_sessions integer null default 0, total_anky_sessions integer null default 0, total_words_written integer null default 0, psychological_profile text null, emotional_signature text null, core_tensions text null, growth_edges text null, last_profile_update text null, created_at text null default anky_now(), updated_at text null default anky_now(), current_streak integer null default 0, longest_streak integer null default 0, best_flow_score double precision null default 0, avg_flow_score double precision null default 0, last_anky_date text null
user_progression: user_id text, total_meditations integer default 0, total_completed integer default 0, current_meditation_level integer default 0, write_unlocked integer default 0, current_streak integer default 0, longest_streak integer default 0, last_session_date text null
user_settings: user_id text, font_family text default 'monospace'::text, font_size integer default 18, theme text default 'dark'::text, idle_timeout integer default 8, keyboard_layout text default 'qwerty'::text, preferred_language text default 'en'::text, preferred_model text default 'default'::text | fks: user_id->users.id
users: id text, created_at text default anky_now(), username text null, wallet_address text null, privy_did text null, farcaster_fid integer null, farcaster_username text null, farcaster_pfp_url text null, email text null, is_premium integer default 0, premium_since text null, generated_wallet_secret text null, wallet_generated_at text null, is_pro integer default 0
video_projects: id text, user_id text, anky_id text null, writing_session_id text null, script_json text null, status text default 'pending'::text, video_path text null, video_path_720p text null, video_path_360p text null, duration_seconds double precision null default 88, total_scenes integer null default 0, completed_scenes integer null default 0, created_at text default anky_now(), current_step text null default 'script'::text, story_spine text null, payment_tx_hash text null | fks: user_id->users.id
video_recordings: id text, user_id text null, title text null, file_path text null, duration_seconds double precision default 0, status text default 'pending'::text, scene_data text null, created_at text default anky_now()
writing_checkpoints: id bigint default nextval('writing_checkpoints_id_seq'::regclass), session_id text, content text, elapsed_seconds double precision, word_count integer default 0, created_at text default anky_now(), session_token text null
writing_sessions: id text, user_id text, content text, duration_seconds double precision, word_count integer, is_anky integer default 0, response text null, created_at text default anky_now(), keystroke_deltas text null, flow_score double precision null, status text default 'completed'::text, pause_used integer default 0, paused_at text null, resumed_at text null, session_token text null, content_deleted_at text null, anky_response text null, anky_next_prompt text null, anky_mood text null | fks: user_id->users.id
x_conversations: tweet_id text, author_id text, author_username text null, parent_tweet_id text null, mention_text text null, anky_reply_text text null, context_summary text null, created_at text default anky_now()
x_evolution_tasks: id text, tweet_id text, tag text, content text, author text, status text default 'running'::text, summary text null, created_at text default anky_now(), completed_at text null
x_interactions: id text, tweet_id text, x_user_id text null, x_username text null, tweet_text text null, prompt_id text null, status text default 'pending'::text, classification text null, reply_tweet_id text null, created_at text default anky_now(), source text default 'filtered_stream'::text, parent_tweet_id text null, tag text null, extracted_content text null, result_text text null, error_message text null, updated_at text null
x_users: x_user_id text, user_id text, username text, display_name text null, profile_image_url text null, access_token text, refresh_token text null, token_expires_at text null, created_at text default anky_now(), updated_at text default anky_now() | fks: user_id->users.id
```

## 13. Environment / Config

### Core runtime config actually read by the Rust backend

From [config.rs](/home/kithkui/anky/src/config.rs):
- Core server/db: `PORT`, `DATABASE_URL`, `ANKY_MODE`
- Local inference: `OLLAMA_BASE_URL`, `OLLAMA_MODEL`, `OLLAMA_LIGHT_MODEL`, `MIND_URL`
- Cloud inference: `OPENROUTER_API_KEY`, `OPENROUTER_LIGHT_MODEL`, `OPENROUTER_ANKY_MODEL`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `OPENAI_API_KEY`, `XAI_API_KEY`
- Payments: `BASE_RPC_URL`, `USDC_ADDRESS`, `TREASURY_ADDRESS`, `X402_FACILITATOR_URL`, `STRIPE_SECRET_KEY`, `STRIPE_PUBLISHABLE_KEY`
- Auth/social: `TWITTER_CLIENT_ID`, `TWITTER_CLIENT_SECRET`, `TWITTER_CALLBACK_URL`, `X_BEARER_TOKEN`, `X_CONSUMER_KEY`, `X_CONSUMER_SECRET`, `X_ACCESS_TOKEN`, `X_ACCESS_TOKEN_SECRET`, `TWITTER_BOT_USER_ID`, `PRIVY_APP_ID`, `PRIVY_APP_SECRET`, `PRIVY_VERIFICATION_KEY`, `NEYNAR_API_KEY`, `NEYNAR_SIGNER_UUID`, `NEYNAR_WEBHOOK_SECRET`, `FARCASTER_BOT_FID`
- Infra/storage: `REDIS_URL`, `R2_ACCOUNT_ID`, `R2_BUCKET_NAME`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_PUBLIC_URL`, `PINATA_JWT`
- Media/gen: `COMFYUI_URL`, `TTS_BASE_URL`, `FLUX_API_KEY`, `FLUX_SECRET_KEY`
- Honcho: `HONCHO_API_KEY`, `HONCHO_WORKSPACE_ID`, `HONCHO_BASE_URL`
- Solana mint worker: `SOLANA_MINT_WORKER_URL`, `SOLANA_MINT_WORKER_SECRET`, `SOLANA_MERKLE_TREE`, `SOLANA_COLLECTION_MINT`, `SOLANA_AUTHORITY_PUBKEY`
- APNs: `APNS_KEY_PATH`, `APNS_KEY_ID`, `APNS_TEAM_ID`, `APNS_BUNDLE_ID`, `APNS_ENVIRONMENT`
- Misc app config: `TRAINING_SECRET`, `DATASET_PASSWORD`, `ANKY_WALLET_PRIVATE_KEY`, `ANKY_IOS_APP_URL`, `ANKY_ENCLAVE_URL`, `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ZONE_ID`, `PUMPFUN_RTMP_URL`, `PUMPFUN_STREAM_KEY`

### Additional environment names present locally but not part of `Config`

These exist in the local `.env`, but I did not find them consumed by the Rust backend paths I inspected:
- `ANKY_AGENT_API_KEY`
- `ANKY_EVM_PRIVATE_KEY`
- `ANKY_EVM_WALLET`
- `ANKY_SOLANA_WALLET`
- `INSTAGRAM_ACCESS_TOKEN`
- `INSTAGRAM_USER_ID`
- `X_BOT_USER_ID`

### Worker/setup-only envs outside the Rust backend

Used by the Solana worker/setup scripts rather than the Rust server:
- `HELIUS_API_KEY`
- `AUTHORITY_KEYPAIR`
- `ANKYS_MERKLE_TREE`
- `ANKYS_COLLECTION_MINT`
- `SOLANA_NETWORK`
- `SOLANA_RPC_URL`

### External services the system currently connects to

Definitely wired in code:
- Postgres
- Redis/Valkey
- Anthropic
- OpenRouter
- Gemini image generation
- OpenAI embeddings
- X/Twitter OAuth + bot APIs
- Privy
- Neynar/Farcaster
- Honcho
- Base RPC
- Stripe
- x402 facilitator
- Cloudflare R2
- Pinata/IPFS
- Solana Cloudflare mint worker
- APNs
- Local Mind server
- Local Ollama
- Local F5-TTS
- Local ComfyUI/Flux

### Configuration drift / dead config worth knowing before a rewrite

I found several mismatches between declared config and runtime use:
- `COMFYUI_URL` is declared in `Config`, but [services/comfyui.rs](/home/kithkui/anky/src/services/comfyui.rs) currently uses a hardcoded `const COMFYUI_URL = "http://127.0.0.1:8188"`.
- Some Claude/Mind paths read `MIND_URL` directly from the environment instead of consistently using `state.config.mind_url`.
- `PUMPFUN_RTMP_URL`, `PUMPFUN_STREAM_KEY`, `FLUX_API_KEY`, and `FLUX_SECRET_KEY` are in `Config`, but I did not find active runtime call sites using them in the current Rust backend.
- The repo has both conceptual docs (`SOUL.md`) and runtime behavior that diverge substantially; for architectural work, trust the code paths above, not the prose doc.
