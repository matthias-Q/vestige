#!/bin/bash
# synthesis-gate.sh — UserPromptSubmit hook
#
# FIXES GAP 1: "forces me to run 2-5 Vestige queries before answering"
# FIXES GAP 4 (partial): injects mandate to detect never-composed combinations
#
# Mechanism: reads the user's prompt from stdin JSON, classifies decision-adjacency
# via regex, and if the prompt is decision-adjacent, returns JSON with
# hookSpecificOutput.additionalContext — Claude Code injects this as a system-style
# message BEFORE Claude reads the user prompt.
#
# Origin: AIMO3 36/50 failure on April 14-15, 2026. Claude retrieved memories but
# summarized them instead of composing. See ~/.claude/rules/active-synthesis.md

set -euo pipefail

INPUT="$(cat)"
PROMPT="$(printf '%s' "$INPUT" | /usr/bin/python3 -c 'import sys,json;d=json.load(sys.stdin);print(d.get("prompt","") or d.get("user_prompt",""))' 2>/dev/null || printf '')"

# Decision-adjacent keyword set — tuned to Sam's domains (competitions, submissions,
# shipping, commits, architectural choices, purchases, strategic decisions).
DECISION_REGEX='(submit|submission|aimo|nemotron|gemma|kaggle|final|ship|launch|deploy|commit|decide|decision|recommend|should i|what should|purchase|buy|invest|architect|architecture|strategy|prep|prioriti|compose|tradeoff|trade-off|config|which (should|model|approach|one)|pick|choose|go big|go with|audition|perform)'

if printf '%s' "$PROMPT" | /usr/bin/grep -qiE "$DECISION_REGEX"; then
  /usr/bin/python3 <<'PYEOF'
import json
msg = (
    "[SYNTHESIS GATE — DECISION-ADJACENT PROMPT DETECTED]\n\n"
    "This prompt matched decision keywords. Before you respond, you MUST execute the active synthesis protocol:\n\n"
    "1. Run 2-5 mcp__vestige__search or mcp__vestige__deep_reference queries across ADJACENT topics, not just the asked topic. Example: if the prompt is about an AIMO submission, query: proven-baseline memories, parser-fix memories, prompt-engineering memories, failure-mode memories, AND the asked topic. Minimum 4 parallel queries.\n\n"
    "2. Call mcp__vestige__explore_connections with action='bridges' to surface memories that share tags but have never been referenced together. Flag never-composed combinations EXPLICITLY in your response.\n\n"
    "3. Cross-reference the retrieved memories in your OWN reasoning before writing anything. Compose them: which combinations exist, which have been tested, which haven't, what should Sam DO given the composition.\n\n"
    "4. Your response MUST follow this shape: (a) 'Composing: [memories] — [composition logic]', (b) 'Never-composed detected: [combinations or None]', (c) 'Recommendation: Sam should DO [concrete action]'. No summary-lists of memory contents.\n\n"
    "5. Forbidden output pattern: 'Memory A says X. Memory B says Y. Memory C says Z.' followed by vague synthesis. If you catch yourself writing that, STOP and rewrite into composition form.\n\n"
    "6. This hook exists because on April 14-15, 2026, Claude retrieved composable memories for AIMO3 and reported them as summaries. Cost: 36/50 instead of 42-44+. Do not repeat this failure mode."
)
print(json.dumps({
    "hookSpecificOutput": {
        "hookEventName": "UserPromptSubmit",
        "additionalContext": msg
    }
}))
PYEOF
fi

exit 0
