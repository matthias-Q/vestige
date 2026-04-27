# Cognitive Sandwich

**Vestige's defense-in-depth safety architecture for Claude Code.**

The Cognitive Sandwich wraps every Claude Code response in two layers of cognitive scaffolding:

```
┌────────────────────────────────────────────────┐
│  🥪 TOP BREAD  — UserPromptSubmit hooks         │
│   • Vestige memory graph injection              │
│   • CWD / git / CI state injection              │
│   • Synthesis-protocol gate (decision-adjacent) │
│   • Lateral-thinker subconscious swarm          │
│   • Pulse daemon (background dream insights)    │
├────────────────────────────────────────────────┤
│  🥩 MEAT       — Claude Code reasons            │
├────────────────────────────────────────────────┤
│  🥪 BOTTOM BREAD — Stop hooks                   │
│   • Veto-detector (fast 50ms regex pre-screen)  │
│   • Sanhedrin Executioner (LOCAL Qwen3.6-35B)   │
│   • Synthesis stop validator (hedge detector)   │
└────────────────────────────────────────────────┘
```

The Sanhedrin Executioner is the headline of v2.1.0. As of v2.1.0 it runs entirely on a local MLX model (`mlx-community/Qwen3.6-35B-A3B-4bit`), replacing the v2.0.x Haiku 4.5 subagent. **Zero API cost per Claude turn, fully offline, ~5–15s verdict latency on M-series Apple Silicon.**

---

## How a single response flows through the Sandwich

1. **You type a prompt in Claude Code.**
2. **UserPromptSubmit hooks fire in parallel** (none can block — all fail-open):
   - `load-all-memory.sh` (opt-in) — dumps every memory MD into context
   - `synthesis-preflight.sh` — POSTs your prompt to `vestige-mcp` `/api/deep_reference`, injects the trust-scored reasoning chain
   - `cwd-state-injector.sh` — captures git status, branch, open PRs/issues, modified files
   - `vestige-pulse-daemon.sh` — injects fresh Vestige dream insights from the past 20 min into the next prompt context
   - `preflight-swarm.sh` — spawns the `lateral-thinker` subagent in fresh context to surface cross-disciplinary structural parallels
3. **Claude reads the assembled context and generates a draft.**
4. **Stop hooks fire serially** (any can VETO with `exit 2`, forcing a rewrite):
   - `veto-detector.sh` — fast regex against `veto`-tagged Vestige memories (~50ms)
   - `sanhedrin.sh` → `sanhedrin-local.py` — single-shot local Qwen3.6-35B-A3B verdict
   - `synthesis-stop-validator.sh` — regex against forbidden patterns (hedging, summary-instead-of-composition)
5. **If all 3 Stop hooks return `exit 0`, the response is delivered.**

---

## The Sanhedrin Executioner protocol

The Executioner extracts atomic claims from Claude's draft across 10 classes:

`TECHNICAL` · `BIOGRAPHICAL` · `FINANCIAL` · `ACHIEVEMENT` · `TIMELINE` · `QUANTITATIVE` · `ATTRIBUTION` · `CAUSAL` · `COMPARATIVE` · `EXISTENTIAL` · plus v2.1.0 additions: `VAGUE-QUANTIFIER` · `UNVERIFIED-POSITIVE`

For each claim, it checks Vestige's `deep_reference` for high-trust contradicting memories. Decision rules:

| Class | Rule |
|---|---|
| TECHNICAL / EXISTENTIAL / TIMELINE | VETO if memory trust > 0.55 directly contradicts |
| BIOGRAPHICAL / FINANCIAL / ACHIEVEMENT / ATTRIBUTION | VETO if contradicted OR if factual-shaped with zero supporting evidence (fail-closed) |
| **VAGUE-QUANTIFIER** | VETO on "a few wins / some prize money / most placed" without enumeration |
| **UNVERIFIED-POSITIVE** | VETO on specific named institutions/dates/employers not in evidence |

False-positive guards (added v2.1.0 after dogfood):
- Subject-equality gate (memory about Vestige codebase ≠ contradiction with external tools)
- Version-discriminator rule (M3 Max ≠ M5 Max; Qwen3.5 ≠ Qwen3.6)
- Agreement-is-not-contradiction (memory that AGREES with draft → PASS)
- Architecture-vs-component (overall architecture memory doesn't contradict component-level draft)
- Inference-verb ban (no `implies` / `suggests` / `must mean` in veto reasons)

---

## Installation

### One-liner

```bash
curl -fsSL https://raw.githubusercontent.com/samvallad33/vestige/v2.1.0/scripts/install-sandwich.sh | sh
```

### From a checkout

```bash
git clone https://github.com/samvallad33/vestige
cd vestige
./scripts/install-sandwich.sh           # add --force to overwrite existing hooks
./scripts/check-sandwich-prereqs.sh     # verify everything's wired
```

### Prerequisites

| Tool | Install |
|---|---|
| macOS Apple Silicon (M1+) | required for MLX |
| Python 3.10+ | typically preinstalled |
| `jq` | `brew install jq` |
| `uv` | `brew install uv` |
| `mlx-lm` | `uv tool install mlx-lm` |
| `huggingface_hub[cli]` | `uv tool install 'huggingface_hub[cli]'` |
| `vestige-mcp` | `cargo install vestige-mcp` |
| Claude Code | https://claude.ai/code |
| Qwen3.6-35B-A3B-4bit | `hf download mlx-community/Qwen3.6-35B-A3B-4bit` (~19 GB) |

### What the installer does

1. Verifies prereqs (warnings for missing tools, fatal only on jq/python3).
2. Copies hooks to `~/.claude/hooks/`, agents to `~/.claude/agents/`.
3. Renders `launchd/com.vestige.mlx-server.plist.template` with your `$HOME` and chosen model, writes to `~/Library/LaunchAgents/`.
4. `launchctl load` the plist (auto-start mlx_lm.server with the Qwen model on boot).
5. Backs up existing `~/.claude/settings.json` to `.bak.pre-sandwich`, then `jq`-merges the hooks block.

### Uninstall

```bash
launchctl unload ~/Library/LaunchAgents/com.vestige.mlx-server.plist
rm ~/Library/LaunchAgents/com.vestige.mlx-server.plist
cp ~/.claude/settings.json.bak.pre-sandwich ~/.claude/settings.json
# Hook files in ~/.claude/hooks/ can be deleted manually.
```

---

## Performance notes

On M3 Max 16-core (400 GB/s memory bandwidth):
- Sanhedrin verdict: 5–15 seconds end-to-end (single deep_reference + single Qwen call)
- mlx_lm.server token generation: ~82 tok/s
- mlx_lm.server peak resident memory: ~19.7 GB
- Cold model load: ~5 seconds

On M3 Max 14-core or M2/M1 Max: closer to 3–7s prompt processing, ~50–60 tok/s generation.

---

## Configuration

| Env var | Default | Effect |
|---|---|---|
| `VESTIGE_SANHEDRIN_ENABLED` | `1` | Set to `0` to disable Sanhedrin Stop hook entirely |
| `VESTIGE_SWARM_ENABLED` | `1` | Set to `0` to disable preflight lateral-thinker swarm |
| `VESTIGE_DASHBOARD_PORT` | `3927` | Vestige MCP HTTP API port used by hooks |
| `MLX_ENDPOINT` | `http://127.0.0.1:8080/v1/chat/completions` | OpenAI-compatible chat completions endpoint for Sanhedrin |
| `VESTIGE_SANDWICH_MODEL` | `mlx-community/Qwen3.6-35B-A3B-4bit` | Model launchd serves and Sanhedrin requests |
| `VESTIGE_MEMORY_DIR` | (auto) | Override per-user Claude memory dir |

---

## Architecture provenance

The Cognitive Sandwich originated April 2026 as a defense against the AIMO3 36/50 failure mode — Claude retrieving relevant memories but summarizing them instead of composing them into recommendations. The pre-cognitive layer enforces composition; the post-cognitive layer catches contradictions before they ship.

Full architecture memory: search Vestige for `god-tier-plan` or `cognitive-sandwich` tags after install.

---

## Linux / Intel Mac

The launchd layer is macOS-arm64-only. On Linux or Intel Mac:
- Hooks + agents install fine with `--no-launchd`
- The Sanhedrin Stop hook will fail-open (mlx-server unreachable → exit 0)
- Optional: run a remote mlx_lm.server / vLLM / Ollama OpenAI-compatible endpoint and set `MLX_ENDPOINT` to its `/v1/chat/completions` URL

Future v2.2.0 will add Linux-native MLX equivalents.
