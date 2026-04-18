# Changelog

All notable changes to Vestige will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.6] - 2026-04-18 — "Composer"

Polish release aimed at new-user happiness. v2.0.5's cognitive stack was already shipping; v2.0.6 makes it *feel* alive in the dashboard and stays out of your way on the prompt side.

### Added

#### Dashboard visual feedback for six live events
- `MemorySuppressed` → violet implosion + compounding pulse whose intensity scales with `suppression_count` (Anderson 2025 SIF visualised).
- `MemoryUnsuppressed` → rainbow burst + green pulse when a memory is brought back within the 24h labile window.
- `Rac1CascadeSwept` → violet wave across a random neighbour sample while the background Rac1 worker fades co-activated memories.
- `Connected` → gentle cyan ripple on WebSocket handshake.
- `ConsolidationStarted` → subtle amber pulses across a 20-node sample during the FSRS-6 decay cycle (matches feed-entry colour).
- `ImportanceScored` → magenta pulse on the scored node with intensity proportional to composite score.

Before v2.0.6 all six events fired against a silent graph. Users perceived the dashboard as broken or unresponsive during real cognitive work.

#### `VESTIGE_SYSTEM_PROMPT_MODE` environment variable
- `minimal` (default) — 3-sentence MCP `instructions` string telling the client how to use Vestige and how to react to explicit feedback. Safe for every audience, every client, every use case.
- `full` — opt in to the composition mandate (Composing / Never-composed / Recommendation response shape + FSRS-trust blocking phrase). Useful for high-stakes decision workflows; misfires on trivial retrievals, which is why it is not the default.

Advertised in `vestige-mcp --help` alongside `VESTIGE_DASHBOARD_ENABLED`.

### Fixed

#### Dashboard intentions page
- `IntentionItem.priority` was typed as `string` but the API returns the numeric FSRS-style scale (1=low, 2=normal, 3=high, 4=critical). Every intention rendered as "normal priority" regardless of its real value. Now uses a `PRIORITY_LABELS` map keyed by the numeric scale.
- `trigger_value` was typed as a plain string but the API returns `trigger_data` as a JSON-encoded payload (e.g. `{"type":"time","at":"..."}`). The UI surfaced raw JSON for every non-manual trigger. A new `summarizeTrigger()` helper parses `trigger_data` and picks the most human-readable field — `condition` / `topic` / formatted `at` / `in_minutes` / `codebase/filePattern` — before truncating for display. Closes the loop on PR #26's snake_case TriggerSpec fix at the UI layer.

### Docs

- `README.md` — new "What's New in v2.0.6" header up top; v2.0.5 block strengthened with explicit contrast against Ebbinghaus 1885 passive decay and Anderson 1994 retrieval-induced forgetting; new "Forgetting" row in the RAG-vs-Vestige comparison table.
- Intel-Mac and Windows install steps replaced with a working `cargo build --release -p vestige-mcp` snippet. The pre-built binaries for those targets are blocked on upstream toolchain gaps (`ort-sys` lacks Intel-Mac prebuilts in the 2.0.0-rc.11 release pinned by `fastembed 5.13.2`; `usearch 2.24.0` hit a Windows MSVC compile break tracked as [usearch#746](https://github.com/unum-cloud/usearch/issues/746)).

### Safety

No regressions of merged contributor PRs — v2.0.6 only touches regions that are non-overlapping with #20 (resource URI strip), #24 (codex integration docs), #26 (snake_case TriggerSpec), #28 (deep_reference query relevance), #29 (older glibc feature flags), #30 (`VESTIGE_DASHBOARD_ENABLED`), #32 (dream eviction), and #33 (keyword-first search).

---

## [2.0.5] - 2026-04-14 — "Intentional Amnesia"

Every AI memory system stores too much. Vestige now treats forgetting as a first-class, neuroscientifically-grounded primitive. This release adds **active forgetting** — top-down inhibitory control over memory retrieval, based on two 2025 papers that no other AI memory system has implemented.

### Scientific grounding

- **Anderson, M. C., Hanslmayr, S., & Quaegebeur, L. (2025).** *"Brain mechanisms underlying the inhibitory control of thought."* Nature Reviews Neuroscience. DOI: [10.1038/s41583-025-00929-y](https://www.nature.com/articles/s41583-025-00929-y). Establishes the right lateral PFC as the domain-general inhibitory controller, and Suppression-Induced Forgetting (SIF) as compounding with each stopping attempt.
- **Cervantes-Sandoval, I., Chakraborty, M., MacMullen, C., & Davis, R. L. (2020).** *"Rac1 Impairs Forgetting-Induced Cellular Plasticity in Mushroom Body Output Neurons."* Front Cell Neurosci. [PMC7477079](https://pmc.ncbi.nlm.nih.gov/articles/PMC7477079/). Establishes Rac1 GTPase as the active synaptic destabilization mechanism — forgetting is a biological PROCESS, not passive decay.

### Added

#### `suppress` MCP Tool (NEW — Tool #24)
- **Top-down memory suppression.** Distinct from `memory.delete` (which removes) and `memory.demote` (which is a one-shot hit). Each `suppress` call compounds: `suppression_count` increments, and a `k × suppression_count` penalty (saturating at 80%) is subtracted from retrieval scores during hybrid search.
- **Rac1 cascade.** Background worker piggybacks the existing consolidation loop, walks `memory_connections` edges from recently-suppressed seeds, and applies attenuated FSRS decay to co-activated neighbors. You don't just forget "Jake" — you fade the café, the roommate, the birthday.
- **Reversible 24h labile window** — matches Nader reconsolidation semantics on a 24-hour axis. Pass `reverse: true` within 24h to undo. After that, it locks in.
- **Never deletes** — the memory persists and is still accessible via `memory.get(id)`. It's INHIBITED, not erased.

#### `active_forgetting` Cognitive Module (NEW — #30)
- `crates/vestige-core/src/neuroscience/active_forgetting.rs` — stateless helper for SIF penalty computation, labile window tracking, and Rac1 cascade factors.
- 7 unit tests + 9 integration tests = 16 new tests.

#### Migration V10
- `ALTER TABLE knowledge_nodes ADD COLUMN suppression_count INTEGER DEFAULT 0`
- `ALTER TABLE knowledge_nodes ADD COLUMN suppressed_at TEXT`
- Partial indices on both columns for efficient sweep queries.
- Additive-only — backward compatible with all existing v2.0.x databases.

#### Dashboard
- `ForgettingIndicator.svelte` — new status pill that pulses when suppressed memories exist.
- 3D graph nodes dim to 20% opacity and lose emissive glow when suppressed.
- New WebSocket events: `MemorySuppressed`, `MemoryUnsuppressed`, `Rac1CascadeSwept`.
- `Heartbeat` event now carries `suppressed_count` for live dashboard display.

### Changed

- `search` scoring pipeline now includes an SIF penalty applied after the accessibility filter.
- Consolidation worker (`VESTIGE_CONSOLIDATION_INTERVAL_HOURS`, default 6h) now runs `run_rac1_cascade_sweep` after each `run_consolidation` call.
- Tool count assertion bumped from 23 → 24.
- Workspace version bumped 2.0.4 → 2.0.5.

### Tests

- Rust: 1,284 passing (up from 1,237). Net +47 new tests for active forgetting, Rac1 cascade, migration V10.
- Dashboard (Vitest): 171 passing (up from 150). +21 regression tests locking in the issue #31 UI fix.
- Zero warnings, clippy clean across all targets.

### Fixed

- **Dashboard graph view rendered glowing squares instead of round halos** ([#31](https://github.com/samvallad33/vestige/issues/31)). Root cause: the node glow `THREE.SpriteMaterial` had no `map` set, so `Sprite` rendered as a solid-coloured 1×1 plane; additive blending plus `UnrealBloomPass(strength=0.8, radius=0.4, threshold=0.85)` then amplified the square edges into hard-edged glowing cubes. The aggressive `FogExp2(..., 0.008)` swallowed edges at depth and dark-navy `0x4a4a7a` lines were invisible against the fog. Fix bundled:
  - Generated a shared 128×128 radial-gradient `CanvasTexture` (module-level singleton) and assigned it as `SpriteMaterial.map`. Gradient stops: `rgba(255,255,255,1.0) → rgba(255,255,255,0.7) → rgba(255,255,255,0.2) → rgba(255,255,255,0.0)`. Sprite now reads as a soft round halo; bloom diffuses cleanly.
  - Retuned `UnrealBloomPass` to `(strength=0.55, radius=0.6, threshold=0.2)` — gentler, allows mid-tones to bloom instead of only blown-out highlights.
  - Halved fog density `FogExp2(0x050510, 0.008) → FogExp2(0x0a0a1a, 0.0035)` so distant memories stay visible.
  - Bumped edge color `0x4a4a7a → 0x8b5cf6` (brand violet). Opacity `0.1 + weight*0.5 → 0.25 + weight*0.5`, cap `0.6 → 0.8`. Added `depthWrite: false` so edges blend cleanly through fog.
  - Added explicit `scene.background = 0x05050f` and a 2000-point starfield distributed on a spherical shell at radius 600–1000, additive-blended with subtle cool-white/violet vertex colors.
  - Glow sprite scale bumped `size × 4 → size × 6` so the gradient has visible screen footprint.
  - All node glow sprites share a single `CanvasTexture` instance (singleton cache — memory leak guard for large graphs).
  - 21 regression tests added in `apps/dashboard/src/lib/graph/__tests__/ui-fixes.test.ts`. Hybrid strategy: runtime unit tests via the existing `three-mock.ts` (extended to propagate `map`/`color`/`depthWrite`/`blending` params and added `createRadialGradient` to the canvas context mock), plus source-level regex assertions on `scene.ts` and `nodes.ts` magic numbers so any accidental revert of fog/bloom/color/helper fails the suite immediately.
- `apps/dashboard/package.json` version stale at 2.0.3 — bumped to 2.0.5 to match the workspace.
- `packages/vestige-mcp-npm/.gitignore` missing `bin/vestige-restore` and `bin/vestige-restore.exe` entries — the other three binaries were already ignored as postinstall downloads.

---

## [2.0.4] - 2026-04-09 — "Deep Reference"

Context windows hit 1M tokens. Memory matters more than ever. This release removes artificial limits, adds contradiction detection, and hardens security.

### Added

#### cross_reference Tool (NEW — Tool #22)
- **Connect the dots across memories.** Given a query or claim, searches broadly, detects agreements and contradictions between memories, identifies superseded/outdated information, and returns a confidence-scored synthesis.
- Pairwise contradiction detection using negation pairs + correction signals, gated on shared topic words to prevent false positives.
- Timeline analysis (newest-first), confidence scoring (agreements boost, contradictions penalize, recency bonus).

#### retrieval_mode Parameter (search tool)
- `precise` — top results only, no spreading activation or competition. Fast, token-efficient.
- `balanced` — full 7-stage cognitive pipeline (default, no behavior change).
- `exhaustive` — 5x overfetch, deep graph traversal, no competition suppression. Maximum recall.

#### get_batch Action (memory tool)
- `memory({ action: "get_batch", ids: ["id1", "id2", ...] })` — retrieve up to 20 full memory nodes in one call.

### Changed
- **Token budget raised: 10K → 100K** on search and session_context tools.
- **HTTP transport CORS**: `permissive()` → localhost-only origin restriction.
- **Auth token display**: Guarded against panic on short tokens.
- **Dormant state threshold**: Aligned search (0.3 → 0.4) with memory tool for consistent state classification.
- **cross_reference false positive prevention**: Requires 2+ shared words before checking negation signals.

### Stats
- 23 MCP tools, 758 tests passing, 0 failures
- Full codebase audit: 3 parallel agents, all issues resolved

---

## [2.0.0] - 2026-02-22 — "Cognitive Leap"

The biggest release in Vestige history. A complete visual and cognitive overhaul.

### Added

#### 3D Memory Dashboard
- **SvelteKit 2 + Three.js dashboard** — full 3D neural visualization at `localhost:3927/dashboard`
- **7 interactive pages**: Graph (3D force-directed), Memories (browser), Timeline, Feed (real-time events), Explore (connections), Intentions, Stats
- **WebSocket event bus** — `tokio::broadcast` channel with 16 event types (MemoryCreated, SearchPerformed, DreamStarted/Completed, ConsolidationStarted/Completed, RetentionDecayed, ConnectionDiscovered, ActivationSpread, ImportanceScored, Heartbeat, etc.)
- **Real-time 3D animations** — memories pulse on access, burst particles on creation, shockwave rings on dreams, golden flash lines on connection discovery, fade on decay
- **Bloom post-processing** — cinematic neural network aesthetic with UnrealBloomPass
- **GPU instanced rendering** — 1000+ nodes at 60fps via Three.js InstancedMesh
- **Text label sprites** — distance-based visibility (fade in <40 units, out >80 units), canvas-based rendering
- **Dream visualization mode** — purple ambient, slow-motion orbit, sequential memory replay
- **FSRS retention curves** — SVG `R(t) = e^(-t/S)` with prediction pills at 1d/7d/30d
- **Command palette** — `Cmd+K` navigation with filtered search
- **Keyboard shortcuts** — `G` Graph, `M` Memories, `T` Timeline, `F` Feed, `E` Explore, `I` Intentions, `S` Stats, `/` Search
- **Responsive layout** — desktop sidebar + mobile bottom nav with safe-area-inset
- **PWA support** — installable via `manifest.json`
- **Single binary deployment** — SvelteKit build embedded via `include_dir!` macro

#### Engine Upgrades
- **HyDE query expansion** — template-based Hypothetical Document Embeddings: classify_intent (6 types) → expand_query (3-5 variants) → centroid_embedding. Wired into `semantic_search_raw`
- **fastembed 5.11** — upgraded from 5.9, adds Nomic v2 MoE + Qwen3 reranker support
- **Nomic Embed Text v2 MoE** — opt-in via `--features nomic-v2` (475M params, 305M active, 8 experts, Candle backend)
- **Qwen3 Reranker** — opt-in via `--features qwen3-reranker` (Candle backend, high-precision cross-encoder)
- **Metal GPU acceleration** — opt-in via `--features metal` (Apple Silicon, significantly faster embedding inference)

#### Backend
- **Axum WebSocket** — `/ws` endpoint with 5-second heartbeat, live stats (memory count, avg retention, uptime)
- **7 new REST endpoints** — `POST /api/dream`, `/api/explore`, `/api/predict`, `/api/importance`, `/api/consolidate`, `GET /api/search`, `/api/retention-distribution`, `/api/intentions`
- **Event emission from MCP tools** — `emit_tool_event()` broadcasts events for smart_ingest, search, dream, consolidate, memory, importance_score
- **Shared broadcast channel** — single `tokio::broadcast::channel(1024)` shared between dashboard and MCP server
- **CORS for SvelteKit dev** — `localhost:5173` allowed in dev mode

#### Benchmarks
- **Criterion benchmark suite** — `cosine_similarity` 296ns, `centroid` 1.3µs, HyDE expand 1.4µs, RRF fusion 17µs

### Changed
- Version: 1.8.0 → 2.0.0 (both crates)
- Rust edition: 2024 (MSRV 1.85)
- Tests: 651 → 734 (352 core + 378 mcp + 4 doctests)
- Binary size: ~22MB (includes embedded SvelteKit dashboard)
- CognitiveEngine moved from main.rs binary crate to lib.rs for dashboard access
- Dashboard served at `/dashboard` prefix (legacy HTML kept at `/` and `/graph`)
- `McpServer` now accepts optional `broadcast::Sender<VestigeEvent>` for event emission

### Technical
- `apps/dashboard/` — new SvelteKit app (Svelte 5, Tailwind CSS 4, Three.js 0.172, `@sveltejs/adapter-static`)
- `dashboard/events.rs` — 16-variant `VestigeEvent` enum with `#[serde(tag = "type", content = "data")]`
- `dashboard/websocket.rs` — WebSocket upgrade handler with heartbeat + event forwarding
- `dashboard/static_files.rs` — `include_dir!` macro for embedded SvelteKit build
- `search/hyde.rs` — HyDE module with intent classification and query expansion
- `benches/search_bench.rs` — Criterion benchmarks for search pipeline components

---

## [1.8.0] - 2026-02-21

### Added
- **`session_context` tool** — one-call session initialization replacing 5 separate calls (search × 2, intention check, system_status, predict). Token-budgeted responses (~15K tokens → ~500-1000 tokens). Returns assembled markdown context, `automationTriggers` (needsDream/needsBackup/needsGc), and `expandable` memory IDs for on-demand retrieval.
- **`token_budget` parameter on `search`** — limits response size (100-10000 tokens). Results exceeding budget moved to `expandable` array with `tokensUsed`/`tokenBudget` tracking.
- **Reader/writer connection split** — `Storage` struct uses `Mutex<Connection>` for separate reader/writer SQLite handles with WAL mode. All methods take `&self` (interior mutability). `Arc<Mutex<Storage>>` → `Arc<Storage>` across ~30 files.
- **int8 vector quantization** — `ScalarKind::F16` → `I8` (2x memory savings, <1% recall loss)
- **Migration v7** — FTS5 porter tokenizer (15-30% keyword recall) + page_size 8192 (10-30% faster large-row reads)
- 22 new tests for session_context and token_budget (335 → 357 mcp tests, 651 total)

### Changed
- Tool count: 18 → 19
- `EmbeddingService::init()` changed from `&mut self` to `&self` (dead `model_loaded` field removed)
- CLAUDE.md updated: session start uses `session_context`, 19 tools documented, development section reflects storage architecture

### Performance
- Session init: ~15K tokens → ~500-1000 tokens (single tool call)
- Vector storage: 2x reduction (F16 → I8)
- Keyword search: 15-30% better recall (FTS5 porter stemming)
- Large-row reads: 10-30% faster (page_size 8192)
- Concurrent reads: non-blocking (reader/writer WAL split)

---

## [1.7.0] - 2026-02-20

### Changed
- **Tool consolidation: 23 → 18 tools** — merged redundant tools while maintaining 100% backward compatibility via deprecated redirects
- **`ingest` → `smart_ingest`** — `ingest` was a duplicate of `smart_ingest`; now redirects automatically
- **`session_checkpoint` → `smart_ingest` batch mode** — new `items` parameter on `smart_ingest` accepts up to 20 items, each running the full cognitive pipeline (importance scoring, intent detection, synaptic tagging, hippocampal indexing). Old `session_checkpoint` skipped the cognitive pipeline.
- **`promote_memory` + `demote_memory` → `memory` unified** — new `promote` and `demote` actions on the `memory` tool with optional `reason` parameter and full cognitive feedback pipeline (reward signal, reconsolidation, competition)
- **`health_check` + `stats` → `system_status`** — single tool returns combined health status, full statistics, FSRS preview, cognitive module health, state distribution, warnings, and recommendations
- **CLAUDE.md automation overhaul** — all 18 tools now have explicit auto-trigger rules; session start expanded to 5 steps (added `system_status` + `predict`); full proactive behaviors table

### Added
- `smart_ingest` batch mode with `items` parameter (max 20 items, full cognitive pipeline per item)
- `memory` actions: `promote` and `demote` with optional `reason` parameter
- `system_status` tool combining health check + statistics + cognitive health
- 30 new tests (305 → 335)

### Deprecated (still work via redirects)
- `ingest` → use `smart_ingest`
- `session_checkpoint` → use `smart_ingest` with `items`
- `promote_memory` → use `memory(action="promote")`
- `demote_memory` → use `memory(action="demote")`
- `health_check` → use `system_status`
- `stats` → use `system_status`

---

## [1.6.0] - 2026-02-19

### Changed
- **F16 vector quantization** — USearch vectors stored as F16 instead of F32 (2x storage savings)
- **Matryoshka 256-dim truncation** — embedding dimensions reduced from 768 to 256 (3x embedding storage savings)
- **Convex Combination fusion** — replaced RRF with 0.3 keyword / 0.7 semantic weighted fusion for better score preservation
- **Cross-encoder reranker** — added Jina Reranker v1 Turbo (fastembed TextRerank) for neural reranking (~20% retrieval quality improvement)
- Combined: **6x vector storage reduction** with better retrieval quality
- Cross-encoder loads in background — server starts instantly
- Old 768-dim embeddings auto-migrated on load

---

## [1.5.0] - 2026-02-18

### Added
- **CognitiveEngine** — 28-module stateful engine with full neuroscience pipeline on every tool call
- **`dream`** tool — memory consolidation via replay, discovers hidden connections and synthesizes insights
- **`explore_connections`** tool — graph traversal with chain, associations, and bridges actions
- **`predict`** tool — proactive retrieval based on context and activity patterns
- **`restore`** tool — restore memories from JSON backup files
- **Automatic consolidation** — FSRS-6 decay runs on a 6-hour timer + inline every 100 tool calls
- ACT-R base-level activation with full access history
- Episodic-to-semantic auto-merge during consolidation
- Cross-memory reinforcement on access
- Park et al. triple retrieval scoring
- Personalized w20 optimization

### Changed
- All existing tools upgraded with cognitive pre/post processing pipelines
- Tool count: 19 → 23

---

## [1.3.0] - 2026-02-12

### Added
- **`importance_score`** tool — 4-channel neuroscience scoring (novelty, arousal, reward, attention)
- **`session_checkpoint`** tool — batch smart_ingest up to 20 items with Prediction Error Gating
- **`find_duplicates`** tool — cosine similarity clustering with union-find for dedup
- `vestige ingest` CLI command for memory ingestion via command line

### Changed
- Tool count: 16 → 19
- Made `get_node_embedding` public in core API
- Added `get_all_embeddings` for duplicate scanning

---

## [1.2.0] - 2026-02-12

### Added
- **Web dashboard** — Axum-based on port 3927 with memory browser, search, and system stats
- **`memory_timeline`** tool — browse memories chronologically, grouped by day
- **`memory_changelog`** tool — audit trail of memory state transitions
- **`health_check`** tool — system health status with recommendations
- **`consolidate`** tool — run FSRS-6 maintenance cycle
- **`stats`** tool — full memory system statistics
- **`backup`** tool — create SQLite database backups
- **`export`** tool — export memories as JSON/JSONL with filters
- **`gc`** tool — garbage collect low-retention memories
- `backup_to()` and `get_recent_state_transitions()` storage APIs

### Changed
- Search now supports `detail_level` (brief/summary/full) to control token usage
- Tool count: 8 → 16

---

## [1.1.3] - 2026-02-12

### Changed
- Upgraded to Rust edition 2024
- Security hardening and dependency updates

### Fixed
- Dedup on ingest edge cases
- Intel Mac CI builds
- NPM package version alignment
- Removed dead TypeScript package

---

## [1.1.2] - 2025-01-27

### Fixed
- Embedding model cache now uses platform-appropriate directories instead of polluting project folders
  - macOS: `~/Library/Caches/com.vestige.core/fastembed`
  - Linux: `~/.cache/vestige/fastembed`
  - Windows: `%LOCALAPPDATA%\vestige\cache\fastembed`
- Can still override with `FASTEMBED_CACHE_PATH` environment variable

---

## [1.1.1] - 2025-01-27

### Fixed
- UTF-8 string slicing issues in keyword search and prospective memory
- Silent error handling in MCP stdio protocol
- Feature flag forwarding between crates
- All GitHub issues resolved (#1, #3, #4)

### Added
- Pre-built binaries for Linux, Windows, and macOS (Intel & ARM)
- GitHub Actions CI/CD for automated releases

---

## [1.1.0] - 2025-01-26

### Changed
- **Tool Consolidation**: 29 tools → 8 cognitive primitives
  - `recall`, `semantic_search`, `hybrid_search` → `search`
  - `get_knowledge`, `delete_knowledge`, `get_memory_state` → `memory`
  - `remember_pattern`, `remember_decision`, `get_codebase_context` → `codebase`
  - 5 intention tools → `intention`
- Stats and maintenance moved from MCP to CLI (`vestige stats`, `vestige health`, etc.)

### Added
- CLI admin commands: `vestige stats`, `vestige health`, `vestige consolidate`, `vestige restore`
- Feedback tools: `promote_memory`, `demote_memory`
- 30+ FAQ entries with verified neuroscience claims
- Storage modes documentation: Global, per-project, multi-Claude household
- CLAUDE.md templates for proactive memory use
- Version pinning via git tags

### Deprecated
- Old tool names (still work with warnings, removed in v2.0)

---

## [1.0.0] - 2025-01-25

### Added
- FSRS-6 spaced repetition algorithm with 21 parameters
- Bjork & Bjork dual-strength memory model (storage + retrieval strength)
- Local semantic embeddings with fastembed v5 (BGE-base-en-v1.5, 768 dimensions)
- HNSW vector search with USearch (20x faster than FAISS)
- Hybrid search combining BM25 keyword + semantic + RRF fusion
- Two-stage retrieval with reranking (+15-20% precision)
- MCP server for Claude Desktop integration
- Tauri desktop application
- Codebase memory module for AI code understanding
- Neuroscience-inspired memory mechanisms:
  - Synaptic Tagging and Capture (retroactive importance)
  - Context-Dependent Memory (Tulving encoding specificity)
  - Spreading Activation Networks
  - Memory States (Active/Dormant/Silent/Unavailable)
  - Multi-channel Importance Signals (Novelty/Arousal/Reward/Attention)
  - Hippocampal Indexing (Teyler & Rudy 2007)
- Prospective memory (intentions and reminders)
- Sleep consolidation with 5-stage processing
- Memory compression for long-term storage
- Cross-project learning for universal patterns

### Changed
- Upgraded embedding model from all-MiniLM-L6-v2 (384d) to BGE-base-en-v1.5 (768d)
- Upgraded fastembed from v4 to v5

### Fixed
- SQL injection protection in FTS5 queries
- Infinite loop prevention in file watcher
- SIGSEGV crash in vector index (reserve before add)
- Memory safety with Mutex wrapper for embedding model

---

## [0.1.0] - 2025-01-24

### Added
- Initial release
- Core memory storage with SQLite + FTS5
- Basic FSRS scheduling
- MCP protocol support
- Desktop app skeleton
