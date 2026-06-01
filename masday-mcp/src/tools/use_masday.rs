//! use_masday MCP tool — universal entry point for routing

use serde_json::Value;

/// Classify user intent and return routing suggestion
///
/// Uses keyword matching to determine:
/// - Intent category (workflow, memory, search, code, deploy, test, review, research, general)
/// - Suggested skill (e.g., masday-workflow-new, masday-tdd)
/// - Suggested agent (e.g., masday-orchestrator, masday-backend, masday-frontend)
/// - Complexity (low, medium, high)
pub async fn use_masday(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let prompt = args
        .get("prompt")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'prompt' argument")?;

    let prompt_lower = prompt.to_lowercase();

    // Define keyword patterns for each intent
    let workflow_keywords = [
        "workflow", "task", "plan", "execute", "orchestrat", "coord", "parallel",
        "agent", "skill", "step", "phase", "gate", "pipeline"
    ];
    let memory_keywords = [
        "remember", "recall", "search memory", "store", "forget", "episodic",
        "context", "knowledge", "graph", "persist"
    ];
    let search_keywords = [
        "search", "find", "lookup", "codebase", "semantic", "context pack",
        "fingerprint", "retrieve", "query"
    ];
    let code_keywords = [
        "implement", "write code", "refactor", "function", "api", "endpoint",
        "database", "migration", "add feature", "fix bug", "backend", "frontend"
    ];
    let deploy_keywords = [
        "deploy", "docker", "kubernetes", "ci/cd", "pipeline", "release",
        "production", "build", "container"
    ];
    let test_keywords = [
        "test", "spec", "coverage", "tdd", "verify", "assert", "unit test",
        "integration test", "e2e"
    ];
    let review_keywords = [
        "review", "audit", "check", "validate", "policy", "quality",
        "compliance", "inspect"
    ];
    let research_keywords = [
        "research", "investigate", "explore", "analyze", "study", "document",
        "learn", "understand"
    ];

    // Match intent based on keyword density
    let mut scores: [(&str, i32); 9] = [
        ("workflow", 0),
        ("memory", 0),
        ("search", 0),
        ("code", 0),
        ("deploy", 0),
        ("test", 0),
        ("review", 0),
        ("research", 0),
        ("general", 0),
    ];

    for (idx, keywords) in [
        (&workflow_keywords[..], 0),
        (&memory_keywords[..], 1),
        (&search_keywords[..], 2),
        (&code_keywords[..], 3),
        (&deploy_keywords[..], 4),
        (&test_keywords[..], 5),
        (&review_keywords[..], 6),
        (&research_keywords[..], 7),
    ].iter().enumerate() {
        for keyword in keywords.0.iter() {
            if prompt_lower.contains(*keyword) {
                scores[idx].1 += 1;
            }
        }
    }

    // Find highest scoring intent (default to general)
    let intent = scores
        .iter()
        .max_by_key(|(_, score)| *score)
        .map(|(intent, _)| *intent)
        .unwrap_or("general");

    // Determine complexity based on prompt characteristics
    let complexity = if prompt_lower.len() < 50 {
        "low"
    } else if prompt_lower.len() < 200 {
        "medium"
    } else {
        "high"
    };

    // Check for multi-task indicators
    let has_multiple = prompt_lower.contains("and") || prompt_lower.contains("then") ||
        prompt_lower.contains("also") || prompt_lower.contains("plus");
    let has_sequence = prompt_lower.contains("then") || prompt_lower.contains("after") ||
        prompt_lower.contains("next");

    // Map intent to suggested skill and agent
    let (suggested_skill, suggested_agent) = match intent {
        "workflow" => {
            if has_multiple || has_sequence {
                ("masday-workflow-new", "masday-orchestrator")
            } else {
                ("masday-workflow-run", "masday-orchestrator")
            }
        }
        "memory" => ("masday-memory-search", "masday-orchestrator"),
        "search" => ("masday-code-analyze", "masday-orchestrator"),
        "code" => {
            if prompt_lower.contains("test") || prompt_lower.contains("tdd") {
                ("masday-tdd", "masday-tdd-guide")
            } else if prompt_lower.contains("backend") || prompt_lower.contains("api") {
                ("masday-backend-dev", "masday-backend")
            } else if prompt_lower.contains("frontend") || prompt_lower.contains("ui") {
                ("masday-frontend-dev", "masday-frontend")
            } else {
                ("masday-full-stack", "masday-orchestrator")
            }
        }
        "deploy" => ("masday-deploy-check", "masday-devops"),
        "test" => ("masday-tdd", "masday-qa"),
        "review" => ("verification-before-completion", "masday-reviewer"),
        "research" => ("masday-research", "masday-researcher"),
        _ => ("masday-active", "general-purpose"),
    };

    Ok(serde_json::json!({
        "prompt": prompt,
        "intent": intent,
        "suggestedSkill": suggested_skill,
        "suggestedAgent": suggested_agent,
        "complexity": complexity,
        "indicators": {
            "hasMultipleTasks": has_multiple,
            "hasSequence": has_sequence
        },
        "confidence": calculate_confidence(&scores, scores.iter().position(|(i, _)| *i == intent))
    }))
}

/// Calculate confidence score based on keyword match distribution
fn calculate_confidence(scores: &[(&str, i32); 9], winner_idx: Option<usize>) -> f64 {
    let winner_idx = match winner_idx {
        Some(idx) => idx,
        None => return 0.5,
    };

    let winner_score = scores[winner_idx].1;
    if winner_score == 0 {
        return 0.3; // Low confidence for general intent with no matches
    }

    // Sum of all scores
    let total: i32 = scores.iter().map(|(_, s)| s).sum();
    if total == 0 {
        return 0.3;
    }

    // Confidence = winner_score / total, with adjustments
    let base_confidence = winner_score as f64 / total as f64;

    // Boost confidence if winner has significantly more matches than runner-up
    let mut sorted_scores: Vec<i32> = scores.iter().map(|(_, s)| *s).collect();
    sorted_scores.sort_by(|a, b| b.cmp(a)); // Descending
    if sorted_scores.len() >= 2 && sorted_scores[0] > sorted_scores[1] * 2 {
        (base_confidence * 1.2).min(0.95)
    } else {
        base_confidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_intent() {
        let result = use_masday(serde_json::json!({
            "prompt": "Create a new workflow orchestration plan"
        }))
        .await
        .unwrap();

        assert_eq!(result["intent"], "workflow");
        assert!(result["suggestedSkill"].as_str().unwrap().contains("workflow"));
    }

    #[tokio::test]
    async fn test_code_intent() {
        let result = use_masday(serde_json::json!({
            "prompt": "Implement a new REST API endpoint for user authentication"
        }))
        .await
        .unwrap();

        assert_eq!(result["intent"], "code");
        assert!(result["suggestedAgent"].as_str().unwrap().contains("backend") ||
                result["suggestedAgent"].as_str().unwrap().contains("orchestrator"));
    }

    #[tokio::test]
    async fn test_tdd_routing() {
        let result = use_masday(serde_json::json!({
            "prompt": "Write tests first for the payment module using TDD"
        }))
        .await
        .unwrap();

        // Test intent is detected (either "test" or "code" depending on keyword weights)
        assert!(result["intent"] == "test" || result["intent"] == "code");
        // The suggested skill should include TDD for test/code intents
        assert!(result["suggestedSkill"].as_str().unwrap().contains("tdd"));
    }

    #[tokio::test]
    async fn test_complexity() {
        let short = use_masday(serde_json::json!({"prompt": "Run tests"}))
            .await
            .unwrap();
        assert_eq!(short["complexity"], "low");

        let medium = use_masday(serde_json::json!({
            "prompt": "Create a new workflow with three tasks: implement the feature, write tests, and deploy to staging"
        }))
        .await
        .unwrap();
        assert_eq!(medium["complexity"], "medium");
    }

    #[tokio::test]
    async fn test_confidence_calculation() {
        let result = use_masday(serde_json::json!({
            "prompt": "workflow workflow workflow workflow"
        }))
        .await
        .unwrap();

        assert!(result["confidence"].as_f64().unwrap() > 0.5);
    }
}
