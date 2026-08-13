use crate::evaluation::{digest_json, ConfidenceSummary, RunManifest};
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub const RUN_BOUNDARY_SCHEMA_VERSION: &str = "RunBoundaryV1";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    Observed,
    Declared,
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ObservedField {
    pub state: EvidenceState,
    pub value: Value,
    pub source_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoundaryIdentity {
    pub id: String,
    pub digest: Option<String>,
    pub attributes: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoundaryCandidate {
    pub candidate_id: String,
    pub model_tag: String,
    pub artifact_digest: String,
    pub format: ObservedField,
    pub quantization: ObservedField,
    pub effective_context: ObservedField,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoundaryScope {
    pub host: BoundaryIdentity,
    pub runtime: BoundaryIdentity,
    pub candidates: Vec<BoundaryCandidate>,
    pub workload: BoundaryIdentity,
    pub execution_semantics: BTreeMap<String, ObservedField>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryDecisionStatus {
    DirectionalChoice,
    Abstain,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryEvidenceStatus {
    Valid,
    Partial,
    Invalid,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoundaryDecision {
    pub status: BoundaryDecisionStatus,
    pub evidence_status: BoundaryEvidenceStatus,
    pub subject: String,
    pub summary: String,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TruthClass {
    Observation,
    Policy,
    Inference,
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceReferenceKind {
    Manifest,
    Trial,
    Result,
    #[allow(dead_code)]
    JudgeAttempt,
    Comparison,
    #[allow(dead_code)]
    ProofRecord,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EvidenceReference {
    pub kind: EvidenceReferenceKind,
    pub id: String,
    pub digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoundaryStatement {
    pub code: String,
    pub truth_class: TruthClass,
    pub statement: String,
    pub evidence_refs: Vec<EvidenceReference>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoundaryExclusion {
    pub code: String,
    pub truth_class: TruthClass,
    pub statement: String,
    pub evidence_refs: Vec<EvidenceReference>,
    pub consequence: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoundaryUnknown {
    pub code: String,
    pub truth_class: TruthClass,
    pub statement: String,
    pub evidence_refs: Vec<EvidenceReference>,
    pub blocks_claims: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BoundaryClearance {
    pub code: String,
    pub statement: String,
    pub proof_required: String,
    pub new_run_required: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryDisposition {
    Blocking,
    Advisory,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoundaryReason {
    pub code: String,
    pub scope: String,
    pub disposition: BoundaryDisposition,
    pub truth_class: TruthClass,
    pub statement: String,
    pub evidence_refs: Vec<EvidenceReference>,
    pub clearance: Vec<BoundaryClearance>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BoundaryPrivacy {
    pub content_mode: String,
    pub local_only: bool,
    pub raw_prompts_included: bool,
    pub raw_outputs_included: bool,
    pub redaction: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoundaryDerivation {
    pub kind: String,
    pub generator: String,
    pub source_run_key: String,
    pub source_manifest_digest: ObservedField,
    pub source_evidence_digest: ObservedField,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RunBoundary {
    pub schema_version: String,
    pub boundary_id: String,
    pub generated_at: String,
    pub derivation: BoundaryDerivation,
    pub scope: BoundaryScope,
    pub decision: BoundaryDecision,
    pub observations: Vec<BoundaryStatement>,
    pub exclusions: Vec<BoundaryExclusion>,
    pub unknowns: Vec<BoundaryUnknown>,
    pub boundaries: Vec<BoundaryReason>,
    pub unsupported_claims: Vec<String>,
    pub privacy: BoundaryPrivacy,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundaryTrialInput {
    pub trial_key: String,
    pub trial_kind: String,
    pub status: String,
    pub model_id: i64,
    pub category: String,
    pub result_id: Option<i64>,
    pub exclusion_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundaryCapabilityInput {
    pub category: String,
    pub model_id: i64,
    pub model_name: String,
    pub scoring_method: String,
    pub confidence: ConfidenceSummary,
    pub result_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundaryRecommendationInput {
    pub category: String,
    pub recommended_model: Option<String>,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExplicitBoundaryKind {
    Quality,
    Constraint,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExplicitBoundaryInput {
    pub kind: ExplicitBoundaryKind,
    pub scope: String,
    pub passed: bool,
    pub statement: String,
    pub evidence_refs: Vec<EvidenceReference>,
    pub clearance: BoundaryClearance,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunBoundaryInput {
    pub run_id: i64,
    pub run_started_at: String,
    pub outcome_status: String,
    pub manifest_digest: Option<String>,
    pub manifest: Option<RunManifest>,
    pub comparable: bool,
    pub comparability_notes: Option<String>,
    pub planned_measured_trials: i64,
    pub completed_measured_trials: i64,
    pub trials: Vec<BoundaryTrialInput>,
    pub capability_evidence: Vec<BoundaryCapabilityInput>,
    pub recommendations: Vec<BoundaryRecommendationInput>,
    pub judge_disagreement_rate: Option<f64>,
    pub judge_disagreement_sufficient: bool,
    pub position_bias_detected: bool,
    pub position_bias_warning: Option<String>,
    pub comparison_ids: Vec<i64>,
    pub runtime_identity_stable: Option<bool>,
    pub explicit_boundaries: Vec<ExplicitBoundaryInput>,
    pub development_only: bool,
}

fn observed(value: Value, source_ref: impl Into<String>) -> ObservedField {
    ObservedField {
        state: EvidenceState::Observed,
        value,
        source_ref: Some(source_ref.into()),
    }
}

fn declared(value: Value, source_ref: impl Into<String>) -> ObservedField {
    ObservedField {
        state: EvidenceState::Declared,
        value,
        source_ref: Some(source_ref.into()),
    }
}

fn unknown(source_ref: Option<String>) -> ObservedField {
    ObservedField {
        state: EvidenceState::Unknown,
        value: Value::Null,
        source_ref,
    }
}

fn normalized_sha256(value: &str) -> Option<String> {
    let raw = value.strip_prefix("sha256:").unwrap_or(value);
    (raw.len() == 64 && raw.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| format!("sha256:{}", raw.to_ascii_lowercase()))
}

fn manifest_reference(input: &RunBoundaryInput) -> EvidenceReference {
    EvidenceReference {
        kind: EvidenceReferenceKind::Manifest,
        id: format!("run:{}:manifest", input.run_id),
        digest: input.manifest_digest.as_deref().and_then(normalized_sha256),
    }
}

fn result_references(inputs: &[&BoundaryCapabilityInput]) -> Vec<EvidenceReference> {
    let mut ids = BTreeSet::new();
    for input in inputs {
        ids.extend(input.result_ids.iter().copied());
    }
    ids.into_iter()
        .map(|id| EvidenceReference {
            kind: EvidenceReferenceKind::Result,
            id: id.to_string(),
            digest: None,
        })
        .collect()
}

fn completed_result_references(input: &RunBoundaryInput) -> Vec<EvidenceReference> {
    let mut ids = BTreeSet::new();
    for trial in &input.trials {
        if trial.trial_kind == "measured" && trial.status == "completed" {
            if let Some(result_id) = trial.result_id {
                ids.insert(result_id);
            }
        }
    }
    ids.into_iter()
        .map(|id| EvidenceReference {
            kind: EvidenceReferenceKind::Result,
            id: id.to_string(),
            digest: None,
        })
        .collect()
}

fn has_rescorable_sample_distribution(input: &RunBoundaryInput) -> bool {
    let mut counts: HashMap<&str, HashMap<i64, usize>> = HashMap::new();
    for trial in &input.trials {
        if trial.trial_kind == "measured"
            && trial.status == "completed"
            && trial.result_id.is_some()
        {
            *counts
                .entry(trial.category.as_str())
                .or_default()
                .entry(trial.model_id)
                .or_default() += 1;
        }
    }
    counts.values().any(|by_model| {
        by_model
            .values()
            .filter(|sample_count| **sample_count >= 3)
            .count()
            >= 2
    })
}

fn comparison_references(input: &RunBoundaryInput) -> Vec<EvidenceReference> {
    let refs: Vec<_> = input
        .comparison_ids
        .iter()
        .map(|id| EvidenceReference {
            kind: EvidenceReferenceKind::Comparison,
            id: id.to_string(),
            digest: None,
        })
        .collect();
    if refs.is_empty() {
        vec![manifest_reference(input)]
    } else {
        refs
    }
}

fn normalized_timestamp(value: &str) -> String {
    if value.contains('T') {
        if value.ends_with('Z') || value.contains('+') {
            value.to_string()
        } else {
            format!("{value}Z")
        }
    } else {
        format!("{}Z", value.replacen(' ', "T", 1))
    }
}

fn identity_scope(input: &RunBoundaryInput) -> BoundaryScope {
    let manifest_ref = format!("run:{}:manifest", input.run_id);
    let Some(manifest) = &input.manifest else {
        return BoundaryScope {
            host: BoundaryIdentity {
                id: "unknown-host".into(),
                digest: None,
                attributes: Map::new(),
            },
            runtime: BoundaryIdentity {
                id: "unknown-runtime".into(),
                digest: None,
                attributes: Map::new(),
            },
            candidates: Vec::new(),
            workload: BoundaryIdentity {
                id: "unknown-workload".into(),
                digest: None,
                attributes: Map::new(),
            },
            execution_semantics: BTreeMap::from([
                ("cache".into(), unknown(None)),
                ("concurrency".into(), unknown(None)),
                ("context".into(), unknown(None)),
                ("retries".into(), unknown(None)),
                ("timeout_seconds".into(), unknown(None)),
                ("warm_state".into(), unknown(None)),
            ]),
        };
    };

    let host_digest = digest_json(&manifest.hardware)
        .ok()
        .and_then(|value| normalized_sha256(&value));
    let runtime_digest = digest_json(&manifest.ollama)
        .ok()
        .and_then(|value| normalized_sha256(&value));
    let mut host_attributes = Map::new();
    host_attributes.insert("architecture".into(), json!(manifest.hardware.architecture));
    host_attributes.insert(
        "logical_cpu_count".into(),
        json!(manifest.hardware.logical_cpu_count),
    );
    host_attributes.insert(
        "total_memory_bytes".into(),
        json!(manifest.hardware.total_memory_bytes),
    );
    let mut runtime_attributes = Map::new();
    runtime_attributes.insert("endpoint".into(), json!(manifest.ollama.endpoint));
    runtime_attributes.insert(
        "server_version".into(),
        json!(manifest.ollama.server_version),
    );
    let candidates = manifest
        .models
        .iter()
        .filter_map(|model| {
            let artifact_digest = model.digest.as_deref().and_then(normalized_sha256)?;
            Some(BoundaryCandidate {
                candidate_id: format!("model:{}", model.database_id),
                model_tag: model.exact_tag.clone(),
                artifact_digest,
                format: unknown(Some(manifest_ref.clone())),
                quantization: model
                    .quantization
                    .as_ref()
                    .map(|value| observed(json!(value), manifest_ref.clone()))
                    .unwrap_or_else(|| unknown(Some(manifest_ref.clone()))),
                effective_context: unknown(Some(manifest_ref.clone())),
            })
        })
        .collect();
    let mut workload_attributes = Map::new();
    workload_attributes.insert("name".into(), json!(manifest.suite.name));
    workload_attributes.insert("prompt_count".into(), json!(manifest.suite.prompts.len()));

    BoundaryScope {
        host: BoundaryIdentity {
            id: format!("host:{}", manifest.hardware.architecture),
            digest: host_digest,
            attributes: host_attributes,
        },
        runtime: BoundaryIdentity {
            id: format!("ollama:{}", manifest.ollama.server_version),
            digest: runtime_digest,
            attributes: runtime_attributes,
        },
        candidates,
        workload: BoundaryIdentity {
            id: format!("suite:{}", manifest.suite.id),
            digest: normalized_sha256(&manifest.suite.digest),
            attributes: workload_attributes,
        },
        execution_semantics: BTreeMap::from([
            ("cache".into(), unknown(None)),
            (
                "concurrency".into(),
                declared(json!(1), "benchmark-runner:serial-execution"),
            ),
            ("context".into(), unknown(Some(manifest_ref.clone()))),
            (
                "retries".into(),
                declared(json!(0), "benchmark-runner:no-retry"),
            ),
            (
                "timeout_seconds".into(),
                observed(
                    json!(manifest.generation.timeout_seconds),
                    manifest_ref.clone(),
                ),
            ),
            (
                "warm_state".into(),
                declared(
                    json!({
                        "warmup_repetitions": manifest.generation.warmup_repetitions,
                        "filesystem_cache": "unknown"
                    }),
                    manifest_ref,
                ),
            ),
        ]),
    }
}

fn blocking_reason(
    code: &str,
    scope: String,
    truth_class: TruthClass,
    statement: String,
    evidence_refs: Vec<EvidenceReference>,
    clearance: BoundaryClearance,
) -> BoundaryReason {
    BoundaryReason {
        code: code.into(),
        scope,
        disposition: BoundaryDisposition::Blocking,
        truth_class,
        statement,
        evidence_refs,
        clearance: vec![clearance],
    }
}

fn advisory_reason(
    code: &str,
    scope: &str,
    truth_class: TruthClass,
    statement: impl Into<String>,
    evidence_refs: Vec<EvidenceReference>,
) -> BoundaryReason {
    BoundaryReason {
        code: code.into(),
        scope: scope.into(),
        disposition: BoundaryDisposition::Advisory,
        truth_class,
        statement: statement.into(),
        evidence_refs,
        clearance: Vec::new(),
    }
}

fn category_method_groups(
    input: &RunBoundaryInput,
) -> HashMap<(String, String), Vec<&BoundaryCapabilityInput>> {
    let mut groups: HashMap<(String, String), Vec<_>> = HashMap::new();
    for evidence in &input.capability_evidence {
        groups
            .entry((evidence.category.clone(), evidence.scoring_method.clone()))
            .or_default()
            .push(evidence);
    }
    groups
}

fn directional_model_for_method(values: &[&BoundaryCapabilityInput]) -> Option<i64> {
    let mut ranked = values.to_vec();
    ranked.sort_by(|a, b| {
        b.confidence
            .mean
            .partial_cmp(&a.confidence.mean)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let (Some(top), Some(runner_up)) = (ranked.first(), ranked.get(1)) else {
        return None;
    };
    if !top.confidence.sufficient_sample || !runner_up.confidence.sufficient_sample {
        return None;
    }
    match (top.confidence.lower_95, runner_up.confidence.upper_95) {
        (Some(top_low), Some(other_high)) if top_low > other_high => Some(top.model_id),
        _ => None,
    }
}

fn conflicting_method_categories(
    groups: &HashMap<(String, String), Vec<&BoundaryCapabilityInput>>,
) -> BTreeSet<String> {
    let mut winners: HashMap<&str, BTreeSet<i64>> = HashMap::new();
    for ((category, _), values) in groups {
        if let Some(model_id) = directional_model_for_method(values) {
            winners
                .entry(category.as_str())
                .or_default()
                .insert(model_id);
        }
    }
    winners
        .into_iter()
        .filter_map(|(category, model_ids)| (model_ids.len() > 1).then(|| category.to_string()))
        .collect()
}

pub fn derive_run_boundary(input: &RunBoundaryInput) -> RunBoundary {
    let manifest_ref = manifest_reference(input);
    let manifest_refs = vec![manifest_ref.clone()];
    let evidence_digest = digest_json(input)
        .ok()
        .and_then(|value| normalized_sha256(&value));
    let manifest_digest = input.manifest_digest.as_deref().and_then(normalized_sha256);
    let boundary_suffix: String = manifest_digest
        .as_deref()
        .map(|value| {
            value
                .trim_start_matches("sha256:")
                .chars()
                .take(12)
                .collect()
        })
        .unwrap_or_else(|| "legacy".into());

    let mut observations = Vec::new();
    if let Some(manifest) = &input.manifest {
        observations.push(BoundaryStatement {
            code: "manifest_recorded".into(),
            truth_class: TruthClass::Observation,
            statement: format!(
                "The run records {} candidate(s), {} planned measured trial(s), and one immutable workload snapshot.",
                manifest.models.len(), input.planned_measured_trials
            ),
            evidence_refs: manifest_refs.clone(),
        });
    }
    observations.push(BoundaryStatement {
        code: "measured_trial_state".into(),
        truth_class: TruthClass::Observation,
        statement: format!(
            "{} of {} planned measured trials completed.",
            input.completed_measured_trials, input.planned_measured_trials
        ),
        evidence_refs: manifest_refs.clone(),
    });

    let invalid_trials: Vec<_> = input
        .trials
        .iter()
        .filter(|trial| trial.trial_kind == "measured" && trial.status != "completed")
        .collect();
    let invalid_trial_refs: Vec<_> = invalid_trials
        .iter()
        .map(|trial| match trial.result_id {
            Some(result_id) => EvidenceReference {
                kind: EvidenceReferenceKind::Result,
                id: result_id.to_string(),
                digest: None,
            },
            None => EvidenceReference {
                kind: EvidenceReferenceKind::Trial,
                id: trial.trial_key.clone(),
                digest: None,
            },
        })
        .collect();
    let mut exclusions = Vec::new();
    if !invalid_trials.is_empty() {
        exclusions.push(BoundaryExclusion {
            code: "invalid_trials_excluded".into(),
            truth_class: TruthClass::Policy,
            statement: format!(
                "{} measured trial(s) were not completed and are excluded from model-choice evidence.",
                invalid_trials.len()
            ),
            evidence_refs: invalid_trial_refs.clone(),
            consequence: "Their timing, output, and scores cannot support a directional choice.".into(),
        });
    }

    let mut unknowns = vec![
        BoundaryUnknown {
            code: "prompt_cache_state_unknown".into(),
            truth_class: TruthClass::Unknown,
            statement: "The run does not record an authoritative prompt-cache state.".into(),
            evidence_refs: manifest_refs.clone(),
            blocks_claims: vec!["cache benefit".into(), "filesystem-cold performance".into()],
        },
        BoundaryUnknown {
            code: "thermal_attribution_unknown".into(),
            truth_class: TruthClass::Unknown,
            statement: "The run does not contain calibrated thermal attribution evidence.".into(),
            evidence_refs: manifest_refs.clone(),
            blocks_claims: vec!["thermal causality".into(), "sustained endurance".into()],
        },
        BoundaryUnknown {
            code: "context_effective_value_unknown".into(),
            truth_class: TruthClass::Unknown,
            statement: "The manifest does not record an effective loaded-context readback.".into(),
            evidence_refs: manifest_refs.clone(),
            blocks_claims: vec!["effective-context behavior".into()],
        },
        BoundaryUnknown {
            code: "transfer_unknown".into(),
            truth_class: TruthClass::Unknown,
            statement: "No cross-host, production, demand, or long-duration evidence is attached to this run.".into(),
            evidence_refs: manifest_refs.clone(),
            blocks_claims: vec![
                "cross-host transfer".into(),
                "production reliability".into(),
                "user demand".into(),
            ],
        },
        BoundaryUnknown {
            code: "external_proof_freshness_unknown".into(),
            truth_class: TruthClass::Unknown,
            statement: "This boundary has no current external-proof freshness receipt; absent or stale external proof cannot support a current claim.".into(),
            evidence_refs: manifest_refs.clone(),
            blocks_claims: vec!["current external state".into()],
        },
    ];
    if input.runtime_identity_stable.is_none() {
        unknowns.push(BoundaryUnknown {
            code: "runtime_identity_end_state_unknown".into(),
            truth_class: TruthClass::Unknown,
            statement: "The run records startup identity but no authoritative end-of-window identity readback.".into(),
            evidence_refs: manifest_refs.clone(),
            blocks_claims: vec!["runtime identity stability across the evidence window".into()],
        });
    }

    let mut boundaries = Vec::new();
    let manifest_identity_conflict = input.manifest.as_ref().is_some_and(|manifest| {
        let mut by_database_id = HashMap::new();
        let mut by_tag = HashMap::new();
        manifest.models.iter().any(|model| {
            let digest = model.digest.clone();
            by_database_id
                .insert(model.database_id, digest.clone())
                .is_some_and(|previous| previous != digest)
                || by_tag
                    .insert(model.exact_tag.clone(), digest.clone())
                    .is_some_and(|previous| previous != digest)
        })
    });
    let provenance_issues = match &input.manifest {
        None => vec!["immutable run manifest is missing".to_string()],
        Some(manifest) => {
            let mut issues = Vec::new();
            if manifest_digest.is_none() {
                issues.push("manifest digest is missing or malformed".into());
            } else {
                let calculated = serde_json::to_string_pretty(manifest)
                    .ok()
                    .map(|value| crate::evaluation::sha256_hex(value.as_bytes()))
                    .and_then(|value| normalized_sha256(&value));
                if calculated != manifest_digest {
                    issues.push("manifest digest does not match the stored manifest".into());
                }
            }
            if manifest.models.is_empty() {
                issues.push("candidate identity is missing".into());
            } else if manifest.models.iter().any(|model| {
                model
                    .digest
                    .as_deref()
                    .and_then(normalized_sha256)
                    .is_none()
            }) {
                issues.push("one or more exact artifact digests are missing".into());
            }
            if manifest.ollama.server_version.trim().is_empty() {
                issues.push("runtime version is missing".into());
            }
            if normalized_sha256(&manifest.suite.digest).is_none() {
                issues.push("workload digest is missing or malformed".into());
            }
            if manifest
                .suite
                .prompts
                .iter()
                .any(|prompt| normalized_sha256(&prompt.digest).is_none())
            {
                issues.push("one or more workload case digests are missing or malformed".into());
            }
            if manifest.measured_trial_count as i64 != input.planned_measured_trials {
                issues.push("manifest and stored trial-plan counts disagree".into());
            }
            if !input.comparable
                && input
                    .recommendations
                    .iter()
                    .any(|recommendation| recommendation.recommended_model.is_some())
            {
                issues.push(
                    "a directional recommendation contradicts the run comparability state".into(),
                );
            }
            issues
        }
    };
    if !provenance_issues.is_empty() {
        boundaries.push(blocking_reason(
            "provenance_missing",
            "entire run".into(),
            TruthClass::Observation,
            format!("Required provenance is incomplete: {}.", provenance_issues.join("; ")),
            manifest_refs.clone(),
            BoundaryClearance {
                code: "record_complete_manifest".into(),
                statement: "Create a new run with an immutable manifest containing exact candidate, runtime, host, and workload identity.".into(),
                proof_required: "A valid manifest digest plus exact model artifact digests, runtime version, host snapshot, and workload digest.".into(),
                new_run_required: true,
            },
        ));
    }

    if input.runtime_identity_stable == Some(false) || manifest_identity_conflict {
        boundaries.push(blocking_reason(
            "runtime_identity_unstable",
            "entire run".into(),
            TruthClass::Observation,
            if manifest_identity_conflict {
                "The manifest contains contradictory exact candidate identities.".into()
            } else {
                "Recorded runtime or candidate identity changed across the evidence window.".into()
            },
            manifest_refs.clone(),
            BoundaryClearance {
                code: "stabilize_runtime_identity".into(),
                statement: "Repeat the run under an isolated identity window with matching pre/post readbacks.".into(),
                proof_required: "Exact artifact, runtime, and effective-context identities matching before and after measurement.".into(),
                new_run_required: true,
            },
        ));
    }

    let run_incomplete = input.outcome_status != "completed"
        || !input.comparable
        || input.planned_measured_trials != input.completed_measured_trials
        || !invalid_trials.is_empty();
    if run_incomplete {
        let refs = if invalid_trial_refs.is_empty() {
            manifest_refs.clone()
        } else {
            invalid_trial_refs.clone()
        };
        let states: BTreeSet<_> = invalid_trials
            .iter()
            .map(|trial| trial.status.as_str())
            .collect();
        boundaries.push(blocking_reason(
            "run_incomplete",
            "entire run".into(),
            TruthClass::Observation,
            format!(
                "The run is '{}' and has {}/{} completed measured trials{}.",
                input.outcome_status,
                input.completed_measured_trials,
                input.planned_measured_trials,
                if states.is_empty() {
                    if input.comparable {
                        String::new()
                    } else {
                        "; the run is not comparable".into()
                    }
                } else {
                    format!("; non-complete states: {}", states.into_iter().collect::<Vec<_>>().join(", "))
                }
            ),
            refs,
            BoundaryClearance {
                code: "complete_valid_trial_plan".into(),
                statement: "Run a complete preregistered measured plan with no failed, timed-out, cancelled, excluded, empty, or missing trials.".into(),
                proof_required: "Every planned measured trial has one valid completed result under the same immutable manifest.".into(),
                new_run_required: true,
            },
        ));
    }

    if input.completed_measured_trials == 0 {
        boundaries.push(blocking_reason(
            "no_measured_evidence",
            "all capabilities".into(),
            TruthClass::Observation,
            "No completed measured result exists.".into(),
            manifest_refs.clone(),
            BoundaryClearance {
                code: "record_eligible_measured_scores".into(),
                statement: "Record valid measured outputs and same-method scores for at least two candidates.".into(),
                proof_required: "At least three valid same-method measured scores per leading candidate for one capability.".into(),
                new_run_required: true,
            },
        ));
    } else if input.capability_evidence.is_empty() {
        let completed_refs = completed_result_references(input);
        let enough_existing_outputs = has_rescorable_sample_distribution(input);
        boundaries.push(blocking_reason(
            "no_scored_evidence",
            "all capabilities".into(),
            TruthClass::Observation,
            "Completed measured outputs exist, but no eligible same-method score evidence exists.".into(),
            if completed_refs.is_empty() {
                manifest_refs.clone()
            } else {
                completed_refs
            },
            BoundaryClearance {
                code: "record_eligible_measured_scores".into(),
                statement: "Score the completed measured outputs for at least two candidates with one shared method.".into(),
                proof_required: "At least three valid same-method measured scores per leading candidate for one capability.".into(),
                new_run_required: !enough_existing_outputs,
            },
        ));
    }

    let empty_outputs: Vec<_> = input
        .trials
        .iter()
        .filter(|trial| {
            trial.trial_kind == "measured"
                && trial.status == "excluded"
                && trial.exclusion_reason.as_deref() == Some("empty output")
        })
        .collect();
    if !empty_outputs.is_empty() {
        let refs = empty_outputs
            .iter()
            .map(|trial| match trial.result_id {
                Some(result_id) => EvidenceReference {
                    kind: EvidenceReferenceKind::Result,
                    id: result_id.to_string(),
                    digest: None,
                },
                None => EvidenceReference {
                    kind: EvidenceReferenceKind::Trial,
                    id: trial.trial_key.clone(),
                    digest: None,
                },
            })
            .collect();
        boundaries.push(blocking_reason(
            "model_failure",
            "measured model outputs".into(),
            TruthClass::Observation,
            format!(
                "{} measured request(s) completed without a usable model output.",
                empty_outputs.len()
            ),
            refs,
            BoundaryClearance {
                code: "produce_valid_non_empty_output".into(),
                statement: "Repeat the affected measured cases and require a valid non-empty output under the same preregistered validator.".into(),
                proof_required: "A completed measured result with non-empty output for every affected case; timeout or runtime recovery alone is insufficient.".into(),
                new_run_required: true,
            },
        ));
    }

    let groups = category_method_groups(input);
    let conflicting_categories = conflicting_method_categories(&groups);
    let directional_categories: BTreeSet<_> = input
        .recommendations
        .iter()
        .filter(|recommendation| {
            recommendation.recommended_model.is_some()
                && !conflicting_categories.contains(&recommendation.category)
        })
        .map(|recommendation| recommendation.category.clone())
        .collect();
    let directional_inputs: Vec<_> = input
        .capability_evidence
        .iter()
        .filter(|evidence| directional_categories.contains(&evidence.category))
        .collect();
    if !directional_inputs.is_empty() {
        observations.push(BoundaryStatement {
            code: "directional_capability_evidence".into(),
            truth_class: TruthClass::Observation,
            statement: "The named directional choice is supported by the referenced same-method measured results.".into(),
            evidence_refs: result_references(&directional_inputs),
        });
    }
    let categories: BTreeSet<_> = input
        .capability_evidence
        .iter()
        .map(|evidence| evidence.category.clone())
        .collect();
    let mut mismatch_categories = Vec::new();
    let mut insufficient_categories = Vec::new();
    let mut overlap_categories = Vec::new();
    let mut mismatch_inputs = Vec::new();
    let mut insufficient_inputs = Vec::new();
    let mut overlap_inputs = Vec::new();

    for category in categories {
        if directional_categories.contains(&category) {
            continue;
        }
        let category_inputs: Vec<_> = input
            .capability_evidence
            .iter()
            .filter(|evidence| evidence.category == category)
            .collect();
        let distinct_models: BTreeSet<_> = category_inputs
            .iter()
            .map(|evidence| evidence.model_id)
            .collect();
        let recommendation_confidence = input
            .recommendations
            .iter()
            .find(|recommendation| recommendation.category == category)
            .map(|recommendation| recommendation.confidence.as_str());
        let shared_groups: Vec<_> = groups
            .iter()
            .filter(|((group_category, _), values)| {
                group_category == &category
                    && values
                        .iter()
                        .map(|value| value.model_id)
                        .collect::<BTreeSet<_>>()
                        .len()
                        >= 2
            })
            .collect();
        if conflicting_categories.contains(&category)
            || distinct_models.len() < 2
            || shared_groups.is_empty()
            || recommendation_confidence == Some("judge-sensitive")
        {
            mismatch_categories.push(category.clone());
            mismatch_inputs.extend(category_inputs);
            continue;
        }
        if shared_groups.iter().any(|(_, values)| {
            values
                .iter()
                .any(|value| !value.confidence.sufficient_sample)
        }) {
            insufficient_categories.push(category.clone());
            insufficient_inputs.extend(category_inputs);
            continue;
        }
        if !shared_groups.is_empty() {
            overlap_categories.push(category.clone());
            overlap_inputs.extend(category_inputs);
        }
    }

    if !mismatch_categories.is_empty() {
        boundaries.push(blocking_reason(
            "judge_method_mismatch",
            mismatch_categories.join(", "),
            TruthClass::Observation,
            "At least two candidates lack one non-conflicting eligible scoring method for the named capabilities.".into(),
            result_references(&mismatch_inputs),
            BoundaryClearance {
                code: "score_with_one_shared_method".into(),
                statement: "Score both candidates with one shared human or exact local-judge method that supports a common direction.".into(),
                proof_required: "Same-method result evidence for both candidates on the same capability and workload cases, without conflicting method-specific direction.".into(),
                new_run_required: false,
            },
        ));
    }
    if !insufficient_categories.is_empty() {
        boundaries.push(blocking_reason(
            "insufficient_samples",
            insufficient_categories.join(", "),
            TruthClass::Policy,
            "At least one leading candidate has fewer than three valid same-method samples.".into(),
            result_references(&insufficient_inputs),
            BoundaryClearance {
                code: "reach_minimum_same_method_samples".into(),
                statement: "Collect at least three valid same-method measured scores for each leading candidate.".into(),
                proof_required: "Three or more eligible samples per candidate for the same capability and scoring method.".into(),
                new_run_required: true,
            },
        ));
    }

    let failed_quality: Vec<_> = input
        .explicit_boundaries
        .iter()
        .filter(|boundary| boundary.kind == ExplicitBoundaryKind::Quality && !boundary.passed)
        .collect();
    if !failed_quality.is_empty() {
        boundaries.push(blocking_reason(
            "quality_gate_failed",
            failed_quality
                .iter()
                .map(|boundary| boundary.scope.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            TruthClass::Observation,
            failed_quality
                .iter()
                .map(|boundary| boundary.statement.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            failed_quality
                .iter()
                .flat_map(|boundary| boundary.evidence_refs.clone())
                .collect(),
            failed_quality[0].clearance.clone(),
        ));
    }

    if !overlap_categories.is_empty() {
        boundaries.push(blocking_reason(
            "uncertainty_overlap",
            overlap_categories.join(", "),
            TruthClass::Inference,
            "The leading candidates' recorded uncertainty intervals overlap.".into(),
            result_references(&overlap_inputs),
            BoundaryClearance {
                code: "resolve_interval_overlap".into(),
                statement: "Collect additional preregistered same-method evidence and recompute the descriptive intervals.".into(),
                proof_required: "Sufficient valid samples whose recorded intervals no longer overlap; more samples alone do not guarantee clearance.".into(),
                new_run_required: true,
            },
        ));
    }

    let timeout_trials: Vec<_> = input
        .trials
        .iter()
        .filter(|trial| trial.trial_kind == "measured" && trial.status == "timeout")
        .collect();
    if !timeout_trials.is_empty() {
        boundaries.push(blocking_reason(
            "constraint_failed",
            "configured request timeout".into(),
            TruthClass::Observation,
            format!(
                "{} measured trial(s) exceeded the timeout declared in the immutable manifest.",
                timeout_trials.len()
            ),
            timeout_trials
                .iter()
                .map(|trial| EvidenceReference {
                    kind: EvidenceReferenceKind::Trial,
                    id: trial.trial_key.clone(),
                    digest: None,
                })
                .collect(),
            BoundaryClearance {
                code: "meet_declared_timeout".into(),
                statement: "Meet the preregistered timeout on every affected measured case, or preregister a different workload contract before a new run.".into(),
                proof_required: "Completed measured trial receipts within the declared timeout under stable identity; a longer timeout does not prove better quality.".into(),
                new_run_required: true,
            },
        ));
    }

    let failed_constraints: Vec<_> = input
        .explicit_boundaries
        .iter()
        .filter(|boundary| boundary.kind == ExplicitBoundaryKind::Constraint && !boundary.passed)
        .collect();
    if !failed_constraints.is_empty()
        && !boundaries
            .iter()
            .any(|boundary| boundary.code == "constraint_failed")
    {
        boundaries.push(blocking_reason(
            "constraint_failed",
            failed_constraints
                .iter()
                .map(|boundary| boundary.scope.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            TruthClass::Observation,
            failed_constraints
                .iter()
                .map(|boundary| boundary.statement.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            failed_constraints
                .iter()
                .flat_map(|boundary| boundary.evidence_refs.clone())
                .collect(),
            failed_constraints[0].clearance.clone(),
        ));
    }

    if input.judge_disagreement_sufficient
        && input.judge_disagreement_rate.is_some_and(|rate| rate > 0.0)
    {
        boundaries.push(advisory_reason(
            "judge_disagreement",
            "human and local auto-judge comparisons",
            TruthClass::Observation,
            format!(
                "Human and local auto-judge outcomes disagree on {:.0}% of paired evidence.",
                input.judge_disagreement_rate.unwrap_or_default() * 100.0
            ),
            comparison_references(input),
        ));
    }
    if input.position_bias_detected || input.position_bias_warning.is_some() {
        boundaries.push(advisory_reason(
            "position_bias_risk",
            "blind comparisons",
            if input.position_bias_detected {
                TruthClass::Observation
            } else {
                TruthClass::Unknown
            },
            input.position_bias_warning.clone().unwrap_or_else(|| {
                "The recorded left/right preference threatens the comparison.".into()
            }),
            comparison_references(input),
        ));
    }
    boundaries.push(advisory_reason(
        "cache_state_unknown",
        "latency attribution",
        TruthClass::Unknown,
        "No authoritative prompt-cache receipt exists.",
        manifest_refs.clone(),
    ));
    boundaries.push(advisory_reason(
        "thermal_attribution_unknown",
        "latency and endurance attribution",
        TruthClass::Unknown,
        "No calibrated thermal attribution evidence exists.",
        manifest_refs.clone(),
    ));
    boundaries.push(advisory_reason(
        "context_effective_value_unknown",
        "effective context",
        TruthClass::Unknown,
        "No effective loaded-context readback exists in the run manifest.",
        manifest_refs.clone(),
    ));
    boundaries.push(advisory_reason(
        "transfer_unsupported",
        "hosts, runtimes, production, and demand",
        TruthClass::Unknown,
        "This local run contains no transfer, production, demand, or long-duration proof.",
        manifest_refs.clone(),
    ));
    if input.development_only {
        boundaries.push(advisory_reason(
            "development_only_evidence",
            "development evidence",
            TruthClass::Policy,
            "The attached evidence is development-only and cannot confirm a deployment choice.",
            manifest_refs.clone(),
        ));
    }
    if !insufficient_categories.is_empty() {
        boundaries.push(advisory_reason(
            "small_sample_warning",
            &insufficient_categories.join(", "),
            TruthClass::Policy,
            "Small samples cannot support universal confidence or transfer claims.",
            result_references(&insufficient_inputs),
        ));
    }

    let blocking: Vec<_> = boundaries
        .iter()
        .filter(|boundary| boundary.disposition == BoundaryDisposition::Blocking)
        .collect();
    let invalid_codes = [
        "provenance_missing",
        "runtime_identity_unstable",
        "run_incomplete",
        "no_measured_evidence",
    ];
    let evidence_status = if blocking
        .iter()
        .any(|boundary| invalid_codes.contains(&boundary.code.as_str()))
    {
        BoundaryEvidenceStatus::Invalid
    } else if blocking.is_empty() {
        BoundaryEvidenceStatus::Valid
    } else {
        BoundaryEvidenceStatus::Partial
    };
    let global_blocker = blocking.iter().any(|boundary| {
        [
            "provenance_missing",
            "runtime_identity_unstable",
            "run_incomplete",
            "no_measured_evidence",
            "no_scored_evidence",
            "constraint_failed",
        ]
        .contains(&boundary.code.as_str())
    });
    let quality_blocked_scopes: BTreeSet<_> = failed_quality
        .iter()
        .map(|boundary| boundary.scope.as_str())
        .collect();
    let supported: Vec<_> = input
        .recommendations
        .iter()
        .filter_map(|recommendation| {
            (!quality_blocked_scopes.contains(recommendation.category.as_str())
                && !conflicting_categories.contains(&recommendation.category))
            .then(|| {
                recommendation
                    .recommended_model
                    .as_ref()
                    .map(|model| format!("{}: {}", recommendation.category, model))
            })
            .flatten()
        })
        .collect();
    let directional = !global_blocker && !supported.is_empty();
    let decision = if directional {
        BoundaryDecision {
            status: BoundaryDecisionStatus::DirectionalChoice,
            evidence_status,
            subject: supported.join(", "),
            summary: format!(
                "Directional choice supported for {} within this run's recorded environment.",
                supported.join(", ")
            ),
            claim_ceiling: format!(
                "This run supports only the named per-capability direction ({}) under its recorded workload, scoring method, runtime, and host; it does not establish deployment fitness or transfer.",
                supported.join(", ")
            ),
        }
    } else {
        let first = blocking.first();
        BoundaryDecision {
            status: BoundaryDecisionStatus::Abstain,
            evidence_status,
            subject: "model choice from this run".into(),
            summary: first
                .map(|boundary| format!("Abstain: {}", boundary.statement))
                .unwrap_or_else(|| "Abstain: no per-capability directional choice is supported.".into()),
            claim_ceiling: "This run supports only its recorded observations and abstention reasons; it does not support a model or deployment choice.".into(),
        }
    };

    RunBoundary {
        schema_version: RUN_BOUNDARY_SCHEMA_VERSION.into(),
        boundary_id: format!("run-boundary:{}:{boundary_suffix}", input.run_id),
        generated_at: normalized_timestamp(&input.run_started_at),
        derivation: BoundaryDerivation {
            kind: "deterministic".into(),
            generator: "model-colosseum/run-boundary-v1".into(),
            source_run_key: input
                .manifest
                .as_ref()
                .map(|manifest| manifest.run_key.clone())
                .unwrap_or_else(|| format!("benchmark-run:{}", input.run_id)),
            source_manifest_digest: manifest_digest
                .map(|value| observed(json!(value), manifest_ref.id.clone()))
                .unwrap_or_else(|| unknown(Some(manifest_ref.id.clone()))),
            source_evidence_digest: evidence_digest
                .map(|value| declared(json!(value), "run-boundary-input-v1"))
                .unwrap_or_else(|| unknown(None)),
        },
        scope: identity_scope(input),
        decision,
        observations,
        exclusions,
        unknowns,
        boundaries,
        unsupported_claims: vec![
            "deployment fitness".into(),
            "cross-host transfer".into(),
            "production reliability".into(),
            "energy use".into(),
            "user demand".into(),
            "product-market fit".into(),
        ],
        privacy: BoundaryPrivacy {
            content_mode: "metadata_only".into(),
            local_only: true,
            raw_prompts_included: false,
            raw_outputs_included: false,
            redaction: "not_performed_content_omitted".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::{
        EvaluationConfig, HardwareSnapshot, ModelSnapshot, OllamaSnapshot, SuiteSnapshot,
    };

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn manifest() -> RunManifest {
        RunManifest {
            schema_version: 1,
            run_key: "eval-7-fixture".into(),
            created_at_unix_ms: 1,
            suite: SuiteSnapshot {
                id: 3,
                name: "Focused suite".into(),
                description: None,
                digest: DIGEST_B.into(),
                prompts: vec![],
            },
            models: vec![
                ModelSnapshot {
                    database_id: 1,
                    exact_tag: "model-a:latest".into(),
                    digest: Some(DIGEST_A.into()),
                    size_bytes: None,
                    parameter_size: None,
                    quantization: Some("Q4_K_M".into()),
                    family: None,
                    modified_at: None,
                    capabilities: vec!["completion".into()],
                },
                ModelSnapshot {
                    database_id: 2,
                    exact_tag: "model-b:latest".into(),
                    digest: Some(DIGEST_B.into()),
                    size_bytes: None,
                    parameter_size: None,
                    quantization: Some("Q4_K_M".into()),
                    family: None,
                    modified_at: None,
                    capabilities: vec!["completion".into()],
                },
            ],
            ollama: OllamaSnapshot {
                server_version: "0.32.9".into(),
                endpoint: "http://localhost:11434".into(),
            },
            hardware: HardwareSnapshot {
                os_name: Some("macOS".into()),
                os_version: Some("26".into()),
                kernel_version: None,
                architecture: "arm64".into(),
                cpu_brand: Some("Apple".into()),
                logical_cpu_count: 12,
                total_memory_bytes: 48 * 1024 * 1024 * 1024,
            },
            generation: EvaluationConfig::default(),
            measured_trial_count: 6,
            warmup_trial_count: 2,
        }
    }

    fn capability(
        model_id: i64,
        model_name: &str,
        method: &str,
        values: &[f64],
        result_start: i64,
    ) -> BoundaryCapabilityInput {
        BoundaryCapabilityInput {
            category: "coding".into(),
            model_id,
            model_name: model_name.into(),
            scoring_method: method.into(),
            confidence: crate::evaluation::mean_confidence_95(values),
            result_ids: (result_start..result_start + values.len() as i64).collect(),
        }
    }

    fn complete_input() -> RunBoundaryInput {
        let manifest = manifest();
        let manifest_digest = crate::evaluation::sha256_hex(
            serde_json::to_string_pretty(&manifest).unwrap().as_bytes(),
        );
        RunBoundaryInput {
            run_id: 7,
            run_started_at: "2026-08-13 10:00:00".into(),
            outcome_status: "completed".into(),
            manifest_digest: Some(manifest_digest),
            manifest: Some(manifest),
            comparable: true,
            comparability_notes: Some("All measured trials completed".into()),
            planned_measured_trials: 6,
            completed_measured_trials: 6,
            trials: (0..6)
                .map(|index| BoundaryTrialInput {
                    trial_key: format!("trial-{index}"),
                    trial_kind: "measured".into(),
                    status: "completed".into(),
                    model_id: if index < 3 { 1 } else { 2 },
                    category: "coding".into(),
                    result_id: Some(index + 1),
                    exclusion_reason: None,
                })
                .collect(),
            capability_evidence: vec![
                capability(1, "Model A", "human_score", &[10.0, 10.0, 10.0], 1),
                capability(2, "Model B", "human_score", &[4.0, 4.0, 4.0], 4),
            ],
            recommendations: vec![BoundaryRecommendationInput {
                category: "coding".into(),
                recommended_model: Some("Model A".into()),
                confidence: "directional".into(),
            }],
            judge_disagreement_rate: None,
            judge_disagreement_sufficient: false,
            position_bias_detected: false,
            position_bias_warning: None,
            comparison_ids: Vec::new(),
            runtime_identity_stable: Some(true),
            explicit_boundaries: Vec::new(),
            development_only: false,
        }
    }

    fn blocking_codes(boundary: &RunBoundary) -> Vec<&str> {
        boundary
            .boundaries
            .iter()
            .filter(|reason| reason.disposition == BoundaryDisposition::Blocking)
            .map(|reason| reason.code.as_str())
            .collect()
    }

    #[test]
    fn complete_directional_run_is_scoped_and_valid() {
        let boundary = derive_run_boundary(&complete_input());
        assert_eq!(
            boundary.decision.status,
            BoundaryDecisionStatus::DirectionalChoice
        );
        assert_eq!(
            boundary.decision.evidence_status,
            BoundaryEvidenceStatus::Valid
        );
        assert!(blocking_codes(&boundary).is_empty());
        assert!(boundary.decision.claim_ceiling.contains("coding: Model A"));
        let directional_refs = boundary
            .observations
            .iter()
            .find(|observation| observation.code == "directional_capability_evidence")
            .unwrap();
        assert_eq!(directional_refs.evidence_refs.len(), 6);
        assert!(directional_refs
            .evidence_refs
            .iter()
            .all(|reference| reference.kind == EvidenceReferenceKind::Result));
    }

    #[test]
    fn incomplete_mixed_trial_states_emit_all_global_blockers_in_order() {
        let mut input = complete_input();
        input.outcome_status = "failed".into();
        input.completed_measured_trials = 0;
        input.capability_evidence.clear();
        input.recommendations.clear();
        input.trials[0].status = "failed".into();
        input.trials[1].status = "timeout".into();
        input.trials[2].status = "cancelled".into();
        input.trials[3].status = "excluded".into();
        let boundary = derive_run_boundary(&input);
        assert_eq!(
            blocking_codes(&boundary),
            vec![
                "run_incomplete",
                "no_measured_evidence",
                "constraint_failed"
            ]
        );
        assert_eq!(
            boundary.decision.evidence_status,
            BoundaryEvidenceStatus::Invalid
        );
    }

    #[test]
    fn completed_but_incomparable_run_still_abstains_as_incomplete() {
        let mut input = complete_input();
        input.comparable = false;
        input.recommendations[0].recommended_model = None;
        input.recommendations[0].confidence = "withheld".into();
        let boundary = derive_run_boundary(&input);
        assert_eq!(blocking_codes(&boundary)[0], "run_incomplete");
        assert_eq!(boundary.decision.status, BoundaryDecisionStatus::Abstain);
    }

    #[test]
    fn completed_outputs_without_scores_abstain_for_no_scored_evidence() {
        let mut input = complete_input();
        input.capability_evidence.clear();
        input.recommendations.clear();
        let boundary = derive_run_boundary(&input);
        assert_eq!(blocking_codes(&boundary), vec!["no_scored_evidence"]);
        assert!(!boundary.boundaries[0].clearance[0].new_run_required);
        assert_eq!(
            boundary.decision.evidence_status,
            BoundaryEvidenceStatus::Partial
        );
    }

    #[test]
    fn unscored_outputs_below_sample_floor_require_a_new_run() {
        let mut input = complete_input();
        input.planned_measured_trials = 2;
        input.completed_measured_trials = 2;
        input.manifest.as_mut().unwrap().measured_trial_count = 2;
        input.trials.truncate(2);
        input.capability_evidence.clear();
        input.recommendations.clear();
        let manifest = input.manifest.as_ref().unwrap();
        input.manifest_digest = Some(crate::evaluation::sha256_hex(
            serde_json::to_string_pretty(manifest).unwrap().as_bytes(),
        ));
        let boundary = derive_run_boundary(&input);
        let blocker = &boundary.boundaries[0];
        assert_eq!(blocker.code, "no_scored_evidence");
        assert!(blocker.clearance[0].new_run_required);
        assert_eq!(blocker.evidence_refs.len(), 2);
        assert!(blocker
            .evidence_refs
            .iter()
            .all(|reference| reference.kind == EvidenceReferenceKind::Result));
    }

    #[test]
    fn aggregate_count_cannot_hide_wrong_sample_distribution() {
        let mut input = complete_input();
        for (index, trial) in input.trials.iter_mut().enumerate() {
            trial.model_id = if index % 2 == 0 { 1 } else { 2 };
            trial.category = format!("capability-{}", index / 2);
        }
        input.capability_evidence.clear();
        input.recommendations.clear();
        let boundary = derive_run_boundary(&input);
        let blocker = &boundary.boundaries[0];
        assert_eq!(blocker.code, "no_scored_evidence");
        assert!(blocker.clearance[0].new_run_required);
    }

    #[test]
    fn empty_output_is_separate_from_runtime_or_timeout_failure() {
        let mut input = complete_input();
        input.outcome_status = "completed_with_failures".into();
        input.completed_measured_trials = 5;
        input.trials[0].status = "excluded".into();
        input.trials[0].exclusion_reason = Some("empty output".into());
        input.capability_evidence.clear();
        input.recommendations.clear();
        let boundary = derive_run_boundary(&input);
        assert_eq!(
            blocking_codes(&boundary),
            vec!["run_incomplete", "no_scored_evidence", "model_failure"]
        );
    }

    #[test]
    fn mismatched_judge_methods_are_not_averaged() {
        let mut input = complete_input();
        input.capability_evidence = vec![
            capability(1, "Model A", "auto_judge:one", &[9.0, 9.0, 9.0], 1),
            capability(2, "Model B", "auto_judge:two", &[8.0, 8.0, 8.0], 4),
        ];
        input.recommendations[0].recommended_model = None;
        let boundary = derive_run_boundary(&input);
        assert_eq!(blocking_codes(&boundary), vec!["judge_method_mismatch"]);
    }

    #[test]
    fn conflicting_shared_judge_methods_are_not_mislabeled_as_interval_overlap() {
        let mut input = complete_input();
        input.capability_evidence = vec![
            capability(1, "Model A", "auto_judge:one", &[9.0, 9.0, 9.0], 1),
            capability(2, "Model B", "auto_judge:one", &[4.0, 4.0, 4.0], 4),
            capability(1, "Model A", "auto_judge:two", &[4.0, 4.0, 4.0], 7),
            capability(2, "Model B", "auto_judge:two", &[9.0, 9.0, 9.0], 10),
        ];
        input.recommendations[0].recommended_model = None;
        input.recommendations[0].confidence = "judge-sensitive".into();
        let boundary = derive_run_boundary(&input);
        assert_eq!(blocking_codes(&boundary), vec!["judge_method_mismatch"]);
    }

    #[test]
    fn directional_recommendation_cannot_override_conflicting_methods() {
        let mut input = complete_input();
        input.capability_evidence = vec![
            capability(1, "Model A", "human_score", &[9.0, 9.0, 9.0], 1),
            capability(2, "Model B", "human_score", &[4.0, 4.0, 4.0], 4),
            capability(1, "Model A", "auto_judge:local", &[4.0, 4.0, 4.0], 7),
            capability(2, "Model B", "auto_judge:local", &[9.0, 9.0, 9.0], 10),
        ];
        let boundary = derive_run_boundary(&input);
        assert_eq!(boundary.decision.status, BoundaryDecisionStatus::Abstain);
        assert_eq!(blocking_codes(&boundary), vec!["judge_method_mismatch"]);
    }

    #[test]
    fn fewer_than_three_samples_abstains() {
        let mut input = complete_input();
        input.capability_evidence = vec![
            capability(1, "Model A", "human_score", &[10.0, 10.0], 1),
            capability(2, "Model B", "human_score", &[4.0, 4.0], 3),
        ];
        input.recommendations[0].recommended_model = None;
        let boundary = derive_run_boundary(&input);
        assert_eq!(blocking_codes(&boundary), vec!["insufficient_samples"]);
        assert!(boundary
            .boundaries
            .iter()
            .any(|reason| reason.code == "small_sample_warning"));
    }

    #[test]
    fn overlapping_intervals_abstain_after_sample_gate() {
        let mut input = complete_input();
        input.capability_evidence = vec![
            capability(1, "Model A", "human_score", &[8.0, 9.0, 8.0], 1),
            capability(2, "Model B", "human_score", &[8.0, 8.0, 9.0], 4),
        ];
        input.recommendations[0].recommended_model = None;
        let boundary = derive_run_boundary(&input);
        assert_eq!(blocking_codes(&boundary), vec!["uncertainty_overlap"]);
    }

    #[test]
    fn legacy_run_fails_closed_without_inventing_identity() {
        let mut input = complete_input();
        input.manifest = None;
        input.manifest_digest = None;
        let boundary = derive_run_boundary(&input);
        assert_eq!(blocking_codes(&boundary)[0], "provenance_missing");
        assert!(boundary.scope.candidates.is_empty());
    }

    #[test]
    fn contradictory_runtime_identity_is_a_distinct_blocker() {
        let mut input = complete_input();
        input.runtime_identity_stable = Some(false);
        let boundary = derive_run_boundary(&input);
        assert_eq!(blocking_codes(&boundary), vec!["runtime_identity_unstable"]);
    }

    #[test]
    fn disagreement_and_position_bias_remain_advisory() {
        let mut input = complete_input();
        input.judge_disagreement_rate = Some(0.5);
        input.judge_disagreement_sufficient = true;
        input.position_bias_detected = true;
        input.position_bias_warning = Some("Left/right preference is large enough".into());
        input.comparison_ids = vec![40, 41];
        let boundary = derive_run_boundary(&input);
        assert_eq!(
            boundary.decision.status,
            BoundaryDecisionStatus::DirectionalChoice
        );
        assert!(boundary.boundaries.iter().any(|reason| {
            reason.code == "judge_disagreement"
                && reason.disposition == BoundaryDisposition::Advisory
        }));
        assert!(boundary.boundaries.iter().any(|reason| {
            reason.code == "position_bias_risk"
                && reason.disposition == BoundaryDisposition::Advisory
        }));
    }

    #[test]
    fn explicit_quality_and_constraint_failures_are_never_invented_or_hidden() {
        let mut input = complete_input();
        input.explicit_boundaries = vec![
            ExplicitBoundaryInput {
                kind: ExplicitBoundaryKind::Quality,
                scope: "coding".into(),
                passed: false,
                statement: "Recorded quality floor failed.".into(),
                evidence_refs: vec![manifest_reference(&input)],
                clearance: BoundaryClearance {
                    code: "meet_quality_floor".into(),
                    statement: "Meet the preregistered floor.".into(),
                    proof_required: "A passing recorded result.".into(),
                    new_run_required: true,
                },
            },
            ExplicitBoundaryInput {
                kind: ExplicitBoundaryKind::Constraint,
                scope: "latency".into(),
                passed: false,
                statement: "Recorded latency constraint failed.".into(),
                evidence_refs: vec![manifest_reference(&input)],
                clearance: BoundaryClearance {
                    code: "meet_latency_constraint".into(),
                    statement: "Meet the preregistered latency limit.".into(),
                    proof_required: "A passing observed latency receipt.".into(),
                    new_run_required: true,
                },
            },
        ];
        let boundary = derive_run_boundary(&input);
        let codes = blocking_codes(&boundary);
        assert!(codes.contains(&"quality_gate_failed"));
        assert!(codes.contains(&"constraint_failed"));
        assert_eq!(boundary.decision.status, BoundaryDecisionStatus::Abstain);
    }

    #[test]
    fn output_is_stable_and_every_blocker_has_clearance() {
        let mut input = complete_input();
        input.recommendations[0].recommended_model = None;
        input.capability_evidence[0].confidence.sufficient_sample = false;
        let first = derive_run_boundary(&input);
        let second = derive_run_boundary(&input);
        assert_eq!(first, second);
        assert!(first
            .boundaries
            .iter()
            .filter(|reason| reason.disposition == BoundaryDisposition::Blocking)
            .all(|reason| !reason.clearance.is_empty()));
    }

    #[test]
    fn serialized_boundary_is_metadata_only() {
        let mut input = complete_input();
        input.comparable = false;
        input.comparability_notes =
            Some("raw prompt from /Users/example and account customer@example.test".into());
        let boundary = derive_run_boundary(&input);
        let value = serde_json::to_value(boundary).unwrap();
        let text = serde_json::to_string(&value).unwrap();
        fn keys(value: &Value, found: &mut Vec<String>) {
            match value {
                Value::Object(object) => {
                    for (key, child) in object {
                        found.push(key.clone());
                        keys(child, found);
                    }
                }
                Value::Array(values) => {
                    for child in values {
                        keys(child, found);
                    }
                }
                _ => {}
            }
        }
        let mut serialized_keys = Vec::new();
        keys(&value, &mut serialized_keys);
        for forbidden in [
            "prompt",
            "prompt_text",
            "output",
            "raw_output",
            "judge_output",
        ] {
            assert!(!serialized_keys.iter().any(|key| key == forbidden));
        }
        assert!(!text.contains("/Users/"));
        assert!(!text.contains("customer@example.test"));
        assert_eq!(value["privacy"]["raw_prompts_included"], false);
        assert_eq!(value["privacy"]["raw_outputs_included"], false);
    }
}
