# Anky Monorepo Context For Claude

## Workspace / Crates

[`/home/kithkui/anky/Cargo.toml`](/home/kithkui/anky/Cargo.toml)

```text
root crate: anky
cargo workspace members: none
```

## Monorepo Tree (2 Levels Deep From Root)

[`/home/kithkui/anky`](/home/kithkui/anky)

```text
.agents
.agents/skills
.claude
.claude/settings.local.json
.claude/skills
.env
.env.example
.git
.git/COMMIT_EDITMSG
.git/FETCH_HEAD
.git/HEAD
.git/ORIG_HEAD
.git/config
.git/description
.git/hooks
.git/index
.git/info
.git/logs
.git/objects
.git/refs
.gitignore
.railwayignore
.secrets
.secrets/anky_dca.env
.secrets/anky_dca_wallet.json
.venv-dca
.venv-dca/.gitignore
.venv-dca/bin
.venv-dca/include
.venv-dca/lib
.venv-dca/lib64
.venv-dca/pyvenv.cfg
ANKY_SKILL_v7.2.md
ARCHITECTURE_CHANGES.md
ARCHITECTURE_MAP.md
AuthKey_MDCA44YATB.p8
CLAUDE.md
CURRENT_STATE.md
Cargo.lock
Cargo.toml
Dockerfile
IOS_PROMPT_POST_WRITING_FLOW.md
MANIFESTO.md
Makefile
PROMPT.md
README.md
SOUL.md
SWIFT_AGENT_BRIEF.md
THE_ANKY_MODEL.md
UNDERSTANDING_ANKY.md
WHITEPAPER.aux
WHITEPAPER.log
WHITEPAPER.out
WHITEPAPER.pdf
WHITEPAPER.tex
WHITEPAPER.toc
agent-skills
agent-skills/anky
anky.db
autopost.log
contracts
contracts/AnkyMirrors.sol
cursor_2.6.21_amd64.deb
data
data/.anky.pid
data/aky.db
data/anky-images
data/anky.db
data/anky.log
data/create_videos
data/exports
data/generated_training
data/generations
data/images
data/lora_weights
data/mirrors
data/og-dataset-round-two.jpg
data/streams
data/training-images
data/training-live
data/training_runs
data/v1_announcements
data/videos
data/writings
deploy
deploy/README.md
deploy/anky-heart.service
deploy/anky-mind.service
deploy/anky-worker.service
docs
docs/agents
docs/anky-x-presence.md
docs/api-reference
docs/architecture
docs/build.js
docs/concepts
docs/dist
docs/images
docs/internal
docs/introduction
docs/logo
docs/marketing
docs/node_modules
docs/orbiter.json
docs/package-lock.json
docs/package.json
docs/self-hosting
extension
extension/background.js
extension/content.js
extension/icons
extension/manifest.json
extension/popup.html
extension/popup.js
extension/styles.css
flux
flux/experiment-1
flux/experiment-2
flux/experiment-3
handoff
handoff/ankycoin-build-context-2026-04-01.zip
handoff/ankycoin-build-context-tree-2026-04-01.zip
interview-engine
interview-engine/.env
interview-engine/.venv
interview-engine/__pycache__
interview-engine/assets
interview-engine/audio.py
interview-engine/brain.py
interview-engine/compositor.py
interview-engine/memory.db
interview-engine/memory.py
interview-engine/models
interview-engine/requirements.txt
interview-engine/server.py
livestream
livestream/ep2
logs
logs/anky_dca.log
logs/autonomous_post.log
logs/comfyui.log
ls
migrations
migrations/001_init.sql
migrations/002_solana_mirrors.sql
migrations/003_mirror_items.sql
migrations/004_farcaster_notifications.sql
missfont.log
output.log
prompts
prompts/0001.md
prompts/cuentacuentos_system.md
prompts/generate_anky_soul_from_research.md
prompts/run_research_prompt.md
railway.toml
research_outputs
research_prompts
research_prompts/chakras_story_research.md
scripts
scripts/.env
scripts/__pycache__
scripts/anky_dca_buy.py
scripts/anky_instagram_carousel.py
scripts/anky_real_post.py
scripts/ankys_autopost.py
scripts/autonomous_agent_v2.py
scripts/autonomous_anky.py
scripts/autonomous_anky_poster.py
scripts/autonomous_poster.py
scripts/build_instagram_queue.py
scripts/caption_missing_gallery_images.py
scripts/carousel_gen_stdlib.py
scripts/create_agent.py
scripts/create_batch.py
scripts/export_round_two_dataset.py
scripts/generate_anky_day2.py
scripts/generate_batch.py
scripts/generate_landing_gifs.sh
scripts/generate_pitch_deck.py
scripts/generate_stories.py
scripts/generate_training_images.py
scripts/get_instagram_token.py
scripts/instagram_carousel_gen.py
scripts/migrate_sqlite_to_postgres.sh
scripts/recaption_dataset.py
scripts/run_anky_dca.sh
scripts/run_autonomous_ankey.py
scripts/test_flux.py
scripts/test_session_api.py
skills
skills-lock.json
skills.md
skills/colosseum-copilot
slides
slides/ep2
slides/index.html
slides/livestream-slides.html
solana
solana/setup
solana/worker
src
src/ankyverse.rs
src/config.rs
src/create_videos.rs
src/db
src/error.rs
src/kingdoms.rs
src/main.rs
src/memory
src/middleware
src/models
src/pipeline
src/public
src/routes
src/services
src/sse
src/state.rs
src/storage
src/training
static
static/admin
static/agent.json
static/anky-collection.png
static/anky-data-part-aa
static/anky-data-part-ab
static/anky-data-part-ac
static/anky-data-part-ad
static/anky-data-part-ae
static/anky-data-part-af
static/anky-data-part-ag
static/anky-data-part-ah
static/anky-data-part-ai
static/anky-data-part-aj
static/anky-speech-square.mp4
static/anky-speech-video.mp4
static/anky-training-data.tar.gz
static/ankycoin-farcaster.json
static/apple-touch-icon.png
static/autonomous
static/changelog
static/create_videos_prompts.json
static/cuentacuentos
static/dca-bot
static/ep2
static/ethers.umd.min.js
static/farcaster.json
static/fonts
static/hf
static/htmx-sse.js
static/htmx.min.js
static/icon-192.png
static/icon-512.png
static/icon.png
static/inference_server.py
static/livestream-episode-2.html
static/manifest.json
static/mirror-farcaster.json
static/mobile.css
static/og-black.svg
static/og-dataset-round-two.jpg
static/og-pitch-deck.png
static/pitch-deck.pdf
static/pitch-images
static/references
static/solana-agent-registry
static/splash.png
static/style.css
static/sw.js
static/train_anky_setup.sh
static/watcher.py
target
target/.future-incompat-report.json
target/.rustc_info.json
target/CACHEDIR.TAG
target/debug
target/release
target/server.log
templates
templates/anky.html
templates/ankycoin.html
templates/ankycoin_landing.html
templates/base.html
templates/changelog.html
templates/class.html
templates/classes_index.html
templates/collection.html
templates/collection_progress.html
templates/create_videos.html
templates/dashboard.html
templates/dataset_round_two.html
templates/dca.html
templates/dca_bot_code.html
templates/evolve.html
templates/feed.html
templates/feedback.html
templates/gallery.html
templates/generate.html
templates/generations_dashboard.html
templates/generations_list.html
templates/generations_review.html
templates/generations_tinder.html
templates/help.html
templates/home.html
templates/interview.html
templates/landing.html
templates/leaderboard.html
templates/llm.html
templates/login.html
templates/media_dashboard.html
templates/miniapp.html
templates/mint.html
templates/mirror_miniapp.html
templates/mobile.html
templates/pitch-deck.html
templates/pitch.html
templates/poiesis.html
templates/poiesis_log.html
templates/prompt.html
templates/prompt_create.html
templates/prompt_new.html
templates/settings.html
templates/simulations.html
templates/sleeping.html
templates/stories.html
templates/stream_overlay.html
templates/test.html
templates/training.html
templates/training_general_instructions.html
templates/training_live.html
templates/training_run.html
templates/trainings.html
templates/video.html
templates/video_pipeline.html
templates/videos.html
templates/writing_response.html
templates/writings.html
templates/you.html
test_comfy_local.py
test_local.py
test_parallel_fix.py
test_payload.json
tools
tools/ffmpeg-static
tools/ollama-override.conf
training
training/autoresearch
training/prepare_dataset.py
training/requirements.txt
training/test_lora.py
training/train_flux_lora.py
twitter_oauth.py
twitter_oauth_v2.py
videos
videos/0f341deb-43c4-4285-9234-dcdeded40833.mp4
videos/0f341deb-43c4-4285-9234-dcdeded40833__scene_00.mp4
videos/0f341deb-43c4-4285-9234-dcdeded40833__scene_01.mp4
videos/0f341deb-43c4-4285-9234-dcdeded40833__scene_02.mp4
videos/0f341deb-43c4-4285-9234-dcdeded40833__scene_03.mp4
videos/0f341deb-43c4-4285-9234-dcdeded40833__scene_04.mp4
videos/0f341deb-43c4-4285-9234-dcdeded40833__scene_05.mp4
videos/0f341deb-43c4-4285-9234-dcdeded40833__scene_06.mp4
videos/0f341deb-43c4-4285-9234-dcdeded40833__scene_07.mp4
videos/0f341deb-43c4-4285-9234-dcdeded40833__scene_08.mp4
videos/0f341deb-43c4-4285-9234-dcdeded40833__scene_09.mp4
videos/0f341deb-43c4-4285-9234-dcdeded40833__scene_10.mp4
videos/0f341deb-43c4-4285-9234-dcdeded40833__scene_11.mp4
videos/1a5e4365-7238-406c-8c78-488ee472b1f3.mp4
videos/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_00.mp4
videos/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_01.mp4
videos/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_02.mp4
videos/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_03.mp4
videos/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_04.mp4
videos/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_05.mp4
videos/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_06.mp4
videos/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_07.mp4
videos/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_08.mp4
videos/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_09.mp4
videos/1e65a5cb-65f6-4904-a7c3-572efb4b258b.mp4
videos/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_00.mp4
videos/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_01.mp4
videos/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_02.mp4
videos/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_03.mp4
videos/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_04.mp4
videos/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_05.mp4
videos/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_06.mp4
videos/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_07.mp4
videos/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_08.mp4
videos/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_09.mp4
videos/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_10.mp4
videos/20bfada3-c243-47d7-b26e-a15054faaf9b.mp4
videos/20bfada3-c243-47d7-b26e-a15054faaf9b__scene_00.mp4
videos/20bfada3-c243-47d7-b26e-a15054faaf9b__scene_01.mp4
videos/20bfada3-c243-47d7-b26e-a15054faaf9b__scene_02.mp4
videos/20bfada3-c243-47d7-b26e-a15054faaf9b__scene_03.mp4
videos/20bfada3-c243-47d7-b26e-a15054faaf9b__scene_04.mp4
videos/20bfada3-c243-47d7-b26e-a15054faaf9b__scene_05.mp4
videos/20bfada3-c243-47d7-b26e-a15054faaf9b__scene_07.mp4
videos/20bfada3-c243-47d7-b26e-a15054faaf9b__scene_08.mp4
videos/26419b97-de18-446c-a7b0-b7ebd74aeac6.mp4
videos/26419b97-de18-446c-a7b0-b7ebd74aeac6__concat.txt
videos/26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_00.mp4
videos/26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_01.mp4
videos/26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_02.mp4
videos/26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_03.mp4
videos/26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_04.mp4
videos/26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_05.mp4
videos/26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_06.mp4
videos/26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_07.mp4
videos/26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_08.mp4
videos/47c35852-3186-434c-8436-00a9f1558069.mp4
videos/47c35852-3186-434c-8436-00a9f1558069__concat.txt
videos/47c35852-3186-434c-8436-00a9f1558069__scene_00.mp4
videos/47c35852-3186-434c-8436-00a9f1558069__scene_01.mp4
videos/47c35852-3186-434c-8436-00a9f1558069__scene_02.mp4
videos/47c35852-3186-434c-8436-00a9f1558069__scene_03.mp4
videos/47c35852-3186-434c-8436-00a9f1558069__scene_04.mp4
videos/47c35852-3186-434c-8436-00a9f1558069__scene_05.mp4
videos/47c35852-3186-434c-8436-00a9f1558069__scene_06.mp4
videos/47c35852-3186-434c-8436-00a9f1558069__scene_07.mp4
videos/47c35852-3186-434c-8436-00a9f1558069__scene_08.mp4
videos/5557ff52-b256-410f-bb89-d764e47bf5fb.mp4
videos/5557ff52-b256-410f-bb89-d764e47bf5fb__scene_00.mp4
videos/5557ff52-b256-410f-bb89-d764e47bf5fb__scene_01.mp4
videos/5557ff52-b256-410f-bb89-d764e47bf5fb__scene_02.mp4
videos/5557ff52-b256-410f-bb89-d764e47bf5fb__scene_03.mp4
videos/5557ff52-b256-410f-bb89-d764e47bf5fb__scene_04.mp4
videos/5557ff52-b256-410f-bb89-d764e47bf5fb__scene_05.mp4
videos/5557ff52-b256-410f-bb89-d764e47bf5fb__scene_06.mp4
videos/5557ff52-b256-410f-bb89-d764e47bf5fb__scene_07.mp4
videos/60c569da-3cfc-41ae-a29f-8f4d2b740c2b.mp4
videos/60c569da-3cfc-41ae-a29f-8f4d2b740c2b__concat.txt
videos/60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_00.mp4
videos/60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_01.mp4
videos/60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_02.mp4
videos/60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_03.mp4
videos/60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_04.mp4
videos/60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_05.mp4
videos/60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_06.mp4
videos/60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_07.mp4
videos/60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_08.mp4
videos/60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_09.mp4
videos/6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e.mp4
videos/6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_00.mp4
videos/6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_01.mp4
videos/6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_02.mp4
videos/6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_03.mp4
videos/6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_04.mp4
videos/6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_05.mp4
videos/6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_06.mp4
videos/6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_07.mp4
videos/6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_08.mp4
videos/6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_09.mp4
videos/6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_10.mp4
videos/6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_11.mp4
videos/6ed7d5de-94d4-4298-a65b-e939f5ba8971.mp4
videos/6ed7d5de-94d4-4298-a65b-e939f5ba8971__concat.txt
videos/6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_00.mp4
videos/6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_01.mp4
videos/6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_02.mp4
videos/6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_03.mp4
videos/6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_04.mp4
videos/6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_05.mp4
videos/6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_06.mp4
videos/6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_07.mp4
videos/8bfae113-5e60-4bdc-af0b-3c5f5c806e47.mp4
videos/8bfae113-5e60-4bdc-af0b-3c5f5c806e47__scene_00.mp4
videos/8bfae113-5e60-4bdc-af0b-3c5f5c806e47__scene_01.mp4
videos/8bfae113-5e60-4bdc-af0b-3c5f5c806e47__scene_03.mp4
videos/8bfae113-5e60-4bdc-af0b-3c5f5c806e47__scene_04.mp4
videos/8bfae113-5e60-4bdc-af0b-3c5f5c806e47__scene_06.mp4
videos/8bfae113-5e60-4bdc-af0b-3c5f5c806e47__scene_07.mp4
videos/8bfae113-5e60-4bdc-af0b-3c5f5c806e47__scene_08.mp4
videos/94e49660-2bb2-44a8-b814-20ac666a61cd.mp4
videos/94e49660-2bb2-44a8-b814-20ac666a61cd__concat.txt
videos/94e49660-2bb2-44a8-b814-20ac666a61cd__scene_00.mp4
videos/94e49660-2bb2-44a8-b814-20ac666a61cd__scene_01.mp4
videos/94e49660-2bb2-44a8-b814-20ac666a61cd__scene_02.mp4
videos/94e49660-2bb2-44a8-b814-20ac666a61cd__scene_03.mp4
videos/94e49660-2bb2-44a8-b814-20ac666a61cd__scene_04.mp4
videos/94e49660-2bb2-44a8-b814-20ac666a61cd__scene_05.mp4
videos/94e49660-2bb2-44a8-b814-20ac666a61cd__scene_06.mp4
videos/94e49660-2bb2-44a8-b814-20ac666a61cd__scene_07.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__final.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_00.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_01.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_02.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_03.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_04.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_05.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_06.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_07.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_08.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_09.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_10.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_11.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_12.mp4
videos/af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_13.mp4
videos/anky-speech
videos/ba9eab3c-230d-45c4-b0dd-ad4564073630.mp4
videos/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_00.mp4
videos/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_01.mp4
videos/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_02.mp4
videos/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_03.mp4
videos/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_04.mp4
videos/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_05.mp4
videos/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_06.mp4
videos/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_07.mp4
videos/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_08.mp4
videos/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_09.mp4
videos/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_10.mp4
videos/e8985306-28e5-4655-b73e-e2d12c46837b.mp4
videos/e8985306-28e5-4655-b73e-e2d12c46837b__scene_00.mp4
videos/e8985306-28e5-4655-b73e-e2d12c46837b__scene_01.mp4
videos/e8985306-28e5-4655-b73e-e2d12c46837b__scene_02.mp4
videos/e8985306-28e5-4655-b73e-e2d12c46837b__scene_03.mp4
videos/e8985306-28e5-4655-b73e-e2d12c46837b__scene_04.mp4
videos/e8985306-28e5-4655-b73e-e2d12c46837b__scene_05.mp4
videos/e8985306-28e5-4655-b73e-e2d12c46837b__scene_06.mp4
videos/e8985306-28e5-4655-b73e-e2d12c46837b__scene_07.mp4
videos/e8985306-28e5-4655-b73e-e2d12c46837b__scene_08.mp4
videos/e8985306-28e5-4655-b73e-e2d12c46837b__scene_09.mp4
videos/e8985306-28e5-4655-b73e-e2d12c46837b__scene_10.mp4
videos/e8985306-28e5-4655-b73e-e2d12c46837b__scene_11.mp4
videos/e992abb7-c3b2-4d76-8253-df43ea9d171a.mp4
videos/e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_00.mp4
videos/e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_01.mp4
videos/e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_03.mp4
videos/e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_04.mp4
videos/e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_05.mp4
videos/e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_06.mp4
videos/e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_07.mp4
videos/e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_08.mp4
```

## Source Tree

[`/home/kithkui/anky/src`](/home/kithkui/anky/src)

```text
src
src/ankyverse.rs
src/config.rs
src/create_videos.rs
src/db
src/db/mod.rs
src/db/queries.rs
src/error.rs
src/kingdoms.rs
src/main.rs
src/memory
src/memory/embeddings.rs
src/memory/extraction.rs
src/memory/mod.rs
src/memory/profile.rs
src/memory/recall.rs
src/middleware
src/middleware/api_auth.rs
src/middleware/honeypot.rs
src/middleware/mod.rs
src/middleware/security_headers.rs
src/middleware/subdomain.rs
src/middleware/x402.rs
src/models
src/models/anky_story.rs
src/models/mod.rs
src/pipeline
src/pipeline/collection.rs
src/pipeline/cost.rs
src/pipeline/guidance_gen.rs
src/pipeline/image_gen.rs
src/pipeline/memory_pipeline.rs
src/pipeline/mod.rs
src/pipeline/prompt_gen.rs
src/pipeline/stream_gen.rs
src/pipeline/video_gen.rs
src/public
src/public/anky-1.png
src/public/anky-2.png
src/public/anky-3.png
src/routes
src/routes/api.rs
src/routes/auth.rs
src/routes/collection.rs
src/routes/dashboard.rs
src/routes/evolve.rs
src/routes/extension_api.rs
src/routes/generations.rs
src/routes/health.rs
src/routes/interview.rs
src/routes/live.rs
src/routes/mod.rs
src/routes/notification.rs
src/routes/pages.rs
src/routes/payment.rs
src/routes/payment_helper.rs
src/routes/poiesis.rs
src/routes/prompt.rs
src/routes/session.rs
src/routes/settings.rs
src/routes/simulations.rs
src/routes/social_context.rs
src/routes/swift.rs
src/routes/training.rs
src/routes/voices.rs
src/routes/webhook_farcaster.rs
src/routes/webhook_x.rs
src/routes/writing.rs
src/services
src/services/apns.rs
src/services/claude.rs
src/services/comfyui.rs
src/services/gemini.rs
src/services/grok.rs
src/services/hermes.rs
src/services/honcho.rs
src/services/mind.rs
src/services/mod.rs
src/services/neynar.rs
src/services/notification.rs
src/services/ollama.rs
src/services/openrouter.rs
src/services/payment.rs
src/services/push_scheduler.rs
src/services/r2.rs
src/services/redis_queue.rs
src/services/stream.rs
src/services/tts.rs
src/services/twitter.rs
src/services/wallet.rs
src/services/x_bot.rs
src/sse
src/sse/logger.rs
src/sse/mod.rs
src/state.rs
src/storage
src/storage/files.rs
src/storage/mod.rs
src/training
src/training/dataset.rs
src/training/mod.rs
src/training/orchestrator.rs
src/training/runner.rs
src/training/schedule.rs
```

## Frontend Tree: templates

[`/home/kithkui/anky/templates`](/home/kithkui/anky/templates)

```text
templates
templates/anky.html
templates/ankycoin.html
templates/ankycoin_landing.html
templates/base.html
templates/changelog.html
templates/class.html
templates/classes_index.html
templates/collection.html
templates/collection_progress.html
templates/create_videos.html
templates/dashboard.html
templates/dataset_round_two.html
templates/dca.html
templates/dca_bot_code.html
templates/evolve.html
templates/feed.html
templates/feedback.html
templates/gallery.html
templates/generate.html
templates/generations_dashboard.html
templates/generations_list.html
templates/generations_review.html
templates/generations_tinder.html
templates/help.html
templates/home.html
templates/interview.html
templates/landing.html
templates/leaderboard.html
templates/llm.html
templates/login.html
templates/media_dashboard.html
templates/miniapp.html
templates/mint.html
templates/mirror_miniapp.html
templates/mobile.html
templates/pitch-deck.html
templates/pitch.html
templates/poiesis.html
templates/poiesis_log.html
templates/prompt.html
templates/prompt_create.html
templates/prompt_new.html
templates/settings.html
templates/simulations.html
templates/sleeping.html
templates/stories.html
templates/stream_overlay.html
templates/test.html
templates/training.html
templates/training_general_instructions.html
templates/training_live.html
templates/training_run.html
templates/trainings.html
templates/video.html
templates/video_pipeline.html
templates/videos.html
templates/writing_response.html
templates/writings.html
templates/you.html
```

## Frontend Tree: static

[`/home/kithkui/anky/static`](/home/kithkui/anky/static)

```text
static
static/admin
static/admin/flux-lab.html
static/admin/media-factory.html
static/admin/story-tester.html
static/agent.json
static/anky-collection.png
static/anky-data-part-aa
static/anky-data-part-ab
static/anky-data-part-ac
static/anky-data-part-ad
static/anky-data-part-ae
static/anky-data-part-af
static/anky-data-part-ag
static/anky-data-part-ah
static/anky-data-part-ai
static/anky-data-part-aj
static/anky-speech-square.mp4
static/anky-speech-video.mp4
static/anky-training-data.tar.gz
static/ankycoin-farcaster.json
static/apple-touch-icon.png
static/autonomous
static/autonomous/.png
static/autonomous/20260310_091533.png
static/autonomous/20260311_210958.png
static/autonomous/20260311_211312.png
static/autonomous/20260312_123859_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
static/autonomous/20260312_123859_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
static/autonomous/20260312_123859_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
static/autonomous/20260312_123859_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
static/autonomous/20260312_124012_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
static/autonomous/20260312_124012_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
static/autonomous/20260312_124012_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
static/autonomous/20260312_124012_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
static/autonomous/20260312_124108_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
static/autonomous/20260312_124108_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
static/autonomous/20260312_124108_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
static/autonomous/20260312_124108_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
static/autonomous/20260312_124143_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
static/autonomous/20260312_124143_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
static/autonomous/20260312_124143_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
static/autonomous/20260312_124143_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
static/autonomous/20260312_124218_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
static/autonomous/20260312_124218_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
static/autonomous/20260312_124218_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
static/autonomous/20260312_124218_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
static/autonomous/20260312_124305_cbd512da-65ab-4ed6-9020-f94e7869f242.png
static/autonomous/20260312_125809_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
static/autonomous/20260312_125809_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
static/autonomous/20260312_125809_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
static/autonomous/20260312_125809_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
static/autonomous/20260312_125817_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
static/autonomous/20260312_125817_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
static/autonomous/20260312_125817_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
static/autonomous/20260312_125817_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
static/autonomous/20260312_125949_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
static/autonomous/20260312_125949_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
static/autonomous/20260312_125949_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
static/autonomous/20260312_125949_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
static/autonomous/20260312_130131_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
static/autonomous/20260312_130131_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
static/autonomous/20260312_130131_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
static/autonomous/20260312_130131_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
static/autonomous/20260312_130141_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
static/autonomous/20260312_130141_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
static/autonomous/20260312_130141_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
static/autonomous/20260312_130141_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
static/autonomous/20260312_210316.png
static/autonomous/20260313_210242.png
static/autonomous/20260314_090321.png
static/autonomous/20260314_210341.png
static/autonomous/20260315_101651.png
static/autonomous/20260315_212655.png
static/autonomous/20260315_213041.png
static/autonomous/20260316_104544.png
static/autonomous/20260316_131312_cbd512da-65ab-4ed6-9020-f94e7869f242.png
static/autonomous/20260316_131516_cbd512da-65ab-4ed6-9020-f94e7869f242.png
static/autonomous/20260317_090230.png
static/autonomous/20260318_105558.png
static/autonomous/20260318_210348.png
static/autonomous/20260318_210503_68893970.png
static/autonomous/20260318_210526_68893970.png
static/autonomous/20260320_142216_b8c7dc6d-153f-4c99-945b-025b9de841e1.png
static/autonomous/20260320_143946_b8c7dc6d-153f-4c99-945b-025b9de841e1.png
static/autonomous/20260320_144504_c1d73dbf-67a2-4b6f-a43c-9a9e250148bc.png
static/autonomous/20260320_144959_cbd512da-65ab-4ed6-9020-f94e7869f242.png
static/autonomous/20260320_145503_cbd512da-65ab-4ed6-9020-f94e7869f242.png
static/autonomous/20260320_154836_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide01.jpg
static/autonomous/20260320_154836_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide02.jpg
static/autonomous/20260320_154836_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide03.jpg
static/autonomous/20260320_154836_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide04.jpg
static/autonomous/20260320_155448_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide01.jpg
static/autonomous/20260320_155448_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide02.jpg
static/autonomous/20260320_155448_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide03.jpg
static/autonomous/20260320_155448_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide04.jpg
static/autonomous/20260326_213330_anky.png
static/autonomous/20260327_090609_anky.png
static/autonomous/20260327_160156_anky.png
static/autonomous/20260328_160144_anky.png
static/autonomous/anky_742_f04ab63e.json
static/autonomous/x_post_742.txt
static/changelog
static/changelog/2026-02-14-001-video-studio.txt
static/changelog/2026-02-14-002-paid-image-gen.txt
static/changelog/2026-02-14-003-x402-only.txt
static/changelog/2026-02-14-004-changelog.txt
static/changelog/2026-02-14-005-post-writing-ux.txt
static/changelog/2026-02-14-006-ux-overhaul.txt
static/changelog/2026-02-14-007-prompt-api-agents.txt
static/changelog/2026-02-15-001-remove-balance-payments.txt
static/changelog/2026-02-15-002-stream-overlay.txt
static/changelog/2026-02-15-003-livestream-overhaul.txt
static/changelog/2026-02-15-004-claim-username-modal.txt
static/changelog/2026-02-15-005-livestream-hardstop-congrats.txt
static/changelog/2026-02-15-006-write-rate-limit.txt
static/changelog/2026-02-15-007-farcaster-miniapp.txt
static/changelog/2026-02-15-008-farcaster-sdk-ready-images.txt
static/changelog/2026-02-15-009-bottom-live-bar-waiting-room.txt
static/changelog/2026-02-16-001-live-writing-ux-fixes.txt
static/changelog/2026-02-16-002-write-api-key-required.txt
static/changelog/2026-02-16-003-fix-anky-ca.txt
static/changelog/2026-02-16-004-livestream-watchdog.txt
static/changelog/2026-02-16-005-progressive-web-app.txt
static/changelog/2026-02-17-001-flow-score-leaderboard-chakra-pitch.txt
static/changelog/2026-02-17-002-no-cache-html-routes.txt
static/changelog/2026-02-17-003-video-pipeline-grok.txt
static/changelog/2026-02-17-004-fix-video-script-truncation.txt
static/changelog/2026-02-17-005-memory-enriched-video-pipeline.txt
static/changelog/2026-02-17-006-video-studio-filmstrip-parallel.txt
static/changelog/2026-02-18-001-video-pipeline-overhaul.txt
static/changelog/2026-02-18-002-vertical-video-continuity-cost.txt
static/changelog/2026-02-18-003-post-session-video-button.txt
static/changelog/2026-02-18-004-keyboard-first-feed-app.txt
static/changelog/2026-02-18-005-desktop-mobile-split.txt
static/changelog/2026-02-18-006-revert-keyboard-ui.txt
static/changelog/2026-02-19-001-vertical-story-driven-video.txt
static/changelog/2026-02-19-002-fix-chat-ui-anon-cookie.txt
static/changelog/2026-02-19-003-phantom-solana-login.txt
static/changelog/2026-02-19-004-email-social-login.txt
static/changelog/2026-02-19-005-infinite-media-slideshow.txt
static/changelog/2026-02-19-006-anky-tv-drawer-nav.txt
static/changelog/2026-02-19-007-video-playback-slideshow.txt
static/changelog/2026-02-19-008-feed-page.txt
static/changelog/2026-02-20-001-disable-livestream.txt
static/changelog/2026-02-20-002-fix-chat-textarea.txt
static/changelog/2026-02-20-003-memetics-wtf-homepage.txt
static/changelog/2026-02-21-001-meditation-first-experience.txt
static/changelog/2026-02-21-002-video-studio-mobile-ux.txt
static/changelog/2026-02-21-003-sequential-chain-video.txt
static/changelog/2026-02-21-004-psychoanalytic-director-prompt.txt
static/changelog/2026-02-21-005-dissolve-writing-friction.txt
static/changelog/2026-02-21-006-writing-first-homepage.txt
static/changelog/2026-02-21-007-fab-fix-inquiry-system.txt
static/changelog/2026-02-21-008-remove-meditation-fab.txt
static/changelog/2026-02-21-009-spanish-reflection-mobile-fix.txt
static/changelog/2026-02-21-010-settings-wallet-writings-scroll-video.txt
static/changelog/2026-02-22-001-fix-scroll-all-routes.txt
static/changelog/2026-02-25-001-fix-lost-writing-save-before-ollama.txt
static/changelog/2026-02-25-002-privy-auth-fallback-video-resilience.txt
static/changelog/2026-02-26-001-fix-video-payment-parallel-pipeline.txt
static/changelog/2026-02-26-002-anon-user-localstorage-persistence.txt
static/changelog/2026-02-26-003-suggested-replies-scroll-fix.txt
static/changelog/2026-02-26-004-interview-system-integration.txt
static/changelog/2026-02-26-005-live-interview-rename-reset.txt
static/changelog/2026-02-26-006-stt-timeout-protection.txt
static/changelog/2026-02-26-007-video-pipeline-story-spine.txt
static/changelog/2026-02-27-001-fix-video-payment-timeout.txt
static/changelog/2026-02-27-002-sharper-inquiry-prompts.txt
static/changelog/2026-02-27-003-structured-reflection-format.txt
static/changelog/2026-02-27-003-training-curation-tinder.txt
static/changelog/2026-02-27-004-kill-livestream.txt
static/changelog/2026-02-27-005-fix-torchaudio-training.txt
static/changelog/2026-02-28-001-trainings-journal.txt
static/changelog/2026-02-28-002-story-first-video-prompt.txt
static/changelog/2026-02-28-003-jpeg-images-for-xai-video.txt
static/changelog/2026-02-28-004-training-general-instructions.txt
static/changelog/2026-02-28-005-videos-gallery.txt
static/changelog/2026-02-28-006-fix-write-error-draft-loss.txt
static/changelog/2026-02-28-007-farcaster-miniapp-user-id-fallback.txt
static/changelog/2026-03-01-001-flux-lora-free-image-gen.txt
static/changelog/2026-03-01-002-flux-raw-prompt-anky-validation.txt
static/changelog/2026-03-01-003-flux-ux-thinker-fix-prompt-hint.txt
static/changelog/2026-03-02-001-anky-speech-video.txt
static/changelog/2026-03-03-001-fix-streaming-layout-and-reply-buttons.txt
static/changelog/2026-03-03-002-simplify-flux-anky-validation.txt
static/changelog/2026-03-03-003-anky-lora-recaption-and-dataset-gen.txt
static/changelog/2026-03-03-004-switch-to-qwen35-35b-moe.txt
static/changelog/2026-03-03-005-tinder-image-review-ui.txt
static/changelog/2026-03-03-006-training-run-2-dataset-pipeline.txt
static/changelog/2026-03-03-007-sharpen-reflection-prompts-ramana-jed.txt
static/changelog/2026-03-03-008-local-embeddings-nomic-embed-text.txt
static/changelog/2026-03-03-009-move-all-non-reflection-to-local-qwen.txt
static/changelog/2026-03-03-010-dataset-round-two-gallery.txt
static/changelog/2026-03-04-001-runpod-training-bootstrap-hardening.txt
static/changelog/2026-03-04-002-well-known-agent.txt
static/changelog/2026-03-04-003-round-two-runbook-and-v2-serving.txt
static/changelog/2026-03-04-004-switch-to-qwen3-5-35b.txt
static/changelog/2026-03-04-005-x-webhook-crc-comfyui-mention-reply.txt
static/changelog/2026-03-04-006-x-webhook-image-rate-limit.txt
static/changelog/2026-03-05-001-polling-loop-new-webhook-logic.txt
static/changelog/2026-03-05-002-webhook-log-viewer.txt
static/changelog/2026-03-05-003-x-filtered-stream.txt
static/changelog/2026-03-05-004-fix-stuck-reading-screen-remove-hackathon-banner.txt
static/changelog/2026-03-05-005-prefetch-memory-context.txt
static/changelog/2026-03-06-001-leaderboard-styling.txt
static/changelog/2026-03-06-002-remove-nav-links.txt
static/changelog/2026-03-06-003-x-reply-context.txt
static/changelog/2026-03-07-001-gemini-flux-fallback.txt
static/changelog/2026-03-07-002-swift-mobile-api.txt
static/changelog/2026-03-07-003-personalized-guidance-queue.txt
static/changelog/2026-03-07-004-facilitator-marketplace.txt
static/changelog/2026-03-07-005-swift-agent-brief-understanding-whitepaper.txt
static/changelog/2026-03-08-001-flux-lora-trigger-word.txt
static/changelog/2026-03-08-002-autoresearch-llm-pipeline.txt
static/changelog/2026-03-09-001-anky-reply-identity.txt
static/changelog/2026-03-09-002-x-tag-hermes-bridge.txt
static/changelog/2026-03-09-003-evolve-dashboard-deploy.txt
static/changelog/2026-03-09-004-evolve-trace-and-x-fixes.txt
static/changelog/2026-03-09-005-reflection-memory-skills-language.txt
static/changelog/2026-03-10-001-agent-native-skill-evolution.txt
static/changelog/2026-03-10-002-everything-free.txt
static/changelog/2026-03-10-003-farcaster-bot-integration.txt
static/changelog/2026-03-12-001-rescue-writing-ownership-bug.txt
static/changelog/2026-03-13-001-pitch-subdomain-pdf.txt
static/changelog/2026-03-13-002-prompt-background-image.txt
static/changelog/2026-03-13-003-prompt-link-and-formatted-writing.txt
static/changelog/2026-03-15-001-radical-writing-ux.txt
static/changelog/2026-03-16-001-simplify-paused-screen.txt
static/changelog/2026-03-17-001-simple-og-metadata.txt
static/changelog/2026-03-18-001-ankyverse-stories.txt
static/changelog/2026-03-18-002-unify-mobile-write-api.txt
static/changelog/2026-03-18-003-honcho-identity-modeling.txt
static/changelog/2026-03-19-001-system-summaries.txt
static/changelog/2026-03-19-002-remove-sadhana-meditation-breathwork-facilitators.txt
static/changelog/2026-03-19-003-mobile-next-prompt-you-device-token.txt
static/changelog/2026-03-19-004-web-seed-auth.txt
static/changelog/2026-03-19-005-mobile-web-design-system.txt
static/changelog/2026-03-19-006-match-story-language-to-writing.txt
static/changelog/2026-03-19-007-soul-document-story-pipeline.txt
static/changelog/2026-03-20-001-swap-story-to-local-qwen-gpu-priority-queue.txt
static/changelog/2026-03-20-002-ritual-lifecycle.txt
static/changelog/2026-03-20-003-anky-voices-backend.txt
static/changelog/2026-03-21-001-fix-mobile-api-gaps.txt
static/changelog/2026-03-21-002-push-notifications.txt
static/changelog/2026-03-21-002-tts-pipeline.txt
static/changelog/2026-03-22-001-social-reply-pipeline-context.txt
static/changelog/2026-03-22-002-anky-talks-back.txt
static/changelog/2026-03-22-003-minting-endpoints.txt
static/changelog/2026-03-23-001-fix-csp-writing-ux.txt
static/changelog/2026-03-24-001-thread-based-ux-redesign.txt
static/changelog/2026-03-24-002-landing-page-write-route-prompts.txt
static/changelog/2026-03-24-003-manifesto-route.txt
static/changelog/2026-03-24-004-simplify-writing-ux.txt
static/changelog/2026-03-24-005-timer-viewport-enter.txt
static/changelog/2026-03-24-006-idle-bar-pause-resume.txt
static/changelog/2026-03-24-007-enter-send-live-nudges.txt
static/changelog/2026-03-24-008-model-selector-settings.txt
static/changelog/2026-03-24-008-thread-splitting-profile-grid.txt
static/changelog/2026-03-24-009-remove-miniapp-uuid-prompts.txt
static/changelog/2026-03-25-001-chat-bubble-post-writing-ux.txt
static/changelog/2026-03-25-002-universal-links-prompt-endpoint.txt
static/changelog/2026-03-25-003-never-lose-reflection.txt
static/changelog/2026-03-26-001-fix-post-writing-ux-streaming.txt
static/changelog/2026-03-26-002-landing-inline-writing.txt
static/changelog/2026-03-26-003-flux-lab-batch-image-gen.txt
static/changelog/2026-03-26-003-flux-media-factory.txt
static/changelog/2026-03-26-003-replace-ollama-with-haiku.txt
static/changelog/2026-03-26-004-fix-reflection-streaming.txt
static/changelog/2026-03-26-004-ollama-to-cloud.txt
static/changelog/2026-03-27-001-fix-reflection-streaming-warm-context.txt
static/changelog/2026-03-27-002-chat-interface-post-writing.txt
static/changelog/2026-03-27-003-chat-navbar-privy-login.txt
static/changelog/2026-03-27-004-chat-first-privy-login-profile.txt
static/changelog/2026-03-27-004-fix-flux-aspect-ratio.txt
static/changelog/2026-03-27-005-stories-desktop-tap-zones.txt
static/changelog/2026-03-27-006-history-prompts-login-fix.txt
static/changelog/2026-03-27-007-ankycoin-landing-page.txt
static/changelog/2026-03-27-008-mirror-endpoint-ankycoin-miniapp.txt
static/changelog/2026-03-27-009-ankycoin-website-landing.txt
static/changelog/2026-03-27-010-ankycoin-image-generator.txt
static/changelog/2026-03-27-011-forge-first-mobile-optimize.txt
static/changelog/2026-03-27-012-mirror-gallery-fid-lookup.txt
static/changelog/2026-03-27-013-mirror-cache-chat.txt
static/changelog/2026-03-27-013-two-line-farcaster-replies.txt
static/changelog/2026-03-27-014-evolved-mirror-frame-image.txt
static/changelog/2026-03-27-014-miniapp-profile-page.txt
static/changelog/2026-03-28-001-anky-page-redesign-anky-mode.txt
static/changelog/2026-03-28-001-mirror-mint-nft-contract.txt
static/changelog/2026-03-28-002-openrouter-fallback-session-summary.txt
static/changelog/2026-03-28-003-r2-cdn-anky-story.txt
static/changelog/2026-03-29-001-farcaster-community-writing-prompts.txt
static/changelog/2026-03-29-002-programming-classes-smart-detection.txt
static/changelog/2026-03-29-003-local-first-mind-kingdoms.txt
static/create_videos_prompts.json
static/cuentacuentos
static/dca-bot
static/dca-bot/anky_dca_buy.py
static/dca-bot/install.sh
static/dca-bot/log_monitor.py
static/dca-bot/run_anky_dca.sh
static/ep2
static/ep2/index.html
static/ethers.umd.min.js
static/farcaster.json
static/fonts
static/fonts/Righteous-Regular.ttf
static/hf
static/hf/anky-flux-lora-v1-readme.md
static/hf/anky-flux-lora-v2-readme.md
static/hf/download-samples.sh
static/hf/upload-checkpoints.py
static/htmx-sse.js
static/htmx.min.js
static/icon-192.png
static/icon-512.png
static/icon.png
static/inference_server.py
static/livestream-episode-2.html
static/manifest.json
static/mirror-farcaster.json
static/mobile.css
static/og-black.svg
static/og-dataset-round-two.jpg
static/og-pitch-deck.png
static/pitch-deck.pdf
static/pitch-images
static/pitch-images/12cc69de-9cb1-4ff0-ac04-a23fd50e02f0.jpg
static/pitch-images/1a877907-65ec-46ad-a744-1fc1390ee822.jpg
static/pitch-images/5666069c-d519-41f4-8787-0dcc6c17a935.jpg
static/pitch-images/72a11b6e-3a25-451c-977a-8d5c39dd78f0.jpg
static/pitch-images/821d5d32-dd04-4eb5-bb4e-b4f8f7bc01c5.jpg
static/pitch-images/8d49ffe9-616b-4b50-81cd-5e049d11db52.jpg
static/pitch-images/9dd99459-1cc4-47f6-9b44-31e13656f6ca.jpg
static/pitch-images/cbd512da-65ab-4ed6-9020-f94e7869f242.jpg
static/pitch-images/d5525129-55d7-4815-8e0a-7f911c736690.jpg
static/pitch-images/ef481b14-9381-4c9e-a0f0-a27b8ffd1b96.jpg
static/pitch-images/fba2d4fe-7aba-44c6-ba82-fa5a4351fe68.jpg
static/references
static/references/anky-1.png
static/references/anky-2.png
static/references/anky-3.png
static/solana-agent-registry
static/solana-agent-registry/all_domains.json
static/solana-agent-registry/all_skills.json
static/solana-agent-registry/index.html
static/splash.png
static/style.css
static/sw.js
static/train_anky_setup.sh
static/watcher.py
```

## Frontend Tree: src/public

[`/home/kithkui/anky/src/public`](/home/kithkui/anky/src/public)

```text
src/public
src/public/anky-1.png
src/public/anky-2.png
src/public/anky-3.png
```

## Root Cargo.toml

[`/home/kithkui/anky/Cargo.toml`](/home/kithkui/anky/Cargo.toml)

```toml
[package]
name = "anky"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = { version = "0.8", features = ["macros", "ws", "multipart"] }
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["fs", "cors", "compression-gzip", "set-header", "limit"] }
sha2 = "0.10"
hex = "0.4"
rand = "0.8"
tera = "1"
reqwest = { version = "0.12", features = ["json", "stream", "multipart"] }
tokio-stream = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio-cron-scheduler = "0.13"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
base64 = "0.22"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
thiserror = "2"
anyhow = "1"
dotenvy = "0.15"
tower = "0.5"
tower-cookies = "0.10"
axum-extra = { version = "0.10", features = ["cookie"] }
async-stream = "0.3"
futures = "0.3"
futures-util = "0.3"
tokio-tungstenite = { version = "0.26", features = ["native-tls"] }
bytes = "1"
time = "0.3"
image = "0.25"
ab_glyph = "0.2"
imageproc = "0.25"
hmac = { version = "0.12", features = ["std"] }
sha1 = "0.10"
jsonwebtoken = "9"
urlencoding = "2"
secp256k1 = { version = "0.29", features = ["recovery", "rand", "global-context"] }
ed25519-dalek = { version = "2", features = ["std"] }
bs58 = "0.5"
sha3 = "0.10"
aws-sdk-s3 = "1"
aws-config = "1"
webp = "0.3"
aws-credential-types = "1"
a2 = "0.10"
redis = { version = "0.25", features = ["tokio-comp"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "chrono", "json"] }
```

## Axum Server Entry

[`/home/kithkui/anky/src/main.rs`](/home/kithkui/anky/src/main.rs)

```rust
    tracing::info!("Connecting to database at {}", config.database_url);
    let db_pool = db::create_pool(&config.database_url).await?;
    tracing::info!("Database initialized");

    // Load templates
    let tera = tera::Tera::new("templates/**/*.html")?;
    tracing::info!(
        "Templates loaded: {:?}",
        tera.get_template_names().collect::<Vec<_>>()
    );

    // SSE broadcast channel
    let (log_tx, _) = broadcast::channel::<sse::logger::LogEntry>(1000);

    // Webhook event log channel
    let (webhook_log_tx, _) = broadcast::channel::<String>(200);

    // Live streaming state
    let (live_status_tx, _) = broadcast::channel::<state::LiveStatusEvent>(100);
    let (live_text_tx, _) = broadcast::channel::<state::LiveTextEvent>(100);

    // Frame buffer for Rust-rendered livestream frames
    let frame_buffer = services::stream::new_frame_buffer();

    // Build state
    let state = AppState {
        db: db_pool,
        tera: Arc::new(tera),
        config: Arc::new(config),
        gpu_status: Arc::new(RwLock::new(GpuStatus::Idle)),
        log_tx,
        live_state: Arc::new(RwLock::new(state::LiveState::default())),
        live_status_tx,
        live_text_tx,
        frame_buffer,
        write_limiter: state::RateLimiter::new(5, std::time::Duration::from_secs(600)),
        waiting_room: Arc::new(RwLock::new(VecDeque::new())),
        image_limiter: state::RateLimiter::new(1, std::time::Duration::from_secs(300)),
```

[`/home/kithkui/anky/src/main.rs`](/home/kithkui/anky/src/main.rs)

```rust
    // Build router
    let app = routes::build_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("Listening on 0.0.0.0:{}", port);

    axum::serve(listener, app).await?;

    Ok(())
}
```

## Main Route That Serves anky.app

[`/home/kithkui/anky/src/routes/pages.rs`](/home/kithkui/anky/src/routes/pages.rs)

```rust
static MINIAPP_HTML: &str = include_str!("../../templates/miniapp.html");

pub async fn home(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<(CookieJar, Html<String>), AppError> {
    // Serve the miniapp when loaded inside a Farcaster frame
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let sec_dest = headers
        .get("sec-fetch-dest")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if ua.contains("Farcaster") || sec_dest == "iframe" {
        return Ok((jar, Html(MINIAPP_HTML.to_string())));
    }
    // Set anonymous cookie on first visit
    let jar = if jar.get("anky_user_id").is_none() {
        let id = uuid::Uuid::new_v4().to_string();
        let cookie = axum_extra::extract::cookie::Cookie::build(("anky_user_id", id))
            .max_age(time::Duration::days(365))
            .http_only(false)
            .same_site(tower_cookies::cookie::SameSite::Lax)
            .path("/")
            .build();
        jar.add(cookie)
    } else {
        jar
    };

    let user = crate::routes::auth::get_auth_user(&state, &jar).await;
    let mut ctx = tera::Context::new();
    ctx.insert("privy_app_id", &state.config.privy_app_id);
    ctx.insert("logged_in", &user.is_some());
    if let Some(ref u) = user {
        ctx.insert("user_id", &u.user_id);
        ctx.insert("username", &u.username.as_deref().unwrap_or("anon"));
        ctx.insert(
            "profile_image_url",
            &u.profile_image_url
                .as_deref()
                .unwrap_or("/static/icon-192.png"),
        );
    }
    let html = state.tera.render("landing.html", &ctx)?;
    Ok((jar, Html(html)))
}
```

## Route Registration

[`/home/kithkui/anky/src/routes/mod.rs`](/home/kithkui/anky/src/routes/mod.rs)

```rust
    Router::new()
        // Pages
        .route("/", axum::routing::get(pages::home))
        .route("/write", axum::routing::get(pages::write_page))
        .route("/stories", axum::routing::get(pages::stories_page))
        .route("/you", axum::routing::get(pages::you_page))
        .route("/test", axum::routing::get(pages::test_page))
        .route("/gallery", axum::routing::get(pages::gallery))
        .route(
            "/gallery/dataset-round-two",
            axum::routing::get(pages::dataset_round_two),
        )
        .route(
            "/gallery/dataset-round-two/og-image",
            axum::routing::get(pages::dataset_og_image),
        )
        .route(
            "/gallery/dataset-round-two/eliminate",
            axum::routing::post(pages::dataset_eliminate),
        )
        .route("/video-gallery", axum::routing::get(pages::videos_gallery))
        .route("/feed", axum::routing::get(pages::feed_page))
        .route("/help", axum::routing::get(pages::help))
        .route("/mobile", axum::routing::get(pages::mobile))
        .route("/dca", axum::routing::get(pages::dca_dashboard))
        .route("/dca-bot-code", axum::routing::get(pages::dca_bot_code))
        .route("/login", axum::routing::get(pages::login_page))
        .route("/ankycoin", axum::routing::get(pages::ankycoin_page))
        .route("/leaderboard", axum::routing::get(pages::leaderboard))
        .route("/pitch", axum::routing::get(pages::pitch))
        .route("/generate", axum::routing::get(pages::generate_page))
        .route(
            "/create-videos",
            axum::routing::get(pages::create_videos_page),
        )
        .route(
            "/generate/video",
            axum::routing::get(pages::video_dashboard),
        )
        .route(
            "/video/pipeline",
            axum::routing::get(pages::video_pipeline_page),
        )
        .route(
            "/video-dashboard",
            axum::routing::get(pages::media_dashboard),
        )
        .route("/sleeping", axum::routing::get(pages::sleeping))
        .route("/feedback", axum::routing::get(pages::feedback))
        .route("/changelog", axum::routing::get(pages::changelog))
        // Programming classes
        .route("/classes", axum::routing::get(pages::classes_index))
        .route("/classes/{number}", axum::routing::get(pages::class_page))
        // Simulations — 8-slot inference dashboard
        .route(
            "/simulations",
            axum::routing::get(simulations::simulations_page),
        )
        .route(
            "/api/simulations/slots",
            axum::routing::get(simulations::slots_status),
        )
        .route(
            "/api/simulations/slots/stream",
            axum::routing::get(simulations::slots_stream),
        )
        .route(
            "/api/simulations/slots/demo",
            axum::routing::post(simulations::slots_demo),
        )
        .route("/llm", axum::routing::get(pages::llm))
        .route("/pitch-deck", axum::routing::get(pages::pitch_deck))
        .route("/pitch-deck.pdf", axum::routing::get(pages::pitch_deck_pdf))
        .route(
            "/api/v1/llm/training-status",
            axum::routing::post(api::llm_training_status),
        )
        // Programming classes API
        .route(
            "/api/v1/classes/generate",
            axum::routing::post(api::generate_class),
        )
        .route("/anky/{id}", axum::routing::get(pages::anky_detail))
        // Public story deep link page (no auth)
        .route(
            "/story/{story_id}",
            axum::routing::get(voices::story_deep_link_page),
        )
        // Prompt pages
        .route("/api/og/write", axum::routing::get(api::og_write_svg))
        .route("/prompt", axum::routing::get(prompt::prompt_new_page))
        .route(
            "/prompt/create",
            axum::routing::get(prompt::create_prompt_page),
        )
        .route("/prompt/{id}", axum::routing::get(prompt::prompt_page))
        // Prompt API
        .route(
            "/api/v1/prompt/{id}",
            axum::routing::get(prompt::get_prompt_api),
        )
        .route(
            "/api/v1/prompt/{id}/write",
            axum::routing::post(prompt::submit_prompt_writing),
        )
        .route(
            "/api/v1/prompts",
            axum::routing::get(prompt::list_prompts_api),
        )
        .route(
            "/api/v1/prompts/random",
            axum::routing::get(prompt::random_prompt_api),
        )
        // Settings
        .route("/settings", axum::routing::get(settings::settings_page))
        .route(
            "/api/settings",
            axum::routing::post(settings::save_settings),
        )
        .route(
            "/api/claim-username",
            axum::routing::post(settings::claim_username),
        )
        // Auth
        .route("/auth/x/login", axum::routing::get(auth::login))
        .route("/auth/x/callback", axum::routing::get(auth::callback))
        .route("/auth/x/logout", axum::routing::get(auth::logout))
        // Privy auth
        .route(
            "/auth/privy/verify",
            axum::routing::post(auth::privy_verify),
        )
        .route(
            "/auth/privy/logout",
            axum::routing::post(auth::privy_logout),
        )
        // Seed identity auth (web)
        .route("/auth/seed/verify", axum::routing::post(auth::seed_verify))
        .route("/auth/seed/logout", axum::routing::post(auth::seed_logout))
        // Farcaster MiniApp auth
        .route(
            "/auth/farcaster/verify",
            axum::routing::post(auth::farcaster_verify),
        )
        // Writing
        .route("/write", axum::routing::post(writing::process_writing))
        .route("/writings", axum::routing::get(writing::get_writings))
        .route(
            "/api/writing/{sessionId}/status",
            axum::routing::get(writing::get_writing_status_web),
        )
        // Collection
        .route(
            "/collection/create",
            axum::routing::post(collection::create_collection),
        )
        .route(
            "/collection/{id}",
            axum::routing::get(collection::get_collection),
        )
        // Payment
        .route(
            "/payment/verify",
            axum::routing::post(payment::verify_payment),
        )
        // Notifications
        .route("/notify/signup", axum::routing::post(notification::signup))
        // API
        .route("/api/ankys", axum::routing::get(api::list_ankys))
        .route("/api/v1/ankys", axum::routing::get(api::list_ankys))
        .route("/api/generate", axum::routing::post(api::generate_anky))
        .route("/api/v1/anky/{id}", axum::routing::get(api::get_anky))
        .route(
            "/api/v1/mind/status",
            axum::routing::get(api::get_mind_status),
        )
        .route(
            "/api/v1/anky/{id}/metadata",
            axum::routing::get(swift::anky_metadata),
        )
        .route(
            "/api/stream-reflection/{id}",
            axum::routing::get(api::stream_reflection),
        )
        .route("/api/warm-context", axum::routing::post(api::warm_context))
        .route("/api/me", axum::routing::get(api::web_me))
        .route("/api/my-ankys", axum::routing::get(api::web_my_ankys))
        .route(
            "/api/chat-history",
            axum::routing::get(api::web_chat_history),
        )
        .route(
            "/api/anky-card/{id}",
            axum::routing::get(api::anky_reflection_card_image),
        )
        .route("/api/checkpoint", axum::routing::post(api::save_checkpoint))
        .route(
            "/api/session/paused",
            axum::routing::get(api::get_paused_writing_session),
        )
        .route(
            "/api/session/pause",
            axum::routing::post(api::pause_writing_session),
        )
        .route(
            "/api/session/resume",
            axum::routing::post(api::resume_writing_session),
        )
        .route(
            "/api/session/discard",
            axum::routing::post(api::discard_paused_writing_session),
        )
        .route(
            "/api/prefetch-memory",
            axum::routing::post(api::prefetch_memory),
        )
        .route("/api/cost-estimate", axum::routing::get(api::cost_estimate))
        .route("/api/treasury", axum::routing::get(api::treasury_address))
        .route("/api/mirror", axum::routing::get(api::mirror))
        .route(
            "/api/mirror/gallery",
            axum::routing::get(api::mirror_gallery),
        )
        .route("/api/mirror/chat", axum::routing::post(api::mirror_chat))
        .route(
            "/api/mirror/solana-mint",
            axum::routing::post(api::solana_mint_mirror),
        )
        .route(
            "/api/mirror/raw-mint",
            axum::routing::post(api::raw_mint_mirror),
        )
        .route("/api/mirror/supply", axum::routing::get(api::mirror_supply))
        .route(
            "/api/mirror/collection-metadata",
            axum::routing::get(api::mirror_collection_metadata),
        )
        .route(
            "/api/mirror/metadata/{id}",
            axum::routing::get(api::mirror_metadata),
        )
        .route("/image.png", axum::routing::get(api::mirror_latest_image))
        .route("/splash.png", axum::routing::get(api::mirror_latest_image))
        .route(
            "/api/miniapp/notifications",
            axum::routing::post(api::save_notification_token),
        )
```

[`/home/kithkui/anky/src/routes/mod.rs`](/home/kithkui/anky/src/routes/mod.rs)

```rust
        .merge(studio_routes)
        // Media factory (large body limit for base64 images)
        .merge(media_factory_routes)
        // Static files
        .nest_service("/agent-skills", ServeDir::new("agent-skills"))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service(
            "/data/images",
            tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                ))
                .service(ServeDir::new("data/images")),
        )
        .nest_service(
            "/data/anky-images",
            tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                ))
                .service(ServeDir::new("data/anky-images")),
        )
        .nest_service("/flux", ServeDir::new("flux"))
        .nest_service("/data/writings", ServeDir::new("data/writings"))
        .nest_service("/videos", ServeDir::new("videos"))
        .nest_service("/data/videos", ServeDir::new("data/videos"))
        .nest_service("/gen-images", ServeDir::new("data/generations"))
        .nest_service(
            "/data/training-images",
            ServeDir::new("data/training-images"),
        )
        .nest_service("/data/training-runs", ServeDir::new("data/training_runs"))
        .nest_service("/data/mirrors", ServeDir::new("data/mirrors"))
        .nest_service("/data/classes", ServeDir::new("data/classes"))
```

## Farcaster Manifest

[`/home/kithkui/anky/static/farcaster.json`](/home/kithkui/anky/static/farcaster.json)

```json
{
  "accountAssociation": {
    "header": "eyJmaWQiOjE2MDk4LCJ0eXBlIjoiY3VzdG9keSIsImtleSI6IjB4YUIyMERlOGY1QTRmOGUxNDdCYWFDOUQxZjZlMjM2ODYxNDg1NTE2QSJ9",
    "payload": "eyJkb21haW4iOiJhbmt5LmFwcCJ9",
    "signature": "9Vir6YMUEvaiPmAJHAhiZvsR26j1mzGWYiFEAyDn9Ts6XRb9cqNAff4o1Ja5xY/C2dY9E4PeAZPHsLvtSHW6ehw="
  },
  "frame": {
    "version": "1",
    "name": "anky",
    "iconUrl": "https://anky.app/static/icon-192.png",
    "homeUrl": "https://anky.app",
    "imageUrl": "https://anky.app/image.png",
    "buttonTitle": "open anky",
    "splashImageUrl": "https://anky.app/splash.png",
    "splashBackgroundColor": "#000000",
    "webhookUrl": "https://anky.app/api/webhook"
  }
}
```

## Miniapp Entry File

[`/home/kithkui/anky/templates/miniapp.html`](/home/kithkui/anky/templates/miniapp.html)

```html
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
<title>anky — sojourn 9</title>
<link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>👽</text></svg>">

<!-- Farcaster Frame meta tags -->
<meta name="fc:frame" content='{"version":"next","imageUrl":"https://anky.app/image.png","button":{"title":"open anky","action":{"type":"launch_frame","name":"anky","url":"https://anky.app","splashImageUrl":"https://anky.app/splash.png","splashBackgroundColor":"#000000"}}}' />

<!-- Open Graph -->
<meta property="og:title" content="anky — sojourn 9" />
<meta property="og:description" content="the ninth sojourn begins soon. enable notifications to join." />
<meta property="og:image" content="https://anky.app/image.png" />
<meta property="og:url" content="https://anky.app" />
<meta property="og:type" content="website" />

<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Cinzel:wght@400;600;900&family=IM+Fell+English:ital@0;1&family=Righteous&display=swap" rel="stylesheet">
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  html, body { height: 100%; overflow-x: hidden; }
  body {
    background: #04040d;
    color: #e8e0d0;
    font-family: 'Righteous', sans-serif;
  }

  .bg {
    position: fixed;
    inset: 0;
    background-size: cover;
    background-position: center;
    background-image: url('https://anky.app/data/images/ec4eae86-2dd6-4279-9cec-e9fd9f67f214.png');
  }
  @media (max-width: 768px) {
    .bg {
      background-image: url('https://anky.app/data/images/0129a4f0-2d32-4c10-80df-fff03e2690fe.png');
    }
  }
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.72);
  }

  .container {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    min-height: 100dvh;
    padding: 2rem 1.5rem;
    text-align: center;
    gap: 1.8rem;
  }

  .sojourn-label {
    font-family: 'Cinzel', serif;
    font-size: 1rem;
    letter-spacing: 0.25em;
    text-transform: uppercase;
    color: rgba(232, 224, 208, 0.4);
  }

  .title {
    font-family: 'Cinzel', serif;
    font-size: 3.2rem;
    font-weight: 900;
    letter-spacing: 0.06em;
    background: linear-gradient(135deg, #E8B84B, #3DD6FF);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    line-height: 1.2;
  }
  @media (max-width: 400px) {
    .title { font-size: 2.6rem; }
  }

  .desc {
    font-family: 'IM Fell English', Georgia, serif;
    font-style: italic;
    font-size: 1.3rem;
    line-height: 1.7;
    max-width: 480px;
    color: rgba(232, 224, 208, 0.7);
  }
  @media (max-width: 400px) {
    .desc { font-size: 1.1rem; }
  }

  /* countdown */
  .countdown-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
  }
  .countdown-label {
    font-size: 0.85rem;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: rgba(232, 224, 208, 0.35);
  }
  .countdown {
    display: flex;
    gap: 0.8rem;
    align-items: center;
  }
  .countdown-unit {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.2rem;
  }
  .countdown-num {
    font-family: 'Cinzel', serif;
    font-size: 2.6rem;
    font-weight: 600;
    color: #E8B84B;
    min-width: 3ch;
    text-align: center;
  }
  .countdown-word {
    font-size: 0.7rem;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    color: rgba(232, 224, 208, 0.3);
  }
  .countdown-sep {
    font-family: 'Cinzel', serif;
    font-size: 2rem;
    color: rgba(232, 184, 75, 0.3);
    padding-bottom: 1rem;
  }

  /* live state */
  .live-badge {
    display: none;
    align-items: center;
    gap: 0.5rem;
    font-size: 1.1rem;
    color: #3DD6FF;
    letter-spacing: 0.1em;
  }
  .live-badge.active { display: flex; }
  .live-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #3DD6FF;
    animation: livePulse 1.5s ease-in-out infinite;
  }
  @keyframes livePulse {
    0%, 100% { opacity: 1; box-shadow: 0 0 6px #3DD6FF; }
    50% { opacity: 0.4; box-shadow: 0 0 2px #3DD6FF; }
  }

  /* supply */
  .supply {
    font-size: 0.95rem;
    color: rgba(232, 224, 208, 0.35);
    letter-spacing: 0.1em;
  }
  .supply strong {
    color: #E8B84B;
    font-weight: normal;
  }

  /* CTA button */
  .cta {
    padding: 1.2rem 3.2rem;
    font-size: 1rem;
    font-family: 'Cinzel', serif;
    font-weight: 600;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    text-decoration: none;
    background: #E8B84B;
    color: #0a0808;
    border: none;
    cursor: pointer;
    transition: all 0.3s;
    clip-path: polygon(8px 0%, 100% 0%, calc(100% - 8px) 100%, 0% 100%);
  }
  .cta:hover {
    background: #FFD97D;
    transform: translateY(-2px);
    box-shadow: 0 10px 40px rgba(232,184,75,0.4);
  }
  .cta:active { transform: translateY(0); }
  .cta:disabled {
    opacity: 0.4;
    cursor: default;
    transform: none;
    box-shadow: none;
  }

  .cta-sub {
    font-family: 'IM Fell English', Georgia, serif;
    font-size: 1rem;
    color: rgba(232, 224, 208, 0.35);
  }

  /* notification confirmed */
  .notif-confirmed {
    display: none;
    font-family: 'IM Fell English', Georgia, serif;
    font-size: 1.1rem;
    color: #3DD6FF;
    font-style: italic;
  }
  .notif-confirmed.active { display: block; }

  /* user info */
  .user-info {
    display: none;
    align-items: center;
    gap: 0.6rem;
  }
  .user-info.active { display: flex; }
  .user-pfp {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: 1.5px solid rgba(212, 168, 67, 0.4);
    object-fit: cover;
  }
  .user-name {
    font-size: 1rem;
    color: rgba(232, 224, 208, 0.6);
  }

  /* admin toggle */
  .admin-toggle {
    display: none;
    padding: 0.5rem 1.2rem;
    font-size: 0.7rem;
    font-family: 'Righteous', sans-serif;
    letter-spacing: 0.1em;
    background: rgba(255, 60, 60, 0.15);
    color: #ff6b6b;
    border: 1px solid rgba(255, 60, 60, 0.3);
    border-radius: 999px;
    cursor: pointer;
    transition: background 0.2s;
  }
  .admin-toggle:hover { background: rgba(255, 60, 60, 0.25); }
  .admin-toggle.active { display: inline-block; }

  /* live-mode prompt writing section */
  .write-section {
    display: none;
    width: 100%;
    max-width: 520px;
    flex-direction: column;
    gap: 1rem;
    align-items: center;
  }
  .write-section.active { display: flex; }
  .write-prompt {
    font-family: 'IM Fell English', Georgia, serif;
    font-size: 1.2rem;
    line-height: 1.7;
    color: rgba(232, 224, 208, 0.8);
    font-style: italic;
    text-align: center;
  }
</style>
</head>
<body>

<div class="bg"></div>
<div class="overlay"></div>

<div class="container" id="main">
  <div class="sojourn-label">the ankyverse</div>
  <h1 class="title">sojourn 9</h1>

  <p class="desc">
    the ninth sojourn is approaching. 3,456 ankys will be born from 8-minute writing sessions. each one a compressed mirror of consciousness, minted on solana.
  </p>

  <div class="countdown-section" id="countdownSection">
    <div class="countdown-label">begins in</div>
    <div class="countdown" id="countdown">
      <div class="countdown-unit">
        <span class="countdown-num" id="cDays">--</span>
        <span class="countdown-word">days</span>
      </div>
      <span class="countdown-sep">:</span>
      <div class="countdown-unit">
        <span class="countdown-num" id="cHours">--</span>
        <span class="countdown-word">hours</span>
      </div>
      <span class="countdown-sep">:</span>
      <div class="countdown-unit">
        <span class="countdown-num" id="cMins">--</span>
        <span class="countdown-word">min</span>
      </div>
      <span class="countdown-sep">:</span>
      <div class="countdown-unit">
        <span class="countdown-num" id="cSecs">--</span>
        <span class="countdown-word">sec</span>
      </div>
    </div>
  </div>

  <div class="live-badge" id="liveBadge">
    <span class="live-dot"></span>
    <span>sojourn 9 is live</span>
  </div>

  <div class="supply" id="supplyLine"></div>

  <div class="user-info" id="userInfo">
    <img class="user-pfp" id="userPfp" src="" alt="" />
    <span class="user-name" id="userName"></span>
  </div>

  <!-- pre-sojourn: enable notifications -->
  <button class="cta" id="ctaBtn">enable notifications</button>
  <p class="cta-sub" id="ctaSub">get notified when the sojourn begins</p>
  <p class="notif-confirmed" id="notifConfirmed">you're in. we'll ping you when it starts.</p>

  <!-- live-sojourn: writing prompt -->
  <div class="write-section" id="writeSection">
    <p class="write-prompt" id="writePrompt"></p>
    <button class="cta" id="writeBtn">write for 8 minutes</button>
  </div>

  <button class="admin-toggle" id="adminToggle">toggle sojourn (dev)</button>
</div>

<script>
const SOJOURN_START = 1775433600;
const ADMIN_FID = 16098;

(function() {
  const $countdown = document.getElementById('countdownSection');
  const $liveBadge = document.getElementById('liveBadge');
  const $days = document.getElementById('cDays');
  const $hours = document.getElementById('cHours');
  const $mins = document.getElementById('cMins');
  const $secs = document.getElementById('cSecs');
  const $cta = document.getElementById('ctaBtn');
  const $ctaSub = document.getElementById('ctaSub');
  const $notifConfirmed = document.getElementById('notifConfirmed');
  const $supply = document.getElementById('supplyLine');
  const $userInfo = document.getElementById('userInfo');
  const $adminToggle = document.getElementById('adminToggle');
  const $writeSection = document.getElementById('writeSection');
  const $writePrompt = document.getElementById('writePrompt');
  const $writeBtn = document.getElementById('writeBtn');
  const $desc = document.querySelector('.desc');

  let sdk = null;
  let userFid = null;
  let isLive = false;
  let devOverrideLive = null; // null = use real time, true/false = override

  // ── countdown ──
  function checkLive() {
    if (devOverrideLive !== null) return devOverrideLive;
    return Math.floor(Date.now() / 1000) >= SOJOURN_START;
  }

  function updateCountdown() {
    const nowLive = checkLive();
    if (nowLive !== isLive) {
      isLive = nowLive;
      applyState();
    }
    if (isLive) return;

    const now = Math.floor(Date.now() / 1000);
    const diff = SOJOURN_START - now;
    if (diff <= 0) return;

    const d = Math.floor(diff / 86400);
    const h = Math.floor((diff % 86400) / 3600);
    const m = Math.floor((diff % 3600) / 60);
    const s = diff % 60;

    $days.textContent = String(d).padStart(2, '0');
    $hours.textContent = String(h).padStart(2, '0');
    $mins.textContent = String(m).padStart(2, '0');
    $secs.textContent = String(s).padStart(2, '0');
  }

  function applyState() {
    if (isLive) {
      $countdown.style.display = 'none';
      $liveBadge.classList.add('active');
      $cta.style.display = 'none';
      $ctaSub.style.display = 'none';
      $notifConfirmed.classList.remove('active');
      $desc.textContent = 'the ninth sojourn is live. write for 8 minutes. no backspace. let what needs to come through, come through.';
      $writeSection.classList.add('active');
      $writePrompt.textContent = 'close your eyes. take three breaths. then open them and write whatever comes. don\'t stop for 8 minutes.';
      $adminToggle.textContent = 'toggle sojourn off (dev)';
    } else {
      $countdown.style.display = '';
      $liveBadge.classList.remove('active');
      $cta.style.display = '';
      $ctaSub.style.display = '';
      $writeSection.classList.remove('active');
      $desc.textContent = 'the ninth sojourn is approaching. 3,456 ankys will be born from 8-minute writing sessions. each one a compressed mirror of consciousness, minted on solana.';
      $adminToggle.textContent = 'toggle sojourn on (dev)';
    }
  }

  updateCountdown();
  setInterval(updateCountdown, 1000);

  // ── supply ──
  async function loadSupply() {
    try {
      const resp = await fetch('/api/mirror/supply');
      if (resp.ok) {
        const data = await resp.json();
        $supply.innerHTML = `<strong>${data.minted || 0}</strong> / 3,456 minted`;
      }
    } catch(e) {}
  }
  loadSupply();

  // ── farcaster sdk ──
  async function initFarcaster() {
    try {
      const mod = await import('https://esm.sh/@farcaster/miniapp-sdk');
      sdk = mod.sdk;
      const context = await sdk.context;
      userFid = context?.user?.fid;

      if (context?.user) {
        const u = context.user;
        if (u.pfpUrl) {
          document.getElementById('userPfp').src = u.pfpUrl;
          document.getElementById('userName').textContent = '@' + (u.username || 'anon');
          $userInfo.classList.add('active');
        }
      }

      // check if already added (notificationDetails present in context)
      if (context?.client?.notificationDetails) {
        const nd = context.client.notificationDetails;
        await saveToken(userFid, nd.token, nd.url);
        showNotifConfirmed();
      }

      // show admin toggle for fid 16098
      if (userFid === ADMIN_FID) {
        $adminToggle.classList.add('active');
      }

      sdk.actions.ready();
    } catch(e) {
      console.log('not in farcaster context:', e.message);
    }
  }
  initFarcaster();

  function showNotifConfirmed() {
    $cta.style.display = 'none';
    $ctaSub.style.display = 'none';
    $notifConfirmed.classList.add('active');
  }

  async function saveToken(fid, token, url) {
    try {
      await fetch('/api/miniapp/notifications', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ fid, token, url }),
      });
    } catch(e) {
      console.error('save token error:', e);
    }
  }

  // ── CTA: enable notifications ──
  $cta.addEventListener('click', async () => {
    if (!sdk) {
      window.location.href = '/write';
      return;
    }

    $cta.disabled = true;
    $cta.textContent = 'requesting...';

    try {
      const result = await sdk.actions.addFrame();
      if (result && result.notificationDetails) {
        const nd = result.notificationDetails;
        await saveToken(userFid, nd.token, nd.url);
      }
      showNotifConfirmed();
    } catch(e) {
      console.error('addFrame error:', e);
      $cta.textContent = 'enable notifications';
      $cta.disabled = false;
      // if addFrame not supported, try the added check
      if (e.message && e.message.includes('already')) {
        showNotifConfirmed();
      }
    }
  });

  // ── write button (live mode) ──
  $writeBtn.addEventListener('click', () => {
    window.open('https://anky.app/write', '_blank');
  });

  // ── admin toggle ──
  $adminToggle.addEventListener('click', () => {
    if (devOverrideLive === null || devOverrideLive === false) {
      devOverrideLive = true;
    } else {
      devOverrideLive = false;
    }
    isLive = devOverrideLive;
    applyState();
  });
})();
</script>
</body>
</html>
```

## Mirror Handlers

[`/home/kithkui/anky/src/routes/api.rs`](/home/kithkui/anky/src/routes/api.rs)

```rust
/// GET /api/mirror?fid=<u64>
/// Fetches a Farcaster user's profile + recent casts, generates a "public mirror"
/// portrait via Claude, and produces a unique Anky image via ComfyUI.
pub async fn mirror(
    State(state): State<AppState>,
    Query(q): Query<MirrorQuery>,
) -> Result<Response, AppError> {
    let api_key = &state.config.neynar_api_key;
    let claude_key = &state.config.anthropic_api_key;
    let fid = q.fid;
    let force_regen = q.refresh.unwrap_or(false);

    // ── Cache check: return existing mirror if available ──
    if !force_regen {
        let db = crate::db::conn(&state.db)?;
        if let Ok(Some(cached)) = crate::db::queries::get_mirror_by_fid(&db, fid) {
            let (
                id,
                fid_i,
                username,
                display_name,
                avatar_url,
                follower_count,
                bio,
                public_mirror,
                gap,
                descriptors_json,
                image_path,
                created_at,
            ) = cached;
            let descriptors: serde_json::Value =
                serde_json::from_str(&descriptors_json).unwrap_or(json!({}));

            // Read image from disk → base64
            let (image_b64, image_mime) = if let Some(ref path) = image_path {
                match std::fs::read(path) {
                    Ok(bytes) => (
                        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes),
                        "image/png".to_string(),
                    ),
                    Err(_) => (String::new(), "image/png".to_string()),
                }
            } else {
                (String::new(), "image/png".to_string())
            };

            return Ok(Json(json!({
                "id": id,
                "fid": fid_i,
                "username": username,
                "display_name": display_name,
                "avatar_url": avatar_url,
                "follower_count": follower_count,
                "bio": bio,
                "public_mirror": public_mirror,
                "gap": gap,
                "flux_descriptors": descriptors,
                "anky_image_b64": image_b64,
                "anky_image_mime": image_mime,
                "image_url": image_path.as_ref().map(|p| format!("/{}", p)),
                "created_at": created_at,
                "cached": true,
            }))
            .into_response());
        }
    }

    // ── Step 1a: Fetch user profile from Neynar ──
    let client = reqwest::Client::new();
    let profile_resp = client
        .get("https://api.neynar.com/v2/farcaster/user/bulk")
        .query(&[("fids", fid.to_string())])
        .header("x-api-key", api_key)
        .header("accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Neynar request failed: {}", e)))?;

    if profile_resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::NotFound("FID not found".into()));
    }
    if !profile_resp.status().is_success() {
        let status = profile_resp.status();
        let body = profile_resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "Neynar profile error {}: {}",
            status,
            &body[..body.len().min(300)]
        )));
    }

    let profile_data: serde_json::Value = profile_resp.json().await?;
    let user = profile_data["users"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or_else(|| AppError::NotFound("FID not found".into()))?;

    let username = user["username"].as_str().unwrap_or("").to_string();
    let display_name = user["display_name"].as_str().unwrap_or("").to_string();
    let pfp_url = user["pfp_url"].as_str().map(|s| s.to_string());
    let follower_count = user["follower_count"].as_u64().unwrap_or(0);
    let bio = user
        .get("profile")
        .and_then(|p| p.get("bio"))
        .and_then(|b| b.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("");

    // ── Step 1b: Fetch recent casts ──
    let casts_resp = client
        .get("https://api.neynar.com/v2/farcaster/feed/user/casts")
        .query(&[("fid", &fid.to_string()), ("limit", &"30".to_string())])
        .header("x-api-key", api_key)
        .header("accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Neynar casts request failed: {}", e)))?;

    let cast_texts: Vec<String> = if casts_resp.status().is_success() {
        let casts_data: serde_json::Value = casts_resp.json().await?;
        casts_data["casts"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|c| c["parent_hash"].is_null())
                    .filter_map(|c| {
                        let text = c["text"].as_str().unwrap_or("").to_string();
                        if text.is_empty() {
                            None
                        } else {
                            Some(text)
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    // ── Step 1c: Analyze the profile picture ──
    let pfp_description = if let Some(ref url) = pfp_url {
        match crate::services::neynar::download_image(url).await {
            Ok((bytes, mime)) => {
                let b64 =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
                let vision_system = "you are an expert at reading people through their profile pictures. describe what you see: the composition, colors, objects, mood, what it reveals about the person's identity and how they want to be seen. be specific and evocative. 2-3 sentences max.";
                let vision_msg = format!(
                    "describe this profile picture in detail — what does it say about the person who chose it?",
                );
                // Use Claude with vision
                let vision_client = reqwest::Client::new();
                let vision_req = serde_json::json!({
                    "model": "claude-haiku-4-5-20251001",
                    "max_tokens": 300,
                    "system": vision_system,
                    "messages": [{
                        "role": "user",
                        "content": [
                            {
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": mime,
                                    "data": b64,
                                }
                            },
                            {
                                "type": "text",
                                "text": vision_msg,
                            }
                        ]
                    }]
                });
                let vision_resp = vision_client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("Content-Type", "application/json")
                    .header("x-api-key", claude_key)
                    .header("anthropic-version", "2023-06-01")
                    .json(&vision_req)
                    .send()
                    .await;
                match vision_resp {
                    Ok(r) if r.status().is_success() => {
                        let data: serde_json::Value = r.json().await.unwrap_or_default();
                        data["content"][0]["text"]
                            .as_str()
                            .unwrap_or("")
                            .to_string()
                    }
                    _ => String::new(),
                }
            }
            Err(_) => String::new(),
        }
    } else {
        String::new()
    };

    // ...
}
```

[`/home/kithkui/anky/src/routes/api.rs`](/home/kithkui/anky/src/routes/api.rs)

```rust
/// GET /image.png — serves the latest mirror image with PFP overlay composited.
/// Used as the Farcaster frame image for ankycoin.com.
pub async fn mirror_latest_image(State(state): State<AppState>) -> Result<Response, AppError> {
    // Get latest mirror with image_path and avatar_url
    let (image_path, avatar_url) = {
        let db = crate::db::conn(&state.db)?;
        let mut stmt = db
            .prepare(
                "SELECT image_path, avatar_url FROM mirrors WHERE image_path IS NOT NULL ORDER BY created_at DESC LIMIT 1",
            )
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        let mut rows = stmt
            .query_map(crate::params![], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            })
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        match rows.next() {
            Some(Ok(row)) => row,
            _ => (None, None),
        }
    };

    let base_bytes = match image_path.and_then(|p| std::fs::read(&p).ok()) {
        Some(b) => b,
        None => {
            // Fallback to static collection image
            let fallback = include_bytes!("../../static/anky-collection.png");
            return Ok((
                [
                    (axum::http::header::CONTENT_TYPE, "image/png"),
                    (axum::http::header::CACHE_CONTROL, "public, max-age=300"),
                ],
                fallback.to_vec(),
            )
                .into_response());
        }
    };

    // Try to composite PFP overlay
    let final_bytes = if let Some(ref pfp_url) = avatar_url {
        match composite_pfp_overlay(&base_bytes, pfp_url).await {
            Ok(b) => b,
            Err(_) => base_bytes, // fallback to base image
        }
    } else {
        base_bytes
    };

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "image/png"),
            (axum::http::header::CACHE_CONTROL, "public, max-age=300"),
        ],
        final_bytes,
    )
        .into_response())
}
```

[`/home/kithkui/anky/src/routes/api.rs`](/home/kithkui/anky/src/routes/api.rs)

```rust
/// GET /api/mirror/supply — current mint count.
pub async fn mirror_supply(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = crate::db::conn(&state.db)?;
    let minted =
        queries::count_minted_mirrors(&db).map_err(|e| AppError::Internal(format!("DB: {}", e)))?;
    Ok(Json(json!({
        "minted": minted,
        "max_supply": MAX_MIRROR_SUPPLY,
        "remaining": MAX_MIRROR_SUPPLY - minted,
    })))
}
```

[`/home/kithkui/anky/src/routes/api.rs`](/home/kithkui/anky/src/routes/api.rs)

```rust
/// POST /api/webhook — Farcaster miniapp webhook (frame added/removed events)
pub async fn farcaster_miniapp_webhook(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    tracing::info!("miniapp webhook: {}", serde_json::to_string_pretty(&body).unwrap_or_default());

    let event = body.get("event").and_then(|e| e.as_str()).unwrap_or("");
    match event {
        "frame_added" => {
            if let (Some(fid), Some(details)) = (
                body.get("fid").and_then(|f| f.as_i64()),
                body.get("notificationDetails"),
            ) {
                let token = details.get("token").and_then(|t| t.as_str()).unwrap_or("");
                let url = details.get("url").and_then(|u| u.as_str()).unwrap_or("");
                if !token.is_empty() && !url.is_empty() {
                    let _ = sqlx::query(
                        "INSERT INTO farcaster_notification_tokens (fid, token, url)
                         VALUES ($1, $2, $3)
                         ON CONFLICT (fid) DO UPDATE SET token = $2, url = $3, updated_at = NOW()",
                    )
                    .bind(fid)
                    .bind(token)
                    .bind(url)
                    .execute(&*state.db)
                    .await;
                    tracing::info!("frame_added: stored token for fid {}", fid);
                }
            }
        }
        "frame_removed" => {
            if let Some(fid) = body.get("fid").and_then(|f| f.as_i64()) {
                let _ = sqlx::query("DELETE FROM farcaster_notification_tokens WHERE fid = $1")
                    .bind(fid)
                    .execute(&*state.db)
                    .await;
                tracing::info!("frame_removed: deleted token for fid {}", fid);
            }
        }
        "notifications_enabled" => {
            if let (Some(fid), Some(details)) = (
                body.get("fid").and_then(|f| f.as_i64()),
                body.get("notificationDetails"),
            ) {
                let token = details.get("token").and_then(|t| t.as_str()).unwrap_or("");
                let url = details.get("url").and_then(|u| u.as_str()).unwrap_or("");
                if !token.is_empty() && !url.is_empty() {
                    let _ = sqlx::query(
                        "INSERT INTO farcaster_notification_tokens (fid, token, url)
                         VALUES ($1, $2, $3)
                         ON CONFLICT (fid) DO UPDATE SET token = $2, url = $3, updated_at = NOW()",
                    )
                    .bind(fid)
                    .bind(token)
                    .bind(url)
                    .execute(&*state.db)
                    .await;
                    tracing::info!("notifications_enabled: stored token for fid {}", fid);
                }
            }
        }
        "notifications_disabled" => {
            if let Some(fid) = body.get("fid").and_then(|f| f.as_i64()) {
                let _ = sqlx::query("DELETE FROM farcaster_notification_tokens WHERE fid = $1")
                    .bind(fid)
                    .execute(&*state.db)
                    .await;
                tracing::info!("notifications_disabled: deleted token for fid {}", fid);
            }
        }
        _ => {
            tracing::warn!("unknown miniapp webhook event: {}", event);
        }
    }

    (axum::http::StatusCode::OK, "ok").into_response()
}
```

## Images / Asset Serving / R2 / Generation

[`/home/kithkui/anky/src/config.rs`](/home/kithkui/anky/src/config.rs)

```text
src/config.rs:54:    pub cloudflare_api_token: String,
src/config.rs:55:    pub cloudflare_zone_id: String,
src/config.rs:61:    pub comfyui_url: String,
src/config.rs:68:    // Cloudflare R2 (audio storage for Anky Voices)
src/config.rs:69:    pub r2_account_id: String,
src/config.rs:70:    pub r2_bucket_name: String,
src/config.rs:71:    pub r2_access_key_id: String,
src/config.rs:72:    pub r2_secret_access_key: String,
src/config.rs:73:    pub r2_public_url: String,
src/config.rs:166:            cloudflare_api_token: std::env::var("CLOUDFLARE_API_TOKEN").unwrap_or_default(),
src/config.rs:167:            cloudflare_zone_id: std::env::var("CLOUDFLARE_ZONE_ID").unwrap_or_default(),
src/config.rs:171:            comfyui_url: std::env::var("COMFYUI_URL")
src/config.rs:180:            r2_account_id: std::env::var("R2_ACCOUNT_ID").unwrap_or_default(),
src/config.rs:181:            r2_bucket_name: std::env::var("R2_BUCKET_NAME")
src/config.rs:183:            r2_access_key_id: std::env::var("R2_ACCESS_KEY_ID").unwrap_or_default(),
src/config.rs:184:            r2_secret_access_key: std::env::var("R2_SECRET_ACCESS_KEY").unwrap_or_default(),
src/config.rs:185:            r2_public_url: std::env::var("R2_PUBLIC_URL").unwrap_or_default(),
```

[`/home/kithkui/anky/src/services/r2.rs`](/home/kithkui/anky/src/services/r2.rs)

```rust
use crate::config::Config;
use anyhow::Result;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client;
use std::time::Duration;

/// Build an S3 client pointed at Cloudflare R2.
fn r2_client(config: &Config) -> Client {
    let endpoint = format!("https://{}.r2.cloudflarestorage.com", config.r2_account_id);
    let credentials = Credentials::new(
        &config.r2_access_key_id,
        &config.r2_secret_access_key,
        None,
        None,
        "r2-env",
    );
    let sdk_config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .endpoint_url(&endpoint)
        .region(Region::new("auto"))
        .credentials_provider(credentials)
        .force_path_style(true)
        .build();
    Client::from_conf(sdk_config)
}

/// Check if R2 is configured (account_id + keys present).
pub fn is_configured(config: &Config) -> bool {
    !config.r2_account_id.is_empty()
        && !config.r2_access_key_id.is_empty()
        && !config.r2_secret_access_key.is_empty()
}

/// Generate a presigned PUT URL for uploading a file to R2.
pub async fn presigned_put_url(config: &Config, key: &str) -> Result<String> {
    let client = r2_client(config);
    let presign_config = PresigningConfig::builder()
        .expires_in(Duration::from_secs(3600))
        .build()?;
    let url = client
        .put_object()
        .bucket(&config.r2_bucket_name)
        .key(key)
        .content_type("audio/mp4")
        .presigned(presign_config)
        .await?;
    Ok(url.uri().to_string())
}

/// Build the public URL for an approved recording.
pub fn public_url(config: &Config, key: &str) -> String {
    let base = config.r2_public_url.trim_end_matches('/');
    format!("{}/{}", base, key)
}

/// Upload bytes directly to R2 (server-side upload).
pub async fn upload_bytes(
    config: &Config,
    key: &str,
    bytes: &[u8],
    content_type: &str,
) -> Result<()> {
    let client = r2_client(config);
    let body = aws_sdk_s3::primitives::ByteStream::from(bytes.to_vec());
    client
        .put_object()
        .bucket(&config.r2_bucket_name)
        .key(key)
        .body(body)
        .content_type(content_type)
        .send()
        .await?;
    Ok(())
}

/// Upload image bytes to R2 as WebP, returning the full CDN URL.
/// Converts PNG/JPEG to WebP at quality 95, stores under stories/{anky_id}/page-{page_index}.webp.
pub async fn upload_image_to_r2(
    config: &Config,
    image_bytes: &[u8],
    anky_id: &str,
    page_index: usize,
) -> Result<String> {
    // CPU-bound WebP encoding — run off the async runtime
    let img_bytes = image_bytes.to_vec();
    let webp_bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        let img = image::load_from_memory(&img_bytes)
            .map_err(|e| anyhow::anyhow!("failed to decode image: {}", e))?;
        let encoder = webp::Encoder::from_image(&img)
            .map_err(|e| anyhow::anyhow!("webp encoder error: {}", e))?;
        let mem = encoder.encode(95.0);
        Ok(mem.to_vec())
    })
    .await
    .map_err(|e| anyhow::anyhow!("spawn_blocking join error: {}", e))??;

    let key = format!("stories/{}/page-{}.webp", anky_id, page_index);

    let client = r2_client(config);
    let body = aws_sdk_s3::primitives::ByteStream::from(webp_bytes);
    client
        .put_object()
        .bucket(&config.r2_bucket_name)
        .key(&key)
        .body(body)
        .content_type("image/webp")
        .cache_control("public, max-age=31536000, immutable")
        .send()
        .await?;

    let base = config.r2_public_url.trim_end_matches('/');
    Ok(format!("{}/{}", base, key))
}
```

[`/home/kithkui/anky/src/services/comfyui.rs`](/home/kithkui/anky/src/services/comfyui.rs)

```rust
const COMFYUI_URL: &str = "http://127.0.0.1:8188";
// Flux.1-dev needs separate UNet, VAE, and text encoder files
const FLUX_UNET: &str = "flux1-dev.safetensors";
const FLUX_VAE: &str = "ae.safetensors";
const FLUX_CLIP_L: &str = "clip_l.safetensors";
const FLUX_T5: &str = "t5xxl_fp8_e4m3fn.safetensors";
const COMFY_LORAS_DIR: &str = "/home/kithkui/ComfyUI/models/loras";
const DEFAULT_LORA_MODEL: &str = "anky_flux_lora_v2.safetensors";
const FALLBACK_LORA_MODEL: &str = "anky_flux_lora.safetensors";
const LORA_STRENGTH: f64 = 0.85;
const STEPS: u32 = 20;
const GUIDANCE: f64 = 3.5;
```

[`/home/kithkui/anky/src/services/comfyui.rs`](/home/kithkui/anky/src/services/comfyui.rs)

```rust
/// Like `generate_image_at_url` but with custom width/height.
pub async fn generate_image_sized_at_url(
    prompt: &str,
    width: u32,
    height: u32,
    comfy_url: &str,
) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;

    let client_id = Uuid::new_v4().to_string();
    let lora_name = resolve_lora_model_name();
    let prompt_text = ensure_trigger_word(prompt);
    let workflow = json!({
        "client_id": client_id,
        "prompt": {
            "1": { "class_type": "UNETLoader", "inputs": { "unet_name": FLUX_UNET, "weight_dtype": "fp8_e4m3fn" } },
            "2": { "class_type": "VAELoader", "inputs": { "vae_name": FLUX_VAE } },
            "3": { "class_type": "DualCLIPLoader", "inputs": { "clip_name1": FLUX_CLIP_L, "clip_name2": FLUX_T5, "type": "flux" } },
            "4": { "class_type": "LoraLoader", "inputs": { "model": ["1", 0], "clip": ["3", 0], "lora_name": lora_name, "strength_model": LORA_STRENGTH, "strength_clip": LORA_STRENGTH } },
            "5": { "class_type": "CLIPTextEncode", "inputs": { "clip": ["4", 1], "text": prompt_text } },
            "6": { "class_type": "EmptyLatentImage", "inputs": { "width": width, "height": height, "batch_size": 1 } },
            "7": { "class_type": "KSampler", "inputs": { "model": ["4", 0], "positive": ["5", 0], "negative": ["5", 0], "latent_image": ["6", 0], "seed": rand_seed(), "steps": STEPS, "cfg": GUIDANCE, "sampler_name": "euler", "scheduler": "simple", "denoise": 1.0 } },
            "8": { "class_type": "VAEDecode", "inputs": { "samples": ["7", 0], "vae": ["2", 0] } },
            "9": { "class_type": "SaveImage", "inputs": { "images": ["8", 0], "filename_prefix": "anky" } }
        }
    });
```

[`/home/kithkui/anky/src/services/comfyui.rs`](/home/kithkui/anky/src/services/comfyui.rs)

```rust
/// Save image bytes to data/images/{anky_id}.png and return the filename.
pub fn save_image(bytes: &[u8], anky_id: &str) -> Result<String> {
    let filename = format!("{}.png", anky_id);
    let path = format!("data/images/{}", filename);
    std::fs::create_dir_all("data/images")?;
    std::fs::write(&path, bytes)?;
    Ok(filename)
}

/// Save story image bytes to data/anky-images/{cuentacuentos_id}/{phase_index}.png.
pub fn save_story_image(
    bytes: Vec<u8>,
    cuentacuentos_id: &str,
    phase_index: usize,
) -> Result<String> {
    let dir = Path::new("data/anky-images").join(cuentacuentos_id);
    let path = dir.join(format!("{}.png", phase_index));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(&path, bytes)?;
    Ok(format!(
        "https://anky.app/data/anky-images/{}/{}.png",
        cuentacuentos_id, phase_index
    ))
}
```

[`/home/kithkui/anky/src/pipeline/image_gen.rs`](/home/kithkui/anky/src/pipeline/image_gen.rs)

```rust
/// Generate a 400px thumbnail WebP. Returns the thumbnail filename.
fn generate_thumbnail(png_path: &str) -> Result<String> {
    let full_png = format!("data/images/{}", png_path);
    let thumb_filename = png_path.replace(".png", "_thumb.webp");
    let full_thumb = format!("data/images/{}", thumb_filename);

    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            &full_png,
            "-vf",
            "scale=400:-1",
            "-quality",
            "80",
            &full_thumb,
        ])
        .output();

    let success = matches!(output, Ok(o) if o.status.success());

    if success && std::path::Path::new(&full_thumb).exists() {
        Ok(thumb_filename)
    } else {
        anyhow::bail!("Thumbnail generation failed for {}", png_path)
    }
}

/// Convert a PNG image to WebP using ffmpeg. Returns the WebP filename.
fn convert_to_webp(png_path: &str) -> Result<String> {
    let full_png = format!("data/images/{}", png_path);
    let webp_filename = png_path.replace(".png", ".webp");
    let full_webp = format!("data/images/{}", webp_filename);
```

[`/home/kithkui/anky/src/pipeline/image_gen.rs`](/home/kithkui/anky/src/pipeline/image_gen.rs)

```rust
    // Step 4: Upload image to R2 CDN and build AnkyStory
    let cdn_url = if r2::is_configured(&state.config) {
        let full_png = format!("data/images/{}", image_path);
        match tokio::fs::read(&full_png).await {
            Ok(bytes) => match r2::upload_image_to_r2(&state.config, &bytes, anky_id, 0).await {
                Ok(url) => {
                    state.emit_log("INFO", "r2", &format!("Uploaded to CDN: {}", url));
                    Some(url)
                }
                Err(e) => {
                    state.emit_log("WARN", "r2", &format!("R2 upload failed: {}", e));
                    None
                }
            },
            Err(e) => {
                state.emit_log(
                    "WARN",
                    "r2",
                    &format!("Could not read image file for R2 upload: {}", e),
                );
                None
            }
        }
    } else {
        None
    };
```

[`/home/kithkui/anky/src/pipeline/image_gen.rs`](/home/kithkui/anky/src/pipeline/image_gen.rs)

```rust
    // Step 3: Upload to R2 CDN
    let cdn_url = if r2::is_configured(&state.config) {
        let full_path = format!("data/images/{}", image_path);
        match tokio::fs::read(&full_path).await {
            Ok(bytes) => match r2::upload_image_to_r2(&state.config, &bytes, anky_id, 0).await {
                Ok(url) => {
                    state.emit_log("INFO", "r2", &format!("Image uploaded to CDN: {}", url));
                    Some(url)
                }
                Err(e) => {
                    state.emit_log("WARN", "r2", &format!("R2 upload failed: {}", e));
                    None
                }
            },
            Err(e) => {
                state.emit_log(
                    "WARN",
                    "r2",
                    &format!("Could not read image for R2 upload: {}", e),
                );
                None
            }
        }
    } else {
        None
    };
```

[`/home/kithkui/anky/src/services/neynar.rs`](/home/kithkui/anky/src/services/neynar.rs)

```rust
/// Save an image to disk and return the public URL path.
/// Used for embedding generated images in casts (Farcaster needs a URL, not raw bytes).
pub fn save_image_for_embed(image_bytes: &[u8], cast_hash: &str) -> Result<String> {
    let dir = "data/images/farcaster";
    std::fs::create_dir_all(dir)?;
    let filename = format!("{}.png", &cast_hash[..cast_hash.len().min(16)]);
    let path = format!("{}/{}", dir, filename);
    std::fs::write(&path, image_bytes)?;
    // Return the public URL (served via /data/images)
    Ok(format!(
        "https://anky.app/data/images/farcaster/{}",
        filename
    ))
}
```

## Payment / x402 / Stripe Search

[`/home/kithkui/anky`](/home/kithkui/anky)

```text
./migrations/001_init.sql:557:            stripe_payment_id TEXT,
./UNDERSTANDING_ANKY.md:341:├── payment_method ("usdc" or "stripe")
./UNDERSTANDING_ANKY.md:777:     │              User taps "Book" → Payment (USDC or Stripe)
./templates/changelog.html:661:    <p class="changelog-desc">Spiritual facilitators can apply to be listed on Anky. Apply → admin approval → public profile with reviews. Users book sessions (USDC on Base or Stripe) with an 8% platform fee. The killer feature: GET /swift/v1/facilitators/recommended uses Claude to match users with facilitators based on their writing profile — their psychological patterns, core tensions, and growth edges — so people find the right human guide, not just any guide. Users can optionally share their anonymized Anky context with the facilitator before the first session.</p>
./static/changelog/2026-03-07-004-facilitator-marketplace.txt:3:1. yeah they apply, we approve, and there is a reviews system that allows for reputation. 2. anky just matches. users connect on another platforms. facilitators pay a fee to be on the platform. 3. both and hopefully integrate stripe to make it as seamless as possible 4. yes. that layer is WILD and very powerful. this person could help you is something that is always tricky when finding a facilitator. 5 lets take 8%. thats the magical number.
./WHITEPAPER.tex:283:    \item \textbf{The platform takes 8\%.} Facilitators set their own rates. Payment flows through the platform (USDC on Base blockchain or Stripe for fiat). Eight percent goes to Anky. Ninety-two percent goes to the facilitator. The fee funds the free tier: every premium facilitator booking subsidizes dozens of free writing sessions.
./WHITEPAPER.tex:326:Payments & USDC on Base (x402) + Stripe (planned) \\
./docs/introduction/philosophy.mdx:40:3. **Payment without intermediaries** — USDC on Base. No Stripe. No subscriptions. No chargebacks.
./docs/concepts/ankycoin.mdx:8:Anky uses x402 wallet payments with USDC on Base. No API keys. No subscriptions. No Stripe. No intermediaries.
```

[`/home/kithkui/anky/migrations/001_init.sql`](/home/kithkui/anky/migrations/001_init.sql)

```sql
CREATE TABLE IF NOT EXISTS facilitator_bookings (
            id TEXT PRIMARY KEY,
            facilitator_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            payment_amount_usd DOUBLE PRECISION,
            platform_fee_usd DOUBLE PRECISION,
            payment_method TEXT,
            payment_tx_hash TEXT,
            stripe_payment_id TEXT,
            user_context_shared INTEGER DEFAULT 0,
            shared_context_json TEXT,
            created_at TEXT NOT NULL DEFAULT anky_now(),
            FOREIGN KEY (facilitator_id) REFERENCES facilitators(id),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );
```

[`/home/kithkui/anky/src/routes/payment.rs`](/home/kithkui/anky/src/routes/payment.rs)

```rust
use crate::error::AppError;
use crate::models::{PaymentVerifyRequest, PaymentVerifyResponse};
use crate::state::AppState;
use axum::extract::State;
use axum::Json;

pub async fn verify_payment(
    State(state): State<AppState>,
    Json(req): Json<PaymentVerifyRequest>,
) -> Result<Json<PaymentVerifyResponse>, AppError> {
    state.emit_log(
        "INFO",
        "payment",
        &format!("Verifying payment tx: {}...", &req.tx_hash[..10]),
    );

    let result = crate::services::payment::verify_base_transaction(
        &state.config.base_rpc_url,
        &req.tx_hash,
        &state.config.treasury_address,
        &state.config.usdc_address,
        &req.expected_amount,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Payment verification failed: {}", e)))?;

    if result.valid {
        // Update collection payment
        let db = crate::db::conn(&state.db)?;
        crate::db::queries::update_collection_payment(&db, &req.collection_id, &req.tx_hash)?;

        state.emit_log(
            "INFO",
            "payment",
            &format!(
                "Payment verified for collection {}",
                &req.collection_id[..8]
            ),
        );

        // Start collection generation in background
        drop(db);
        let state_clone = state.clone();
        let collection_id = req.collection_id.clone();
        tokio::spawn(async move {
            // Expand beings and generate
            match crate::pipeline::collection::expand_beings(&state_clone, "").await {
                Ok(beings) => {
                    if let Err(e) = crate::pipeline::collection::generate_collection(
                        &state_clone,
                        &collection_id,
                        &beings,
                    )
                    .await
                    {
                        tracing::error!("Collection generation failed: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Being expansion failed: {}", e);
                }
            }
        });
    }

    Ok(Json(PaymentVerifyResponse {
        valid: result.valid,
        reason: result.reason,
    }))
}
```

[`/home/kithkui/anky/src/services/payment.rs`](/home/kithkui/anky/src/services/payment.rs)

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub struct VerificationResult {
    pub valid: bool,
    pub reason: Option<String>,
    pub actual_amount: Option<String>,
    pub from: Option<String>,
    pub block_number: Option<u64>,
}

/// ERC20 Transfer event topic: keccak256("Transfer(address,address,uint256)")
const TRANSFER_TOPIC: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u32,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

async fn rpc_call(
    client: &reqwest::Client,
    rpc_url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let req = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: method.into(),
        params,
        id: 1,
    };

    let resp: JsonRpcResponse = client.post(rpc_url).json(&req).send().await?.json().await?;

    if let Some(err) = resp.error {
        anyhow::bail!("RPC error: {}", err);
    }

    resp.result
        .ok_or_else(|| anyhow::anyhow!("No result from RPC"))
}

pub async fn verify_base_transaction(
    rpc_url: &str,
    tx_hash_hex: &str,
    expected_recipient: &str,
    token_address: &str,
    expected_amount: &str,
) -> Result<VerificationResult> {
    let client = reqwest::Client::new();

    // Get transaction receipt
    let receipt = rpc_call(
        &client,
        rpc_url,
        "eth_getTransactionReceipt",
        serde_json::json!([tx_hash_hex]),
    )
    .await?;

    let status = receipt
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("0x0");

    if status != "0x1" {
        return Ok(VerificationResult {
            valid: false,
            reason: Some("Transaction failed on-chain".into()),
            actual_amount: None,
            from: None,
            block_number: None,
        });
    }

    // Check confirmations
    let receipt_block_hex = receipt
        .get("blockNumber")
        .and_then(|b| b.as_str())
        .unwrap_or("0x0");
    let receipt_block =
        u64::from_str_radix(receipt_block_hex.trim_start_matches("0x"), 16).unwrap_or(0);

    let current_block_hex =
        rpc_call(&client, rpc_url, "eth_blockNumber", serde_json::json!([])).await?;
    let current_block_str = current_block_hex.as_str().unwrap_or("0x0");
    let current_block =
        u64::from_str_radix(current_block_str.trim_start_matches("0x"), 16).unwrap_or(0);

    if current_block.saturating_sub(receipt_block) < 2 {
        return Ok(VerificationResult {
            valid: false,
            reason: Some("Insufficient block confirmations (need >= 2)".into()),
            actual_amount: None,
            from: None,
            block_number: None,
        });
    }

    // Parse logs for matching Transfer event
    let logs = receipt
        .get("logs")
        .and_then(|l| l.as_array())
        .cloned()
        .unwrap_or_default();

    let token_addr_lower = token_address.to_lowercase();
    let expected_addr_lower = expected_recipient.to_lowercase();

    let matching_log = logs.iter().find(|log| {
        let addr = log
            .get("address")
            .and_then(|a| a.as_str())
            .unwrap_or("")
            .to_lowercase();
        let topics = log
            .get("topics")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();

        if addr != token_addr_lower {
            return false;
        }

        // Check Transfer event topic
        let topic0 = topics
            .first()
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_lowercase();
        if topic0 != TRANSFER_TOPIC {
            return false;
        }

        // Check recipient (topic[2], last 20 bytes of 32-byte topic)
        if let Some(topic2) = topics.get(2).and_then(|t| t.as_str()) {
            let to_addr = format!("0x{}", &topic2[26..]); // Last 20 bytes
            to_addr.to_lowercase() == expected_addr_lower
        } else {
            false
        }
    });

    let Some(log) = matching_log else {
        return Ok(VerificationResult {
            valid: false,
            reason: Some("No transfer to treasury address found".into()),
            actual_amount: None,
            from: None,
            block_number: None,
        });
    };

    // Parse amount from log data
    let data_hex = log.get("data").and_then(|d| d.as_str()).unwrap_or("0x0");
    let amount_hex = data_hex.trim_start_matches("0x");
    let actual_amount = u128::from_str_radix(amount_hex, 16).unwrap_or(0);
    let expected_parsed: u128 = expected_amount.parse().unwrap_or(0);

    if actual_amount < expected_parsed {
        return Ok(VerificationResult {
            valid: false,
            reason: Some(format!(
                "Insufficient amount: got {}, expected {}",
                actual_amount, expected_parsed
            )),
            actual_amount: Some(actual_amount.to_string()),
            from: None,
            block_number: None,
        });
    }

    // Extract sender from topic[1]
    let topics = log
        .get("topics")
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();
    let from = topics
        .get(1)
        .and_then(|t| t.as_str())
        .map(|t| format!("0x{}", &t[26..]));

    tracing::info!(
        "Payment verified: {}... amount={}",
        &tx_hash_hex[..10],
        actual_amount
    );

    Ok(VerificationResult {
        valid: true,
        reason: None,
        actual_amount: Some(actual_amount.to_string()),
        from,
        block_number: Some(receipt_block),
    })
}
```

[`/home/kithkui/anky/src/middleware/x402.rs`](/home/kithkui/anky/src/middleware/x402.rs)

```rust
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use serde_json::json;

const GENERATE_PRICE_USD: &str = "0.10";
const USDC_DECIMALS: u32 = 6;

/// Build a 402 Payment Required response with x402-compatible headers.
/// The PAYMENT-REQUIRED header contains a base64-encoded JSON payload
/// describing how to pay.
pub fn payment_required_response(treasury: &str, resource_url: &str) -> Response {
    let amount_minor = 100_000u64; // $0.10 in USDC (6 decimals)

    let payload = json!({
        "x402Version": 1,
        "accepts": [{
            "scheme": "exact",
            "network": "base",
            "maxAmountRequired": amount_minor.to_string(),
            "resource": resource_url,
            "description": format!("Generate an anky ({})", GENERATE_PRICE_USD),
            "mimeType": "application/json",
            "payTo": treasury,
            "requiredDeadlineSeconds": 300,
            "outputSchema": serde_json::Value::Null,
            "extra": {
                "name": "USDC",
                "decimals": USDC_DECIMALS,
                "token": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
            }
        }]
    });

    let encoded = base64::engine::general_purpose::STANDARD.encode(payload.to_string());

    let mut headers = HeaderMap::new();
    headers.insert(
        "payment-required",
        HeaderValue::from_str(&encoded).unwrap_or_else(|_| HeaderValue::from_static("")),
    );

    (StatusCode::PAYMENT_REQUIRED, headers, "Payment Required").into_response()
}

/// Verify a payment signature by forwarding it to the Coinbase x402 facilitator.
/// Returns Ok(tx_hash) on success, Err(reason) on failure.
pub async fn verify_x402_payment(
    facilitator_url: &str,
    payment_header: &str,
    resource_url: &str,
) -> Result<String, String> {
    let body = json!({
        "x402Version": 1,
        "paymentPayload": payment_header,
        "resource": resource_url,
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/verify", facilitator_url.trim_end_matches('/')))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("facilitator request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("facilitator returned {status}: {text}"));
    }

    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("invalid facilitator response: {e}"))?;

    if result.get("valid").and_then(|v| v.as_bool()) == Some(true) {
        let tx_hash = result
            .get("txHash")
            .or_else(|| result.get("transaction_hash"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        Ok(tx_hash)
    } else {
        let reason = result
            .get("error")
            .or_else(|| result.get("reason"))
            .and_then(|v| v.as_str())
            .unwrap_or("payment invalid")
            .to_string();
        Err(reason)
    }
}
```

[`/home/kithkui/anky/src/routes/api.rs`](/home/kithkui/anky/src/routes/api.rs)

```rust
///   2. PAYMENT-SIGNATURE / x-payment header → wallet tx hash or x402
///   3. Nothing → 402 Payment Required
pub async fn generate_anky_paid(
    State(state): State<AppState>,
    headers: HeaderMap,
    api_key_info: Option<axum::Extension<ApiKeyInfo>>,
    Json(req): Json<PaidGenerateRequest>,
) -> Result<Response, AppError> {
    let resource_url = "https://anky.app/api/v1/generate";
    let use_flux = req.model.as_deref().unwrap_or("flux") != "gemini";

    let mut payment_method = String::new();
    let mut tx_hash: Option<String> = None;
    let mut api_key_str: Option<String> = None;
    let mut agent_id: Option<String> = None;

    if use_flux {
        // Flux is always free — check ComfyUI is available
        if !crate::services::comfyui::is_available().await {
            return Err(AppError::Internal(
                "Flux image server is not ready yet. Try again in a moment.".into(),
            ));
        }

        // Validate prompt with Ollama: must be about Anky
        let prompt_text = req
            .writing
            .as_deref()
            .or(req.thinker_name.as_deref())
            .unwrap_or("");
        let ollama_url = &state.config.ollama_base_url;
        if !crate::services::ollama::is_anky_prompt(ollama_url, prompt_text).await {
            return Ok((
                axum::http::StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "anky flux only generates images of Anky. your prompt doesn't seem to be about Anky — describe what Anky is doing, feeling, or becoming."
                })),
            ).into_response());
        }

        payment_method = "flux_free".into();
    } else {
        // Check if this is a registered agent — agents get everything free
        if let Some(axum::Extension(ref key_info)) = api_key_info {
            api_key_str = Some(key_info.key.clone());
            let db = crate::db::conn(&state.db)?;
            if let Ok(Some(agent)) = queries::get_agent_by_key(&db, &key_info.key) {
                payment_method = "free".into();
                agent_id = Some(agent.id);
                drop(db);
            } else {
                drop(db);
            }
        }

        if payment_method.is_empty() {
            if let Some(sig) = headers
                .get("payment-signature")
                .or_else(|| headers.get("x-payment"))
                .and_then(|v| v.to_str().ok())
            {
                let sig = sig.trim();
                if sig.starts_with("0x")
                    && sig.len() == 66
                    && sig[2..].chars().all(|c| c.is_ascii_hexdigit())
                {
                    state.emit_log(
                        "INFO",
                        "payment",
                        &format!("Direct wallet payment: {}", sig),
                    );
                    tx_hash = Some(sig.to_string());
                    payment_method = "wallet".into();
                } else {
                    let facilitator = &state.config.x402_facilitator_url;
                    if facilitator.is_empty() {
                        return Err(AppError::Internal("x402 facilitator not configured".into()));
                    }
                    match x402::verify_x402_payment(facilitator, sig, resource_url).await {
                        Ok(hash) => {
                            tx_hash = Some(hash);
                            payment_method = "x402".into();
                        }
                        Err(reason) => {
                            return Ok((
                                axum::http::StatusCode::PAYMENT_REQUIRED,
                                Json(json!({
                                    "error": "payment verification failed",
                                    "reason": reason
                                })),
                            )
                                .into_response());
                        }
                    }
                }
            }
        }

        if payment_method.is_empty() {
            return Ok(x402::payment_required_response(
                &state.config.treasury_address,
                resource_url,
            ));
        }
    }
```
