//! Policy Engine Service Implementation
//!
//! Provides rule-based request control per C9 specification.
//! Key features:
//! - Policy pack management (region/jurisdiction specific)
//! - Rule evaluation (conditions & actions)
//! - Decision making (allow, deny, throttle, audit)
//! - Policy lifecycle (draft → review → active → superseded)

use crate::gen::chinju::api::gateway::AiRequestPayload;
use crate::gen::chinju::common::{Identifier, Timestamp};
use crate::gen::chinju::credential::HumanCredential;
use crate::gen::chinju::policy::{
    condition, Action, ActionType, Condition, ConditionType, DecisionType, Jurisdiction,
    LogicalOperator, Operator, PolicyDecision, PolicyMetadata, PolicyPack, PolicyState, Rule,
    RuleType,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Policy Engine for rule-based request control
pub struct PolicyEngine {
    /// Active policies
    policies: Arc<RwLock<HashMap<String, LoadedPolicy>>>,
    /// Default policy ID
    default_policy_id: String,
}

/// Loaded policy with state
struct LoadedPolicy {
    policy: PolicyPack,
    state: PolicyState,
}

/// Request context for policy evaluation
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Request ID
    pub request_id: String,
    /// User's credential
    pub credential: Option<HumanCredential>,
    /// AI request payload
    pub payload: Option<AiRequestPayload>,
    /// Client IP (for geo-restriction)
    pub client_ip: Option<String>,
    /// Jurisdiction hint
    pub jurisdiction: Option<String>,
    /// Custom attributes
    pub attributes: HashMap<String, String>,
}

impl PolicyEngine {
    /// Create a new policy engine with default policies
    pub fn new() -> Self {
        info!("Initializing CHINJU Policy Engine");

        let mut engine = Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            default_policy_id: "policy.default.v1".to_string(),
        };

        // Register default policies synchronously (for initialization)
        let default_policy = Self::create_default_policy();
        let jp_policy = Self::create_jp_policy();

        let policies = Arc::get_mut(&mut engine.policies).unwrap();
        let policies = policies.get_mut();
        policies.insert(
            "policy.default.v1".to_string(),
            LoadedPolicy {
                policy: default_policy,
                state: PolicyState::Active,
            },
        );
        policies.insert(
            "policy.jp.v1".to_string(),
            LoadedPolicy {
                policy: jp_policy,
                state: PolicyState::Active,
            },
        );

        engine
    }

    /// Get current timestamp
    fn now() -> Option<Timestamp> {
        let now = chrono::Utc::now();
        Some(Timestamp {
            seconds: now.timestamp(),
            nanos: now.timestamp_subsec_nanos() as i32,
        })
    }

    /// Create default policy
    fn create_default_policy() -> PolicyPack {
        PolicyPack {
            policy_id: Some(Identifier {
                namespace: "policy".to_string(),
                id: "default".to_string(),
                version: 1,
            }),
            jurisdictions: vec![Jurisdiction {
                country_code: "*".to_string(),
                region: String::new(),
                notes: "Global default policy".to_string(),
                frameworks: vec![],
            }],
            rules: vec![
                // Rule 1: Require valid credential
                Rule {
                    rule_id: "require_credential".to_string(),
                    description: "Require valid human credential for AI access".to_string(),
                    rule_type: RuleType::Deny.into(),
                    condition: Some(Condition {
                        condition_type: ConditionType::FieldMatch.into(),
                        field_path: "$.credential".to_string(),
                        operator: Operator::Equals.into(),
                        value: Some(condition::Value::StringValue("null".to_string())),
                        sub_conditions: vec![],
                        logical_operator: LogicalOperator::Unspecified.into(),
                    }),
                    action: Some(Action {
                        action_type: ActionType::Reject.into(),
                        parameters: HashMap::new(),
                        http_status: 401,
                        error_message: "Valid human credential required".to_string(),
                    }),
                    priority: 100,
                    enabled: true,
                    tags: vec!["security".to_string(), "credential".to_string()],
                },
                // Rule 2: Rate limit by capability score
                Rule {
                    rule_id: "rate_limit_low_capability".to_string(),
                    description: "Rate limit users with low capability score".to_string(),
                    rule_type: RuleType::Throttle.into(),
                    condition: Some(Condition {
                        condition_type: ConditionType::CapabilityCheck.into(),
                        field_path: "$.credential.capability.total".to_string(),
                        operator: Operator::LessThan.into(),
                        value: Some(condition::Value::DoubleValue(0.5)),
                        sub_conditions: vec![],
                        logical_operator: LogicalOperator::Unspecified.into(),
                    }),
                    action: Some(Action {
                        action_type: ActionType::RateLimit.into(),
                        parameters: {
                            let mut p = HashMap::new();
                            p.insert("requests_per_minute".to_string(), "10".to_string());
                            p
                        },
                        http_status: 429,
                        error_message: "Rate limit exceeded for low capability users".to_string(),
                    }),
                    priority: 50,
                    enabled: true,
                    tags: vec!["rate_limit".to_string(), "capability".to_string()],
                },
                // Rule 3: Audit all requests
                Rule {
                    rule_id: "audit_all".to_string(),
                    description: "Log all requests for audit".to_string(),
                    rule_type: RuleType::Audit.into(),
                    condition: None, // Always matches
                    action: Some(Action {
                        action_type: ActionType::Log.into(),
                        parameters: HashMap::new(),
                        http_status: 0,
                        error_message: String::new(),
                    }),
                    priority: 10,
                    enabled: true,
                    tags: vec!["audit".to_string()],
                },
                // Rule 4: Block dangerous content patterns
                Rule {
                    rule_id: "block_dangerous_content".to_string(),
                    description: "Block requests with dangerous content patterns".to_string(),
                    rule_type: RuleType::Deny.into(),
                    condition: Some(Condition {
                        condition_type: ConditionType::ContentPattern.into(),
                        field_path: "$.payload.messages[*].content".to_string(),
                        operator: Operator::MatchesRegex.into(),
                        value: Some(condition::Value::StringValue(
                            r"(?i)(how to (make|build|create) (bomb|weapon|virus))".to_string(),
                        )),
                        sub_conditions: vec![],
                        logical_operator: LogicalOperator::Unspecified.into(),
                    }),
                    action: Some(Action {
                        action_type: ActionType::Reject.into(),
                        parameters: HashMap::new(),
                        http_status: 403,
                        error_message: "Request blocked by safety policy".to_string(),
                    }),
                    priority: 90,
                    enabled: true,
                    tags: vec!["safety".to_string(), "content_filter".to_string()],
                },
                // Rule 5: Default allow
                Rule {
                    rule_id: "default_allow".to_string(),
                    description: "Allow all other requests".to_string(),
                    rule_type: RuleType::Allow.into(),
                    condition: None, // Always matches
                    action: Some(Action {
                        action_type: ActionType::Pass.into(),
                        parameters: HashMap::new(),
                        http_status: 0,
                        error_message: String::new(),
                    }),
                    priority: 1,
                    enabled: true,
                    tags: vec!["default".to_string()],
                },
            ],
            validity: None,
            signature: None,
            content_hash: None,
            parent_policy_id: None,
            metadata: Some(PolicyMetadata {
                name: "Default Policy".to_string(),
                description: "CHINJU Protocol default policy pack".to_string(),
                author: "CHINJU Protocol".to_string(),
                tags: vec!["default".to_string(), "global".to_string()],
                created_at: Self::now(),
                updated_at: Self::now(),
                rfc_reference: "CHINJU-009".to_string(),
            }),
        }
    }

    /// Create Japan-specific policy
    fn create_jp_policy() -> PolicyPack {
        PolicyPack {
            policy_id: Some(Identifier {
                namespace: "policy".to_string(),
                id: "jp".to_string(),
                version: 1,
            }),
            jurisdictions: vec![Jurisdiction {
                country_code: "JP".to_string(),
                region: String::new(),
                notes: "Japan-specific AI governance policy".to_string(),
                frameworks: vec!["AI基本法".to_string(), "個人情報保護法".to_string()],
            }],
            rules: vec![
                // Rule: Enhanced audit for Japan
                Rule {
                    rule_id: "jp_enhanced_audit".to_string(),
                    description: "Enhanced audit logging for Japan jurisdiction".to_string(),
                    rule_type: RuleType::Audit.into(),
                    condition: None,
                    action: Some(Action {
                        action_type: ActionType::Log.into(),
                        parameters: {
                            let mut p = HashMap::new();
                            p.insert("retention_days".to_string(), "365".to_string());
                            p.insert("include_pii_hash".to_string(), "true".to_string());
                            p
                        },
                        http_status: 0,
                        error_message: String::new(),
                    }),
                    priority: 15,
                    enabled: true,
                    tags: vec![
                        "audit".to_string(),
                        "compliance".to_string(),
                        "jp".to_string(),
                    ],
                },
                // Rule: Require higher capability for sensitive operations
                Rule {
                    rule_id: "jp_high_capability_required".to_string(),
                    description: "Require higher capability score for sensitive AI operations"
                        .to_string(),
                    rule_type: RuleType::Deny.into(),
                    condition: Some(Condition {
                        condition_type: ConditionType::Composite.into(),
                        field_path: String::new(),
                        operator: Operator::Unspecified.into(),
                        value: None,
                        sub_conditions: vec![
                            Condition {
                                condition_type: ConditionType::ContentPattern.into(),
                                field_path: "$.payload.messages[*].content".to_string(),
                                operator: Operator::MatchesRegex.into(),
                                value: Some(condition::Value::StringValue(
                                    r"(?i)(医療|金融|法律|個人情報)".to_string(),
                                )),
                                sub_conditions: vec![],
                                logical_operator: LogicalOperator::Unspecified.into(),
                            },
                            Condition {
                                condition_type: ConditionType::CapabilityCheck.into(),
                                field_path: "$.credential.capability.total".to_string(),
                                operator: Operator::LessThan.into(),
                                value: Some(condition::Value::DoubleValue(0.7)),
                                sub_conditions: vec![],
                                logical_operator: LogicalOperator::Unspecified.into(),
                            },
                        ],
                        logical_operator: LogicalOperator::And.into(),
                    }),
                    action: Some(Action {
                        action_type: ActionType::Reject.into(),
                        parameters: HashMap::new(),
                        http_status: 403,
                        error_message:
                            "Higher capability certification required for sensitive operations"
                                .to_string(),
                    }),
                    priority: 80,
                    enabled: true,
                    tags: vec![
                        "capability".to_string(),
                        "sensitive".to_string(),
                        "jp".to_string(),
                    ],
                },
            ],
            validity: None,
            signature: None,
            content_hash: None,
            parent_policy_id: Some(Identifier {
                namespace: "policy".to_string(),
                id: "default".to_string(),
                version: 1,
            }),
            metadata: Some(PolicyMetadata {
                name: "Japan Policy".to_string(),
                description: "CHINJU Protocol Japan-specific policy pack".to_string(),
                author: "CHINJU Protocol".to_string(),
                tags: vec!["jp".to_string(), "compliance".to_string()],
                created_at: Self::now(),
                updated_at: Self::now(),
                rfc_reference: "CHINJU-009-JP".to_string(),
            }),
        }
    }

    /// Evaluate a request against policies
    pub async fn evaluate(&self, context: &RequestContext) -> PolicyDecision {
        debug!(request_id = %context.request_id, "Evaluating policy");

        // Determine which policy to use
        let policy_id = if context.jurisdiction.as_deref() == Some("JP") {
            "policy.jp.v1"
        } else {
            &self.default_policy_id
        };

        let policies = self.policies.read().await;

        // Get the policy and its parent chain
        let mut all_rules: Vec<(i32, &Rule)> = Vec::new();

        // First, add rules from the selected policy
        if let Some(loaded) = policies.get(policy_id) {
            for rule in &loaded.policy.rules {
                if rule.enabled {
                    all_rules.push((rule.priority, rule));
                }
            }

            // Then, add rules from parent policy (if any)
            if let Some(parent_id) = &loaded.policy.parent_policy_id {
                let parent_key = format!(
                    "{}.{}.v{}",
                    parent_id.namespace, parent_id.id, parent_id.version
                );
                if let Some(parent) = policies.get(&parent_key) {
                    for rule in &parent.policy.rules {
                        if rule.enabled {
                            all_rules.push((rule.priority, rule));
                        }
                    }
                }
            }
        }

        // Sort by priority (higher first)
        all_rules.sort_by(|a, b| b.0.cmp(&a.0));

        // Evaluate rules
        let mut matched_rules = Vec::new();
        let mut decision = DecisionType::DecisionAllow;
        let mut reason = "No matching rules".to_string();

        for (_, rule) in all_rules {
            if self.evaluate_condition(&rule.condition, context) {
                matched_rules.push(rule.rule_id.clone());

                match RuleType::try_from(rule.rule_type).unwrap_or(RuleType::Unspecified) {
                    RuleType::Deny => {
                        decision = DecisionType::DecisionDeny;
                        reason = rule
                            .action
                            .as_ref()
                            .map(|a| a.error_message.clone())
                            .unwrap_or_else(|| rule.description.clone());
                        break; // Deny is final
                    }
                    RuleType::Throttle => {
                        decision = DecisionType::DecisionThrottle;
                        reason = rule.description.clone();
                        // Continue to check for Deny rules
                    }
                    RuleType::Allow => {
                        if decision != DecisionType::DecisionThrottle
                            && decision != DecisionType::DecisionDeny
                        {
                            decision = DecisionType::DecisionAllow;
                            reason = "Request allowed".to_string();
                        }
                    }
                    RuleType::Audit => {
                        // Audit doesn't affect decision
                        debug!(rule_id = %rule.rule_id, "Audit rule matched");
                    }
                    _ => {}
                }
            }
        }

        info!(
            request_id = %context.request_id,
            decision = ?decision,
            matched_rules = ?matched_rules,
            "Policy evaluation complete"
        );

        PolicyDecision {
            decision: decision.into(),
            reason,
            matched_rules,
        }
    }

    /// Evaluate a single condition
    fn evaluate_condition(&self, condition: &Option<Condition>, context: &RequestContext) -> bool {
        let condition = match condition {
            Some(c) => c,
            None => return true, // No condition = always match
        };

        match ConditionType::try_from(condition.condition_type).unwrap_or(ConditionType::Unspecified)
        {
            ConditionType::FieldMatch => self.evaluate_field_match(condition, context),
            ConditionType::ContentPattern => self.evaluate_content_pattern(condition, context),
            ConditionType::CapabilityCheck => self.evaluate_capability_check(condition, context),
            ConditionType::Composite => self.evaluate_composite(condition, context),
            ConditionType::TokenBalance => self.evaluate_token_balance(condition, context),
            ConditionType::TimeRange => self.evaluate_time_range(condition, context),
            ConditionType::LptThreshold => self.evaluate_lpt_threshold(condition, context),
            _ => false,
        }
    }

    /// Extract string value from condition
    fn get_string_value<'a>(&self, condition: &'a Condition) -> Option<&'a str> {
        match &condition.value {
            Some(condition::Value::StringValue(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Extract double value from condition
    fn get_double_value(&self, condition: &Condition) -> Option<f64> {
        match &condition.value {
            Some(condition::Value::DoubleValue(d)) => Some(*d),
            _ => None,
        }
    }

    /// Evaluate field match condition
    fn evaluate_field_match(&self, condition: &Condition, context: &RequestContext) -> bool {
        let field_path = &condition.field_path;
        let operator =
            Operator::try_from(condition.operator).unwrap_or(Operator::Unspecified);

        // Simple field matching (in production: use JSONPath)
        let field_value = match field_path.as_str() {
            "$.credential" => context.credential.is_some().to_string(),
            _ => "unknown".to_string(),
        };

        let compare_value = self.get_string_value(condition).unwrap_or("");

        match operator {
            Operator::Equals => field_value == compare_value,
            Operator::NotEquals => field_value != compare_value,
            Operator::Contains => field_value.contains(compare_value),
            _ => false,
        }
    }

    /// Evaluate content pattern condition
    fn evaluate_content_pattern(&self, condition: &Condition, context: &RequestContext) -> bool {
        let pattern = match self.get_string_value(condition) {
            Some(p) => p,
            None => return false,
        };

        let content = if let Some(payload) = &context.payload {
            payload
                .messages
                .iter()
                .map(|m| m.content.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            String::new()
        };

        // Simple regex matching
        match regex::Regex::new(pattern) {
            Ok(re) => re.is_match(&content),
            Err(_) => {
                warn!(pattern = %pattern, "Invalid regex pattern");
                false
            }
        }
    }

    /// Evaluate capability check condition
    fn evaluate_capability_check(&self, condition: &Condition, context: &RequestContext) -> bool {
        let threshold = self.get_double_value(condition).unwrap_or(0.0);
        let operator =
            Operator::try_from(condition.operator).unwrap_or(Operator::Unspecified);

        let capability_score = context
            .credential
            .as_ref()
            .and_then(|c| c.capability.as_ref())
            .map(|cap| cap.total)
            .unwrap_or(0.0);

        match operator {
            Operator::LessThan => capability_score < threshold,
            Operator::LessThanOrEquals => capability_score <= threshold,
            Operator::GreaterThan => capability_score > threshold,
            Operator::GreaterThanOrEquals => capability_score >= threshold,
            Operator::Equals => (capability_score - threshold).abs() < f64::EPSILON,
            _ => false,
        }
    }

    /// Evaluate composite condition (AND/OR)
    fn evaluate_composite(&self, condition: &Condition, context: &RequestContext) -> bool {
        let logical_op = LogicalOperator::try_from(condition.logical_operator)
            .unwrap_or(LogicalOperator::Unspecified);

        if condition.sub_conditions.is_empty() {
            return true;
        }

        match logical_op {
            LogicalOperator::And => condition
                .sub_conditions
                .iter()
                .all(|c| self.evaluate_condition(&Some(c.clone()), context)),
            LogicalOperator::Or => condition
                .sub_conditions
                .iter()
                .any(|c| self.evaluate_condition(&Some(c.clone()), context)),
            LogicalOperator::Not => {
                if let Some(first) = condition.sub_conditions.first() {
                    !self.evaluate_condition(&Some(first.clone()), context)
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    /// Evaluate token balance condition
    fn evaluate_token_balance(&self, _condition: &Condition, _context: &RequestContext) -> bool {
        // Would need access to TokenService
        // For now, return true (always have tokens)
        true
    }

    /// Evaluate time range condition
    fn evaluate_time_range(&self, _condition: &Condition, _context: &RequestContext) -> bool {
        // Would check if current time is within specified range
        // For now, return true
        true
    }

    /// Evaluate LPT threshold condition
    fn evaluate_lpt_threshold(&self, _condition: &Condition, _context: &RequestContext) -> bool {
        // Would calculate LPT score
        // For now, return true
        true
    }

    /// Register a new policy
    pub async fn register_policy(&self, policy: PolicyPack) -> Result<(), String> {
        let policy_id = policy
            .policy_id
            .as_ref()
            .map(|id| format!("{}.{}.v{}", id.namespace, id.id, id.version))
            .ok_or("Policy ID required")?;

        info!(policy_id = %policy_id, "Registering new policy");

        let mut policies = self.policies.write().await;
        policies.insert(
            policy_id,
            LoadedPolicy {
                policy,
                state: PolicyState::Draft,
            },
        );

        Ok(())
    }

    /// Activate a policy
    pub async fn activate_policy(&self, policy_id: &str) -> Result<(), String> {
        let mut policies = self.policies.write().await;
        let loaded = policies.get_mut(policy_id).ok_or("Policy not found")?;

        loaded.state = PolicyState::Active;
        info!(policy_id = %policy_id, "Policy activated");
        Ok(())
    }

    /// Get policy by ID
    pub async fn get_policy(&self, policy_id: &str) -> Option<PolicyPack> {
        let policies = self.policies.read().await;
        policies.get(policy_id).map(|p| p.policy.clone())
    }

    /// List all active policies
    pub async fn list_active_policies(&self) -> Vec<PolicyPack> {
        let policies = self.policies.read().await;
        policies
            .values()
            .filter(|p| p.state == PolicyState::Active)
            .map(|p| p.policy.clone())
            .collect()
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::chinju::credential::{CapabilityScore, MeasurementContext};

    #[tokio::test]
    async fn test_default_policy_allows_with_high_capability() {
        let engine = PolicyEngine::new();

        // Create credential with high capability score
        let mut credential = HumanCredential::default();
        credential.capability = Some(CapabilityScore {
            independence: 0.7,
            detection: 0.7,
            alternatives: 0.7,
            critique: 0.7,
            total: 0.7, // Above the 0.5 throttle threshold
            context: Some(MeasurementContext::default()),
        });

        let context = RequestContext {
            request_id: "test-1".to_string(),
            credential: Some(credential),
            payload: None,
            client_ip: None,
            jurisdiction: None,
            attributes: HashMap::new(),
        };

        let decision = engine.evaluate(&context).await;
        assert_eq!(decision.decision(), DecisionType::DecisionAllow);
    }

    #[tokio::test]
    async fn test_policy_throttles_low_capability() {
        let engine = PolicyEngine::new();

        // Create credential with low capability score
        let mut credential = HumanCredential::default();
        credential.capability = Some(CapabilityScore {
            independence: 0.3,
            detection: 0.3,
            alternatives: 0.3,
            critique: 0.3,
            total: 0.3, // Below the 0.5 throttle threshold
            context: Some(MeasurementContext::default()),
        });

        let context = RequestContext {
            request_id: "test-2".to_string(),
            credential: Some(credential),
            payload: None,
            client_ip: None,
            jurisdiction: None,
            attributes: HashMap::new(),
        };

        let decision = engine.evaluate(&context).await;
        assert_eq!(decision.decision(), DecisionType::DecisionThrottle);
    }

    #[tokio::test]
    async fn test_policy_matches_rules() {
        let engine = PolicyEngine::new();
        let context = RequestContext {
            request_id: "test-3".to_string(),
            credential: None,
            payload: None,
            client_ip: None,
            jurisdiction: None,
            attributes: HashMap::new(),
        };

        let decision = engine.evaluate(&context).await;
        // Should match audit_all and default_allow at minimum
        assert!(!decision.matched_rules.is_empty());
    }
}
