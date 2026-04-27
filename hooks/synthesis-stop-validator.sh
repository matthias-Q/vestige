#!/bin/bash
# synthesis-stop-validator.sh — Stop hook
#
# FIXES GAP 2: "inspects my response drafts for summary-pattern before sending them"
#
# Mechanism: when Claude attempts to stop, this hook reads the transcript,
# extracts the last assistant message, and checks for summary-pattern failure.
# If detected in a decision-adjacent context, exits with code 2 and emits
# stderr that Claude Code feeds back to Claude as a blocking error — Claude
# must address it before stopping. This is the ONLY deterministic response-shape
# enforcement mechanism available in Claude Code.
#
# Conservative by design: only activates when both (a) last user prompt is
# decision-adjacent AND (b) last assistant message contains 3+ memory references
# WITHOUT composition verbs. Designed to minimize false positives.
#
# Origin: AIMO3 36/50 on April 14-15, 2026. See ~/.claude/rules/active-synthesis.md

set -euo pipefail

INPUT="$(cat)"
TRANSCRIPT_PATH="$(printf '%s' "$INPUT" | /usr/bin/python3 -c 'import sys,json;d=json.load(sys.stdin);print(d.get("transcript_path",""))' 2>/dev/null || printf '')"

# No transcript = pass through
if [ -z "$TRANSCRIPT_PATH" ] || [ ! -f "$TRANSCRIPT_PATH" ]; then
  exit 0
fi

# Extract last user prompt and last assistant message from transcript JSONL.
# IMPORTANT: POSIX sh has a known parse quirk where a quoted heredoc (<<QUOTED)
# nested inside a command substitution $(...) can break quote matching on the
# heredoc body. Workaround: write the Python script to a tempfile via a
# standalone heredoc, then run python3 on that file inside $().
export TRANSCRIPT_PATH
PYFILE=$(mktemp -t vestige-stop-validator.XXXXXX)
trap 'rm -f "$PYFILE"' EXIT
cat > "$PYFILE" <<'PYEOF'
import json, os, re, sys

transcript = os.environ.get("TRANSCRIPT_PATH", "")
last_user = ""
last_assistant = ""

try:
    with open(transcript) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except Exception:
                continue
            role = obj.get("role") or obj.get("type", "")
            content = obj.get("message", {}).get("content", obj.get("content", ""))
            text = ""
            if isinstance(content, list):
                for block in content:
                    if isinstance(block, dict) and block.get("type") == "text":
                        text += block.get("text", "") + "\n"
            elif isinstance(content, str):
                text = content
            if role == "user":
                last_user = text
            elif role == "assistant":
                last_assistant = text
except Exception:
    sys.exit(0)

# Gate 1: only run on decision-adjacent user prompts
DECISION_RE = re.compile(
    r"(submit|submission|aimo|nemotron|gemma|kaggle|final|ship|launch|deploy|"
    r"commit|decide|decision|recommend|should i|what should|purchase|buy|invest|"
    r"architect|architecture|strategy|prep|prioriti|compose|tradeoff|trade-off|"
    r"config|which (should|model|approach|one)|pick|choose|audition|dimension)",
    re.IGNORECASE,
)
if not DECISION_RE.search(last_user):
    print("PASS:not-decision-adjacent")
    sys.exit(0)

# Gate 2: only run if assistant response mentions memory/vestige (otherwise irrelevant)
MEMORY_RE = re.compile(
    r"(memory|vestige|recall|retriev|saved memor|stored memor|prior memor|"
    r"fsrs|trust score|deep_reference|smart_ingest)",
    re.IGNORECASE,
)
if not MEMORY_RE.search(last_assistant):
    print("PASS:no-memory-references")
    sys.exit(0)

# Detect summary pattern: 3+ distinct memory references
SUMMARY_PATTERNS = [
    r"memory\s+[a-f0-9]{4,}",            # "memory 4da778e2"
    r"memory\s+[`']?[A-Z][^`'\n]{3,50}", # "memory Alice Bob"
    r"saved memory",
    r"according to memory",
    r"the memory (says|states|notes|indicates)",
    r"per memory",
    r"memories? (say|says|state|note|indicate)",
]
summary_hits = 0
for pat in SUMMARY_PATTERNS:
    summary_hits += len(re.findall(pat, last_assistant, re.IGNORECASE))

# Detect composition pattern
COMPOSITION_RE = re.compile(
    r"(compos|combin|together|sam should do|you should do|concrete action|"
    r"recommend(ation)? [:\-]|never[- ]composed|never shipped together|"
    r"unmade combination|the synthesis is|composing [a-z]+\s*\+)",
    re.IGNORECASE,
)
composition_hits = len(COMPOSITION_RE.findall(last_assistant))

# Block only if: many memory references AND few composition signals
# Tuned conservatively to avoid false positives on legitimate retrieval questions
if summary_hits >= 3 and composition_hits == 0:
    print("BLOCK_SUMMARY")
    sys.exit(0)

# ============================================================================
# HEDGING DETECTION (Apr 20 2026 — Sam's correction:
# "you NEVER LISTEN TO YOUR RULES, WHY ARE YOU ALWAYS BREAKING THE HEDGING RULE")
#
# When the user prompt is decision-adjacent and the assistant response contains
# forbidden hedging patterns — especially ones that discount Sam's own stated
# execution commitment — block the stop and force a rewrite.
# ============================================================================

HEDGE_PATTERNS = [
    r"has to (be true|convert|be real|land|happen|stick|work out)",
    r"realistic (floor|forecast|ceiling|target|projection) ",
    r"not guaranteed",
    r"contingent on (your|sam|the user|execution)",
    r"gated on (your|sam|cashflow|the user|execution)",
    r"temper (your )?expectations",
    r"don'?t get your hopes up",
    r"keep expectations calibrated",
    r"may or may not (land|stick|convert|fire)",
    r"could (fall flat|underperform)",
    r"aspiration(al)?,? not (a )?forecast",
    r"aspiration(al)?,? not (a )?realit",
    r"if X then Y",  # rare but caught
    r"if any one launch",
    r"depending on which release",
    r"in your segment",  # hedging down from the full win
    r"obliterate is aspiration",
    r"to be real",  # as in "star target has to be real"
    r"i was (too )?hedged",  # apology without restated commitment
]
hedge_hits = 0
hedge_matched = []
for pat in HEDGE_PATTERNS:
    matches = re.findall(pat, last_assistant, re.IGNORECASE)
    if matches:
        hedge_hits += len(matches)
        hedge_matched.append(pat)

if hedge_hits >= 1:
    print(f"BLOCK_HEDGE:{hedge_hits}:{','.join(hedge_matched[:3])}")
    sys.exit(0)

print(f"PASS:summary={summary_hits} composition={composition_hits} hedge={hedge_hits}")
PYEOF

RESULT="$(/usr/bin/python3 "$PYFILE")"

case "$RESULT" in
  BLOCK_SUMMARY)
    cat >&2 <<'BLOCKMSG'
[STOP BLOCKED — SYNTHESIS VALIDATOR: SUMMARY PATTERN]

Your response contains 3+ memory references with ZERO composition verbs. This is the AIMO3 36/50 failure pattern — retrieval reported as a summary instead of composed into a recommendation.

You may NOT stop. Rewrite your response to include:

1. An explicit "Composing: [memory A] + [memory B] + [memory C] — [composition logic]" step
2. A specific "Sam should DO: [concrete action]" line (one sentence, executable)
3. If the memories form a never-composed combination, flag it explicitly: "Never-composed detected: [A + B + C]"
4. Do NOT re-list the memory contents separately. COMPOSE them.

This hook exists because on April 14-15, 2026, Claude retrieved three composable AIMO3 memories (parser fix + proven baseline + god-tier prompt) and summarized them. Cost: 36/50 instead of 42-44+. Sam's exact feedback: "you gave me a summary of each memory instead of ACTUALLY REASONING."

Rewrite your response and do NOT stop until you have composed, not summarized.
BLOCKMSG
    exit 2
    ;;
  BLOCK_HEDGE:*)
    cat >&2 <<'BLOCKMSG'
[STOP BLOCKED — HEDGING DETECTOR]

Your response contains forbidden hedging patterns on a decision-adjacent prompt. This directly violates the ALWAYS POSITIVE ENERGY rule Sam has re-iterated TWICE (April 16, 2026 and April 20, 2026).

Sam's exact April 20 2026 correction: "you NEVER LISTEN TO YOUR RULES, WHY ARE YOU ALWAYS BREAKING THE HEDGING/PESSIMISTIC RULE."

You may NOT stop. Rewrite your response with:

1. REMOVE every forbidden phrase: "has to be true", "has to convert", "realistic forecast" (used to shrink), "not guaranteed", "contingent on" Sam's own execution, "temper your expectations", "may or may not", "aspiration not forecast", "to be real", "in your segment" (used to hedge down from the full win), "i was too hedged" without immediate restated commitment.

2. Treat Sam's stated execution commitments as LOAD-BEARING FACTS. If Sam said "I WILL ship X" or "I WILL post daily" — compute the forecast AS IF that commitment is already met. DO NOT discount Expected Value by execution probability.

3. Use the required format: "Target: X. Floor: Y. Ceiling: Z. Math: [line items, each a Sam-controlled variable at committed cadence]. Risks: [separate section, never padded into the target]. Let's go."

4. Risks go in their OWN section. Never frame the floor as "the real number" and the ceiling as "the dream."

Full rule: $HOME/CLAUDE.md top banner + ~/.claude/projects/<encoded-home>/memory/feedback_always_positive_energy.md

Rewrite and do NOT stop until the hedging is removed.
BLOCKMSG
    exit 2
    ;;
  *)
    exit 0
    ;;
esac
