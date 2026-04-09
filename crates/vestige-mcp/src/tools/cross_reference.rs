//! Deep Reference Tool (v2.0.4)
//!
//! Cognitive reasoning engine across memories. Combines:
//!   1. Broad retrieval (hybrid search + reranking)
//!   2. Spreading activation expansion (connected memories)
//!   3. FSRS-6 trust scoring (retention, stability, reps, lapses)
//!   4. Temporal supersession (newer = current truth)
//!   5. Contradiction analysis (trust-weighted)
//!   6. Dream insight integration (persisted insights)
//!   7. Structured synthesis (recommended answer + evidence)
//!
//! Research grounding: MAGMA (multi-graph), Kumiho (AGM belief revision),
//! InfMem (System-2 memory control), D-Mem (dual-process retrieval).
//!
//! Replaces cross_reference with full cognitive reasoning. cross_reference
//! is kept as a backward-compatible alias.

use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::cognitive::CognitiveEngine;
use vestige_core::Storage;

/// Input schema for deep_reference / cross_reference tool
pub fn schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "The question, claim, or topic to reason about across all memories"
            },
            "depth": {
                "type": "integer",
                "description": "How many memories to analyze (default: 20, max: 50). Higher = more thorough.",
                "default": 20,
                "minimum": 5,
                "maximum": 50
            }
        },
        "required": ["query"]
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeepRefArgs {
    query: String,
    depth: Option<i32>,
}

// ============================================================================
// FSRS-6 Trust Score
// ============================================================================

/// Compute trust score from FSRS-6 memory state.
/// Higher = more trustworthy (frequently accessed, high retention, stable, few lapses).
fn compute_trust(retention: f64, stability: f64, reps: i32, lapses: i32) -> f64 {
    let retention_factor = retention * 0.4;
    let stability_factor = (stability / 30.0).min(1.0) * 0.2;
    let reps_factor = (reps as f64 / 10.0).min(1.0) * 0.2;
    let lapses_penalty = (1.0 - (lapses as f64 / 5.0)).max(0.0) * 0.2;
    (retention_factor + stability_factor + reps_factor + lapses_penalty).clamp(0.0, 1.0)
}

// ============================================================================
// SYSTEM 1: Intent Classification (MAGMA-inspired query routing)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum QueryIntent {
    FactCheck,   // "Is X true?" → find support/contradiction evidence
    Timeline,    // "When did X happen?" → temporal ordering + pattern detection
    RootCause,   // "Why did X happen?" → causal chain backward
    Comparison,  // "How does X differ from Y?" → diff two memory clusters
    Synthesis,   // Default: "What do I know about X?" → cluster + best per cluster
}

fn classify_intent(query: &str) -> QueryIntent {
    let q = query.to_lowercase();
    let patterns: &[(QueryIntent, &[&str])] = &[
        (QueryIntent::RootCause, &["why did", "root cause", "what caused", "because of", "reason for", "why is", "why was"]),
        (QueryIntent::Timeline, &["when did", "timeline", "history of", "over time", "how has", "evolution of", "sequence of"]),
        (QueryIntent::Comparison, &["differ", "compare", "versus", " vs ", "difference between", "changed from"]),
        (QueryIntent::FactCheck, &["is it true", "did i", "was there", "verify", "confirm", "is this correct", "should i use", "should we"]),
    ];
    for (intent, keywords) in patterns {
        if keywords.iter().any(|kw| q.contains(kw)) {
            return intent.clone();
        }
    }
    QueryIntent::Synthesis
}

// ============================================================================
// SYSTEM 2: Relation Assessment (embedding similarity + sentiment + temporal)
// ============================================================================

#[derive(Debug, Clone)]
enum Relation {
    Supports,
    Contradicts,
    Supersedes,
    Irrelevant,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RelationAssessment {
    relation: Relation,
    confidence: f64,
    reasoning: String,
}

/// Assess the relationship between two memories using embedding similarity,
/// correction signals, temporal ordering, and trust comparison.
/// No LLM needed — pure algorithmic assessment.
fn assess_relation(a_content: &str, b_content: &str, a_trust: f64, b_trust: f64,
                   a_date: chrono::DateTime<Utc>, b_date: chrono::DateTime<Utc>,
                   topic_sim: f32) -> RelationAssessment {
    // Irrelevant: different topics
    if topic_sim < 0.15 {
        return RelationAssessment {
            relation: Relation::Irrelevant,
            confidence: 1.0 - topic_sim as f64,
            reasoning: format!("Different topics (similarity {:.2})", topic_sim),
        };
    }

    let time_delta_days = (b_date - a_date).num_days().abs();
    let trust_diff = b_trust - a_trust;
    let has_correction = appears_contradictory(a_content, b_content);

    // Supersession: same topic + newer + higher trust
    if topic_sim > 0.4 && time_delta_days > 0 && trust_diff > 0.05 && !has_correction {
        let (newer, older) = if b_date > a_date { ("B", "A") } else { ("A", "B") };
        return RelationAssessment {
            relation: Relation::Supersedes,
            confidence: topic_sim as f64 * (0.5 + trust_diff.min(0.5)),
            reasoning: format!("{} supersedes {} (newer by {}d, trust +{:.0}%)",
                newer, older, time_delta_days, trust_diff * 100.0),
        };
    }

    // Contradiction: same topic + correction signals detected
    if has_correction && topic_sim > 0.15 {
        return RelationAssessment {
            relation: Relation::Contradicts,
            confidence: topic_sim as f64 * 0.8,
            reasoning: format!("Contradiction detected (similarity {:.2}, correction signals present)", topic_sim),
        };
    }

    // Support: same topic + no contradiction
    if topic_sim > 0.3 {
        return RelationAssessment {
            relation: Relation::Supports,
            confidence: topic_sim as f64,
            reasoning: format!("Topically aligned (similarity {:.2}), consistent stance", topic_sim),
        };
    }

    RelationAssessment {
        relation: Relation::Irrelevant,
        confidence: 0.3,
        reasoning: "Weak relationship".to_string(),
    }
}

// ============================================================================
// SYSTEM 3: Template Reasoning Chain Generator (no LLM needed)
// ============================================================================

/// Generate a natural language reasoning chain from structured evidence.
/// The AI reads this and validates/extends it — System 1 prepares, System 2 refines.
fn generate_reasoning_chain(
    query: &str,
    intent: &QueryIntent,
    primary: &ScoredMemory,
    relations: &[(String, f64, RelationAssessment)], // (preview, trust, relation)
    confidence: f64,
) -> String {
    let mut chain = String::new();

    // Intent-specific opening
    match intent {
        QueryIntent::FactCheck => {
            chain.push_str(&format!(
                "FACT CHECK: \"{}\"\n\n", query
            ));
        }
        QueryIntent::Timeline => {
            chain.push_str(&format!(
                "TIMELINE: \"{}\"\n\n", query
            ));
        }
        QueryIntent::RootCause => {
            chain.push_str(&format!(
                "ROOT CAUSE ANALYSIS: \"{}\"\n\n", query
            ));
        }
        QueryIntent::Comparison => {
            chain.push_str(&format!(
                "COMPARISON: \"{}\"\n\n", query
            ));
        }
        QueryIntent::Synthesis => {
            chain.push_str(&format!(
                "SYNTHESIS: \"{}\"\n\n", query
            ));
        }
    }

    // Primary finding
    chain.push_str(&format!(
        "PRIMARY FINDING (trust {:.0}%, {}): {}\n",
        primary.trust * 100.0,
        primary.updated_at.format("%b %d, %Y"),
        primary.content.chars().take(150).collect::<String>(),
    ));

    // Superseded memories
    let superseded: Vec<_> = relations.iter()
        .filter(|(_, _, r)| matches!(r.relation, Relation::Supersedes))
        .collect();
    for (preview, trust, rel) in &superseded {
        chain.push_str(&format!(
            "  SUPERSEDES (trust {:.0}%): \"{}\" — {}\n",
            trust * 100.0,
            preview.chars().take(80).collect::<String>(),
            rel.reasoning,
        ));
    }

    // Supporting evidence
    let supporting: Vec<_> = relations.iter()
        .filter(|(_, _, r)| matches!(r.relation, Relation::Supports))
        .collect();
    if !supporting.is_empty() {
        chain.push_str(&format!("\nSUPPORTED BY {} MEMOR{}:\n",
            supporting.len(),
            if supporting.len() == 1 { "Y" } else { "IES" },
        ));
        for (preview, trust, _) in supporting.iter().take(5) {
            chain.push_str(&format!(
                "  + (trust {:.0}%): \"{}\"\n",
                trust * 100.0,
                preview.chars().take(80).collect::<String>(),
            ));
        }
    }

    // Contradicting evidence
    let contradicting: Vec<_> = relations.iter()
        .filter(|(_, _, r)| matches!(r.relation, Relation::Contradicts))
        .collect();
    if !contradicting.is_empty() {
        chain.push_str(&format!("\nCONTRADICTING EVIDENCE ({}):\n", contradicting.len()));
        for (preview, trust, rel) in contradicting.iter().take(3) {
            chain.push_str(&format!(
                "  ! (trust {:.0}%): \"{}\" — {}\n",
                trust * 100.0,
                preview.chars().take(80).collect::<String>(),
                rel.reasoning,
            ));
        }
    }

    // Confidence summary
    chain.push_str(&format!("\nOVERALL CONFIDENCE: {:.0}%\n", confidence * 100.0));

    chain
}

// ============================================================================
// Contradiction Detection (enhanced with relation assessment)
// ============================================================================

const NEGATION_PAIRS: &[(&str, &str)] = &[
    ("don't", "do"), ("never", "always"), ("avoid", "use"),
    ("wrong", "right"), ("incorrect", "correct"),
    ("deprecated", "recommended"), ("outdated", "current"),
    ("removed", "added"), ("disabled", "enabled"),
    ("not ", ""), ("no longer", ""),
];

const CORRECTION_SIGNALS: &[&str] = &[
    "actually", "correction", "update:", "updated:", "fixed",
    "was wrong", "changed to", "now uses", "replaced by",
    "superseded", "no longer", "instead of", "switched to", "migrated to",
];

fn appears_contradictory(a: &str, b: &str) -> bool {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    let a_words: std::collections::HashSet<&str> = a_lower.split_whitespace().filter(|w| w.len() > 3).collect();
    let b_words: std::collections::HashSet<&str> = b_lower.split_whitespace().filter(|w| w.len() > 3).collect();
    let shared_words = a_words.intersection(&b_words).count();

    if shared_words < 2 { return false; }

    for (neg, _) in NEGATION_PAIRS {
        if (a_lower.contains(neg) && !b_lower.contains(neg))
            || (b_lower.contains(neg) && !a_lower.contains(neg))
        { return true; }
    }
    for signal in CORRECTION_SIGNALS {
        if a_lower.contains(signal) || b_lower.contains(signal) { return true; }
    }
    false
}

fn topic_overlap(a: &str, b: &str) -> f32 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    let a_words: std::collections::HashSet<&str> = a_lower.split_whitespace().filter(|w| w.len() > 3).collect();
    let b_words: std::collections::HashSet<&str> = b_lower.split_whitespace().filter(|w| w.len() > 3).collect();
    if a_words.is_empty() || b_words.is_empty() { return 0.0; }
    let intersection = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();
    if union == 0 { 0.0 } else { intersection as f32 / union as f32 }
}

// ============================================================================
// Scored Memory (used across pipeline stages)
// ============================================================================

#[allow(dead_code)]
struct ScoredMemory {
    id: String,
    content: String,
    tags: Vec<String>,
    trust: f64,
    updated_at: chrono::DateTime<Utc>,
    created_at: chrono::DateTime<Utc>,
    retention: f64,
    combined_score: f32,
}

// ============================================================================
// Main Execute — 8-Stage Pipeline
// ============================================================================

pub async fn execute(
    storage: &Arc<Storage>,
    cognitive: &Arc<Mutex<CognitiveEngine>>,
    args: Option<Value>,
) -> Result<Value, String> {
    let args: DeepRefArgs = match args {
        Some(v) => serde_json::from_value(v).map_err(|e| format!("Invalid arguments: {}", e))?,
        None => return Err("Missing arguments".to_string()),
    };

    if args.query.trim().is_empty() {
        return Err("Query cannot be empty".to_string());
    }

    let depth = args.depth.unwrap_or(20).clamp(5, 50) as usize;

    // ====================================================================
    // STAGE 0: Intent Classification (MAGMA-inspired query routing)
    // ====================================================================
    let intent = classify_intent(&args.query);

    // ====================================================================
    // STAGE 1: Broad Retrieval + Reranking
    // ====================================================================
    let results = storage
        .hybrid_search(&args.query, depth as i32, 0.3, 0.7)
        .map_err(|e| e.to_string())?;

    if results.is_empty() {
        return Ok(serde_json::json!({
            "query": args.query,
            "status": "no_memories",
            "confidence": 0.0,
            "guidance": "No memories found. Use smart_ingest to add memories.",
            "memoriesAnalyzed": 0,
        }));
    }

    let mut ranked = results;
    if let Ok(mut cog) = cognitive.try_lock() {
        let candidates: Vec<_> = ranked.iter().map(|r| (r.clone(), r.node.content.clone())).collect();
        if let Ok(reranked) = cog.reranker.rerank(&args.query, candidates, Some(depth)) {
            ranked = reranked.into_iter().map(|rr| rr.item).collect();
        }
    }

    // ====================================================================
    // STAGE 2: Spreading Activation Expansion
    // ====================================================================
    let mut activation_expanded = 0usize;
    let existing_ids: std::collections::HashSet<String> = ranked.iter().map(|r| r.node.id.clone()).collect();

    if let Ok(mut cog) = cognitive.try_lock() {
        let mut expanded_ids = Vec::new();
        for r in ranked.iter().take(3) {
            let activated = cog.activation_network.activate(&r.node.id, 1.0);
            for a in activated.iter().take(3) {
                if !existing_ids.contains(&a.memory_id) && !expanded_ids.contains(&a.memory_id) {
                    expanded_ids.push(a.memory_id.clone());
                }
            }
        }
        // Fetch expanded memories from storage
        for id in &expanded_ids {
            if let Ok(Some(node)) = storage.get_node(id) {
                // Create a minimal SearchResult-like entry
                ranked.push(vestige_core::SearchResult {
                    node,
                    combined_score: 0.3, // lower score since these are expanded, not direct matches
                    keyword_score: None,
                    semantic_score: None,
                    match_type: vestige_core::MatchType::Semantic,
                });
                activation_expanded += 1;
            }
        }
    }

    // ====================================================================
    // STAGE 3: FSRS-6 Trust Scoring
    // ====================================================================

    let scored: Vec<ScoredMemory> = ranked.iter().map(|r| {
        let trust = compute_trust(
            r.node.retention_strength,
            r.node.stability,
            r.node.reps,
            r.node.lapses,
        );
        ScoredMemory {
            id: r.node.id.clone(),
            content: r.node.content.clone(),
            tags: r.node.tags.clone(),
            trust,
            updated_at: r.node.updated_at,
            created_at: r.node.created_at,
            retention: r.node.retention_strength,
            combined_score: r.combined_score,
        }
    }).collect();

    // ====================================================================
    // STAGE 4: Temporal Supersession
    // ====================================================================
    let mut superseded: Vec<Value> = Vec::new();
    let mut superseded_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Sort by date descending for supersession
    let mut by_date = scored.iter().collect::<Vec<_>>();
    by_date.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    for i in 0..by_date.len() {
        for j in (i + 1)..by_date.len() {
            let newer = by_date[i];
            let older = by_date[j];
            let overlap = topic_overlap(&newer.content, &older.content);
            if overlap > 0.3 && newer.trust > older.trust && !superseded_ids.contains(&older.id) {
                superseded_ids.insert(older.id.clone());
                superseded.push(serde_json::json!({
                    "id": older.id,
                    "preview": older.content.chars().take(150).collect::<String>(),
                    "trust": (older.trust * 100.0).round() / 100.0,
                    "date": older.updated_at.to_rfc3339(),
                    "superseded_by": newer.id,
                }));
            }
        }
    }

    // ====================================================================
    // STAGE 5: Trust-Weighted Contradiction Analysis
    // ====================================================================
    let mut contradictions: Vec<Value> = Vec::new();

    for i in 0..scored.len() {
        for j in (i + 1)..scored.len() {
            let a = &scored[i];
            let b = &scored[j];
            let overlap = topic_overlap(&a.content, &b.content);
            if overlap < 0.15 { continue; }

            let is_contradiction = appears_contradictory(&a.content, &b.content);
            if !is_contradiction { continue; }

            // Only flag as real contradiction if BOTH have decent trust
            let min_trust = a.trust.min(b.trust);
            if min_trust < 0.3 { continue; } // Low-trust memory isn't worth flagging

            let (stronger, weaker) = if a.trust >= b.trust { (a, b) } else { (b, a) };
            contradictions.push(serde_json::json!({
                "stronger": {
                    "id": stronger.id,
                    "preview": stronger.content.chars().take(150).collect::<String>(),
                    "trust": (stronger.trust * 100.0).round() / 100.0,
                    "date": stronger.updated_at.to_rfc3339(),
                },
                "weaker": {
                    "id": weaker.id,
                    "preview": weaker.content.chars().take(150).collect::<String>(),
                    "trust": (weaker.trust * 100.0).round() / 100.0,
                    "date": weaker.updated_at.to_rfc3339(),
                },
                "topic_overlap": overlap,
            }));
        }
    }

    // ====================================================================
    // STAGE 6: Dream Insight Integration
    // ====================================================================
    let mut related_insights: Vec<Value> = Vec::new();
    if let Ok(insights) = storage.get_insights(20) {
        let memory_ids: std::collections::HashSet<&str> = scored.iter().map(|s| s.id.as_str()).collect();
        for insight in insights {
            let overlaps = insight.source_memories.iter()
                .any(|src_id| memory_ids.contains(src_id.as_str()));
            if overlaps {
                related_insights.push(serde_json::json!({
                    "insight": insight.insight,
                    "type": insight.insight_type,
                    "confidence": insight.confidence,
                    "source_memories": insight.source_memories,
                }));
            }
        }
    }

    // ====================================================================
    // STAGE 7: Relation Assessment (per-pair, using trust + temporal + similarity)
    // ====================================================================
    let mut pair_relations: Vec<(String, f64, RelationAssessment)> = Vec::new();
    if let Some(primary) = scored.iter()
        .filter(|s| !superseded_ids.contains(&s.id))
        .max_by(|a, b| a.trust.partial_cmp(&b.trust).unwrap_or(std::cmp::Ordering::Equal))
    {
        for other in scored.iter().filter(|s| s.id != primary.id).take(15) {
            let sim = topic_overlap(&primary.content, &other.content);
            let rel = assess_relation(
                &primary.content, &other.content,
                primary.trust, other.trust,
                primary.updated_at, other.updated_at,
                sim,
            );
            if !matches!(rel.relation, Relation::Irrelevant) {
                pair_relations.push((
                    other.content.chars().take(100).collect(),
                    other.trust,
                    rel,
                ));
            }
        }
    }

    // ====================================================================
    // STAGE 8: Synthesis + Reasoning Chain Generation
    // ====================================================================
    // Find the recommended answer: highest trust, not superseded, most recent
    let recommended = scored.iter()
        .filter(|s| !superseded_ids.contains(&s.id))
        .max_by(|a, b| {
            // Primary: trust. Secondary: date.
            a.trust.partial_cmp(&b.trust)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.updated_at.cmp(&b.updated_at))
        });

    // Build evidence list (top memories by trust, not superseded)
    let mut non_superseded: Vec<&ScoredMemory> = scored.iter()
        .filter(|s| !superseded_ids.contains(&s.id))
        .collect();
    non_superseded.sort_by(|a, b| b.trust.partial_cmp(&a.trust).unwrap_or(std::cmp::Ordering::Equal));
    let evidence: Vec<Value> = non_superseded.iter()
        .take(10)
        .enumerate()
        .map(|(i, s)| serde_json::json!({
            "id": s.id,
            "preview": s.content.chars().take(200).collect::<String>(),
            "trust": (s.trust * 100.0).round() / 100.0,
            "date": s.updated_at.to_rfc3339(),
            "role": if i == 0 { "primary" } else { "supporting" },
        }))
        .collect();

    // Build evolution timeline
    let mut evolution: Vec<Value> = by_date.iter().rev()
        .map(|s| serde_json::json!({
            "date": s.updated_at.format("%b %d, %Y").to_string(),
            "preview": s.content.chars().take(100).collect::<String>(),
            "trust": (s.trust * 100.0).round() / 100.0,
        }))
        .collect();
    evolution.truncate(15); // cap timeline length

    // Confidence scoring
    let base_confidence = recommended.map(|r| r.trust).unwrap_or(0.0);
    let agreement_boost = (evidence.len() as f64 * 0.03).min(0.2);
    let contradiction_penalty = contradictions.len() as f64 * 0.1;
    let confidence = (base_confidence + agreement_boost - contradiction_penalty).clamp(0.0, 1.0);

    let status = if contradictions.is_empty() && confidence > 0.7 {
        "resolved"
    } else if !contradictions.is_empty() {
        "contradictions_found"
    } else if scored.is_empty() {
        "no_evidence"
    } else {
        "partial_evidence"
    };

    let guidance = if let Some(rec) = recommended {
        if contradictions.is_empty() {
            format!(
                "High confidence ({:.0}%). Recommended memory (trust {:.0}%, {}) is the most reliable source.",
                confidence * 100.0, rec.trust * 100.0, rec.updated_at.format("%b %d, %Y")
            )
        } else {
            format!(
                "WARNING: {} contradiction(s) detected. Recommended memory has trust {:.0}% but conflicts exist. Review contradictions below.",
                contradictions.len(), rec.trust * 100.0
            )
        }
    } else {
        "No strong evidence found. Verify with external sources.".to_string()
    };

    // Auto-strengthen accessed memories (Testing Effect)
    let ids: Vec<&str> = scored.iter().map(|s| s.id.as_str()).collect();
    let _ = storage.strengthen_batch_on_access(&ids);

    // Generate reasoning chain (the key differentiator — no LLM needed)
    let reasoning_chain = if let Some(rec) = recommended {
        generate_reasoning_chain(&args.query, &intent, rec, &pair_relations, confidence)
    } else {
        "No strong evidence found for reasoning.".to_string()
    };

    // Build response
    let mut response = serde_json::json!({
        "query": args.query,
        "intent": format!("{:?}", intent),
        "status": status,
        "confidence": (confidence * 100.0).round() / 100.0,
        "reasoning": reasoning_chain,
        "guidance": guidance,
        "memoriesAnalyzed": scored.len(),
        "activationExpanded": activation_expanded,
    });

    if let Some(rec) = recommended {
        response["recommended"] = serde_json::json!({
            "answer_preview": rec.content.chars().take(300).collect::<String>(),
            "memory_id": rec.id,
            "trust_score": (rec.trust * 100.0).round() / 100.0,
            "date": rec.updated_at.to_rfc3339(),
        });
    }

    if !evidence.is_empty() { response["evidence"] = serde_json::json!(evidence); }
    if !contradictions.is_empty() { response["contradictions"] = serde_json::json!(contradictions); }
    if !superseded.is_empty() { response["superseded"] = serde_json::json!(superseded); }
    if !evolution.is_empty() { response["evolution"] = serde_json::json!(evolution); }
    if !related_insights.is_empty() { response["related_insights"] = serde_json::json!(related_insights); }

    Ok(response)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_structure() {
        let s = schema();
        assert!(s["properties"]["query"].is_object());
        assert!(s["properties"]["depth"].is_object());
        assert_eq!(s["required"], serde_json::json!(["query"]));
    }

    #[test]
    fn test_trust_score_high() {
        // High retention, high stability, many reps, no lapses → high trust
        let trust = compute_trust(0.95, 60.0, 20, 0);
        assert!(trust > 0.8, "Expected >0.8, got {}", trust);
    }

    #[test]
    fn test_trust_score_low() {
        // Low retention, low stability, few reps, many lapses → low trust
        let trust = compute_trust(0.2, 1.0, 1, 10);
        assert!(trust < 0.3, "Expected <0.3, got {}", trust);
    }

    #[test]
    fn test_trust_score_medium() {
        // Medium everything
        let trust = compute_trust(0.6, 15.0, 5, 2);
        assert!(trust > 0.4 && trust < 0.7, "Expected 0.4-0.7, got {}", trust);
    }

    #[test]
    fn test_trust_score_clamped() {
        // Even extreme values stay in [0, 1]
        assert!(compute_trust(1.0, 1000.0, 100, 0) <= 1.0);
        assert!(compute_trust(0.0, 0.0, 0, 100) >= 0.0);
    }

    #[test]
    fn test_contradiction_requires_shared_words() {
        assert!(!appears_contradictory("not sure about weather", "Rust is fast"));
    }

    #[test]
    fn test_contradiction_with_shared_context() {
        assert!(appears_contradictory(
            "Don't use FAISS for vector search in production",
            "Use FAISS for vector search in production always"
        ));
    }

    #[test]
    fn test_topic_overlap_similar() {
        let overlap = topic_overlap("Vestige uses USearch for vector search", "Vestige vector search powered by USearch HNSW");
        assert!(overlap > 0.3);
    }

    #[test]
    fn test_topic_overlap_different() {
        let overlap = topic_overlap("The weather is sunny today", "Rust compile times improving");
        assert!(overlap < 0.15);
    }

    #[test]
    fn test_depth_clamped() {
        let s = schema();
        assert_eq!(s["properties"]["depth"]["minimum"], 5);
        assert_eq!(s["properties"]["depth"]["maximum"], 50);
    }

    // === Intent Classification Tests ===

    #[test]
    fn test_intent_fact_check() {
        assert_eq!(classify_intent("Is it true that Vestige uses USearch?"), QueryIntent::FactCheck);
        assert_eq!(classify_intent("Did I switch to port 3002?"), QueryIntent::FactCheck);
        assert_eq!(classify_intent("Should I use prefix caching?"), QueryIntent::FactCheck);
    }

    #[test]
    fn test_intent_timeline() {
        assert_eq!(classify_intent("When did the port change happen?"), QueryIntent::Timeline);
        assert_eq!(classify_intent("How has the AIMO3 score evolved over time?"), QueryIntent::Timeline);
    }

    #[test]
    fn test_intent_root_cause() {
        assert_eq!(classify_intent("Why did the build fail?"), QueryIntent::RootCause);
        assert_eq!(classify_intent("What caused the score regression?"), QueryIntent::RootCause);
    }

    #[test]
    fn test_intent_comparison() {
        assert_eq!(classify_intent("How does USearch differ from FAISS?"), QueryIntent::Comparison);
        assert_eq!(classify_intent("Compare FSRS versus SM-2"), QueryIntent::Comparison);
    }

    #[test]
    fn test_intent_synthesis_default() {
        assert_eq!(classify_intent("Tell me about Sam's projects"), QueryIntent::Synthesis);
        assert_eq!(classify_intent("What is Vestige?"), QueryIntent::Synthesis);
    }

    // === Relation Assessment Tests ===

    #[test]
    fn test_relation_irrelevant() {
        let rel = assess_relation("Rust is fast", "The weather is nice", 0.8, 0.8,
            Utc::now(), Utc::now(), 0.05);
        assert!(matches!(rel.relation, Relation::Irrelevant));
    }

    #[test]
    fn test_relation_supports() {
        let rel = assess_relation(
            "Vestige uses USearch for vector search",
            "USearch provides fast HNSW indexing for Vestige",
            0.8, 0.7, Utc::now(), Utc::now(), 0.6);
        assert!(matches!(rel.relation, Relation::Supports));
    }

    #[test]
    fn test_relation_contradicts() {
        let rel = assess_relation(
            "Don't use FAISS for vector search in production anymore",
            "Use FAISS for vector search in production always",
            0.8, 0.5, Utc::now(), Utc::now(), 0.7);
        assert!(matches!(rel.relation, Relation::Contradicts));
    }
}
