//! Module Health Analyzer
//!
//! Implements whitebox analysis for Terraform modules based on software engineering principles.
//! Detects issues related to cohesion, coupling, and module structure.
//!
//! Reference: Infrastructure code requires whitebox understanding - detailed visibility
//! into internal structure is essential, unlike application code abstraction.

use crate::terraform::model::{
    CohesionAnalysis, CohesionType, CouplingAnalysis, CouplingType, DependencyType, IssueCategory,
    IssueSeverity, ModuleBoundary, ModuleDependency, ModuleHealthAnalysis, ModuleIssue,
    ModuleMetrics, ProposedModuleStructure, RefactoringSuggestion, RefactoringType,
    ResourceDependencyGraph, ResourceEdge, ResourceNode, ResourceTypeGroup, TerraformAnalysis,
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, HashSet};

// Regex patterns for extended parsing
static DATA_SOURCE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"data\s+"([^"]+)"\s+"([^"]+)""#).expect("Invalid data source regex"));

static MODULE_CALL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"module\s+"([^"]+)"\s*\{"#).expect("Invalid module call regex"));

static LOCALS_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"locals\s*\{"#).expect("Invalid locals regex"));

static REFERENCE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(aws_[a-z_]+|azurerm_[a-z_]+|google_[a-z_]+|kubernetes_[a-z_]+)\.([a-z_0-9]+)"#)
        .expect("Invalid reference regex")
});

static DEPENDS_ON_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"depends_on\s*=\s*\[([^\]]+)\]"#).expect("Invalid depends_on regex"));

static COUNT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"count\s*=\s*"#).expect("Invalid count regex"));

static FOR_EACH_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"for_each\s*=\s*"#).expect("Invalid for_each regex"));

static MODULE_SOURCE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"source\s*=\s*"([^"]+)""#).expect("Invalid module source regex"));

// Thresholds for health analysis
const MAX_RECOMMENDED_VARIABLES: usize = 20;
const WARNING_VARIABLES: usize = 30;
const CRITICAL_VARIABLES: usize = 50;
const MAX_RESOURCE_TYPES: usize = 5;
const MAX_HIERARCHY_DEPTH: usize = 2;
const MIN_DESCRIPTION_RATIO: f64 = 0.8;

/// Resource type categories for cohesion analysis
fn get_resource_category(resource_type: &str) -> &'static str {
    let type_lower = resource_type.to_lowercase();

    // AWS categories
    if type_lower.contains("vpc")
        || type_lower.contains("subnet")
        || type_lower.contains("route")
        || type_lower.contains("internet_gateway")
        || type_lower.contains("nat_gateway")
        || type_lower.contains("network_acl")
    {
        return "networking-core";
    }
    if type_lower.contains("security_group") {
        return "networking-security";
    }
    if type_lower.contains("vpn") || type_lower.contains("transit") {
        return "networking-connectivity";
    }
    if type_lower.contains("flow_log") {
        return "networking-monitoring";
    }
    if type_lower.contains("lb")
        || type_lower.contains("load_balancer")
        || type_lower.contains("target_group")
        || type_lower.contains("listener")
    {
        return "load-balancing";
    }
    if type_lower.contains("instance")
        || type_lower.contains("launch_template")
        || type_lower.contains("autoscaling")
    {
        return "compute";
    }
    if type_lower.contains("rds")
        || type_lower.contains("db_")
        || type_lower.contains("dynamodb")
        || type_lower.contains("elasticache")
    {
        return "database";
    }
    if type_lower.contains("s3") || type_lower.contains("bucket") {
        return "storage";
    }
    if type_lower.contains("iam")
        || type_lower.contains("role")
        || type_lower.contains("policy")
        || type_lower.contains("kms")
    {
        return "security";
    }
    if type_lower.contains("lambda") || type_lower.contains("function") {
        return "serverless";
    }
    if type_lower.contains("eks")
        || type_lower.contains("ecs")
        || type_lower.contains("kubernetes")
        || type_lower.contains("container")
    {
        return "containers";
    }
    if type_lower.contains("cloudwatch")
        || type_lower.contains("log_group")
        || type_lower.contains("alarm")
        || type_lower.contains("metric")
    {
        return "monitoring";
    }
    if type_lower.contains("sns")
        || type_lower.contains("sqs")
        || type_lower.contains("eventbridge")
    {
        return "messaging";
    }
    if type_lower.contains("route53")
        || type_lower.contains("dns")
        || type_lower.contains("hosted_zone")
    {
        return "dns";
    }
    if type_lower.contains("acm") || type_lower.contains("certificate") {
        return "certificates";
    }
    if type_lower.contains("cloudfront") || type_lower.contains("cdn") {
        return "cdn";
    }
    if type_lower.contains("api_gateway") || type_lower.contains("apigateway") {
        return "api";
    }

    // Azure categories
    if type_lower.contains("azurerm_virtual_network")
        || type_lower.contains("azurerm_subnet")
        || type_lower.contains("azurerm_network")
    {
        return "networking-core";
    }
    if type_lower.contains("azurerm_vm") || type_lower.contains("azurerm_virtual_machine") {
        return "compute";
    }

    // GCP categories
    if type_lower.contains("google_compute_network")
        || type_lower.contains("google_compute_subnetwork")
    {
        return "networking-core";
    }
    if type_lower.contains("google_compute_instance") {
        return "compute";
    }

    "other"
}

/// Analyze module health
pub fn analyze_module_health(
    analysis: &TerraformAnalysis,
    file_contents: &HashMap<String, String>,
) -> ModuleHealthAnalysis {
    let metrics = calculate_metrics(analysis, file_contents);
    let cohesion = analyze_cohesion(analysis);
    let coupling = analyze_coupling(analysis, file_contents);
    let issues = detect_issues(analysis, &metrics, &cohesion, &coupling, file_contents);
    let recommendations = generate_recommendations(&issues, &metrics, &cohesion);
    let health_score = calculate_health_score(&metrics, &cohesion, &coupling, &issues);

    ModuleHealthAnalysis {
        module_path: analysis.project_directory.clone(),
        metrics,
        health_score,
        issues,
        recommendations,
        cohesion_analysis: cohesion,
        coupling_analysis: coupling,
    }
}

/// Calculate module metrics
fn calculate_metrics(
    analysis: &TerraformAnalysis,
    file_contents: &HashMap<String, String>,
) -> ModuleMetrics {
    let resource_types: HashSet<_> = analysis
        .resources
        .iter()
        .map(|r| r.resource_type.clone())
        .collect();

    let mut data_source_count = 0;
    let mut local_count = 0;
    let mut module_call_count = 0;
    let mut lines_of_code = 0;
    let mut hierarchy_depth = 0;

    for content in file_contents.values() {
        data_source_count += DATA_SOURCE_REGEX.captures_iter(content).count();
        local_count += LOCALS_REGEX.captures_iter(content).count();
        module_call_count += MODULE_CALL_REGEX.captures_iter(content).count();
        lines_of_code += content.lines().count();

        // Check for nested modules
        for cap in MODULE_SOURCE_REGEX.captures_iter(content) {
            let source = &cap[1];
            if source.starts_with("./") || source.starts_with("../") {
                let depth = source.matches('/').count();
                hierarchy_depth = hierarchy_depth.max(depth);
            }
        }
    }

    let variables_with_defaults = analysis
        .variables
        .iter()
        .filter(|v| v.default.is_some())
        .count();

    let variables_without_description = analysis
        .variables
        .iter()
        .filter(|v| {
            v.description.is_none()
                || v.description
                    .as_ref()
                    .map(|d| d.is_empty())
                    .unwrap_or(false)
        })
        .count();

    ModuleMetrics {
        variable_count: analysis.variables.len(),
        output_count: analysis.outputs.len(),
        resource_count: analysis.resources.len(),
        resource_type_count: resource_types.len(),
        provider_count: analysis.providers.len(),
        data_source_count,
        local_count,
        module_call_count,
        file_count: analysis.file_count,
        lines_of_code,
        hierarchy_depth,
        variables_with_defaults,
        variables_without_description,
    }
}

/// Analyze module cohesion
fn analyze_cohesion(analysis: &TerraformAnalysis) -> CohesionAnalysis {
    // Group resources by category
    let mut category_counts: HashMap<&str, Vec<String>> = HashMap::new();

    for resource in &analysis.resources {
        let category = get_resource_category(&resource.resource_type);
        category_counts
            .entry(category)
            .or_default()
            .push(resource.resource_type.clone());
    }

    let resource_type_groups: Vec<ResourceTypeGroup> = category_counts
        .into_iter()
        .map(|(name, types)| {
            let unique_types: HashSet<_> = types.iter().cloned().collect();
            ResourceTypeGroup {
                name: name.to_string(),
                resource_types: unique_types.into_iter().collect(),
                resource_count: types.len(),
            }
        })
        .collect();

    let num_categories = resource_type_groups.len();
    let total_resources = analysis.resources.len();

    // Determine cohesion type based on resource distribution
    let (cohesion_type, score, explanation) = if num_categories == 0 {
        (
            CohesionType::Functional,
            100,
            "Empty module or no resources analyzed".to_string(),
        )
    } else if num_categories == 1 {
        (
            CohesionType::Functional,
            95,
            format!(
                "Excellent cohesion: All {} resources belong to '{}' category",
                total_resources, resource_type_groups[0].name
            ),
        )
    } else if num_categories == 2 {
        // Check if the categories are related
        let categories: Vec<_> = resource_type_groups
            .iter()
            .map(|g| g.name.as_str())
            .collect();
        if are_categories_related(&categories) {
            (
                CohesionType::Sequential,
                85,
                format!(
                    "Good cohesion: {} categories ({}) are functionally related",
                    num_categories,
                    categories.join(", ")
                ),
            )
        } else {
            (
                CohesionType::Communicational,
                70,
                format!(
                    "Moderate cohesion: {} categories ({}) - consider if they should be separate modules",
                    num_categories,
                    categories.join(", ")
                ),
            )
        }
    } else if num_categories <= 4 {
        (
            CohesionType::Logical,
            50,
            format!(
                "Weak cohesion: {} different resource categories. Resources are grouped by type rather than function. Consider splitting into focused modules.",
                num_categories
            ),
        )
    } else {
        (
            CohesionType::Coincidental,
            25,
            format!(
                "Poor cohesion: {} different resource categories mixed together. This 'kitchen sink' module should be split.",
                num_categories
            ),
        )
    };

    CohesionAnalysis {
        cohesion_type,
        score,
        resource_type_groups,
        explanation,
    }
}

/// Check if resource categories are functionally related
fn are_categories_related(categories: &[&str]) -> bool {
    let related_pairs = [
        ("networking-core", "networking-security"),
        ("compute", "load-balancing"),
        ("database", "storage"),
        ("containers", "load-balancing"),
        ("serverless", "api"),
        ("monitoring", "logging"),
    ];

    if categories.len() != 2 {
        return false;
    }

    for (a, b) in &related_pairs {
        if (categories[0] == *a && categories[1] == *b)
            || (categories[0] == *b && categories[1] == *a)
        {
            return true;
        }
    }
    false
}

/// Analyze module coupling
fn analyze_coupling(
    analysis: &TerraformAnalysis,
    file_contents: &HashMap<String, String>,
) -> CouplingAnalysis {
    let mut dependencies = Vec::new();
    let mut control_coupling_count = 0;
    let mut module_sources: Vec<String> = Vec::new();

    for content in file_contents.values() {
        // Count control coupling (count/for_each based on variables)
        control_coupling_count += COUNT_REGEX.captures_iter(content).count();
        control_coupling_count += FOR_EACH_REGEX.captures_iter(content).count();

        // Extract module sources
        for cap in MODULE_SOURCE_REGEX.captures_iter(content) {
            module_sources.push(cap[1].to_string());
        }
    }

    // Analyze module dependencies
    for source in &module_sources {
        let dep_type = if source.starts_with("registry.terraform.io")
            || source.starts_with("terraform-")
            || source.contains("/") && !source.starts_with("./") && !source.starts_with("../")
        {
            "public-registry"
        } else if source.starts_with("./") || source.starts_with("../") {
            "local"
        } else {
            "other"
        };

        dependencies.push(ModuleDependency {
            source_module: analysis.project_directory.clone(),
            target_module: source.clone(),
            dependency_type: dep_type.to_string(),
            variables_passed: Vec::new(),
        });
    }

    // Determine coupling type
    let variable_ratio = if analysis.resources.is_empty() {
        0.0
    } else {
        analysis.variables.len() as f64 / analysis.resources.len() as f64
    };

    let (coupling_type, score, explanation) = if control_coupling_count > 10 || variable_ratio > 5.0
    {
        (
            CouplingType::Control,
            75,
            format!(
                "High control coupling: {} conditional constructs, {:.1} variables per resource. The module's behavior is heavily parameterized.",
                control_coupling_count, variable_ratio
            ),
        )
    } else if analysis.variables.len() > MAX_RECOMMENDED_VARIABLES {
        (
            CouplingType::Stamp,
            60,
            format!(
                "Moderate coupling: {} variables expose internal structure. Consider grouping related variables into objects.",
                analysis.variables.len()
            ),
        )
    } else if control_coupling_count > 5 {
        (
            CouplingType::Stamp,
            50,
            format!(
                "Moderate coupling: {} conditional constructs. Review if all conditions are necessary.",
                control_coupling_count
            ),
        )
    } else {
        (
            CouplingType::Data,
            30,
            "Low coupling: Module has clean interfaces with minimal control flow dependencies."
                .to_string(),
        )
    };

    CouplingAnalysis {
        coupling_type,
        score,
        dependencies,
        explanation,
    }
}

/// Detect issues in the module
fn detect_issues(
    _analysis: &TerraformAnalysis,
    metrics: &ModuleMetrics,
    cohesion: &CohesionAnalysis,
    coupling: &CouplingAnalysis,
    file_contents: &HashMap<String, String>,
) -> Vec<ModuleIssue> {
    let mut issues = Vec::new();

    // Check variable count
    if metrics.variable_count >= CRITICAL_VARIABLES {
        issues.push(ModuleIssue {
            severity: IssueSeverity::Critical,
            category: IssueCategory::ExcessiveVariables,
            message: format!(
                "Critical: {} variables exposed (threshold: {}). This indicates internal model exposure (モデル結合). Consider reducing interface surface.",
                metrics.variable_count, CRITICAL_VARIABLES
            ),
            file: None,
            line: None,
        });
    } else if metrics.variable_count >= WARNING_VARIABLES {
        issues.push(ModuleIssue {
            severity: IssueSeverity::Warning,
            category: IssueCategory::ExcessiveVariables,
            message: format!(
                "Warning: {} variables exposed (recommended: <{}). Review if all variables are necessary.",
                metrics.variable_count, MAX_RECOMMENDED_VARIABLES
            ),
            file: None,
            line: None,
        });
    }

    // Check resource type diversity (logical cohesion)
    if metrics.resource_type_count > MAX_RESOURCE_TYPES {
        issues.push(ModuleIssue {
            severity: IssueSeverity::Warning,
            category: IssueCategory::LogicalCohesion,
            message: format!(
                "Logical cohesion detected: {} different resource types in one module. This 'まとめすぎ' pattern reduces maintainability. Consider splitting by function.",
                metrics.resource_type_count
            ),
            file: None,
            line: None,
        });
    }

    // Check hierarchy depth
    if metrics.hierarchy_depth > MAX_HIERARCHY_DEPTH {
        issues.push(ModuleIssue {
            severity: IssueSeverity::Warning,
            category: IssueCategory::DeepHierarchy,
            message: format!(
                "Deep module hierarchy: {} levels (recommended: ≤{}). Deep nesting reduces visibility and makes debugging harder (多段構成).",
                metrics.hierarchy_depth, MAX_HIERARCHY_DEPTH
            ),
            file: None,
            line: None,
        });
    }

    // Check documentation
    let description_ratio = if metrics.variable_count > 0 {
        (metrics.variable_count - metrics.variables_without_description) as f64
            / metrics.variable_count as f64
    } else {
        1.0
    };

    if description_ratio < MIN_DESCRIPTION_RATIO {
        issues.push(ModuleIssue {
            severity: IssueSeverity::Info,
            category: IssueCategory::MissingDocumentation,
            message: format!(
                "{} of {} variables lack descriptions ({:.0}% documented). Documentation is essential for whitebox usage.",
                metrics.variables_without_description,
                metrics.variable_count,
                description_ratio * 100.0
            ),
            file: None,
            line: None,
        });
    }

    // Check for public module usage
    for dep in &coupling.dependencies {
        if dep.dependency_type == "public-registry" {
            issues.push(ModuleIssue {
                severity: IssueSeverity::Warning,
                category: IssueCategory::PublicModuleRisk,
                message: format!(
                    "Public module detected: '{}'. Public modules often have excessive variables and logical cohesion. Consider creating an organization-specific wrapper.",
                    dep.target_module
                ),
                file: None,
                line: None,
            });
        }
    }

    // Check for control coupling patterns
    for (filename, content) in file_contents {
        let count_occurrences = COUNT_REGEX.captures_iter(content).count();
        let for_each_occurrences = FOR_EACH_REGEX.captures_iter(content).count();

        if count_occurrences + for_each_occurrences > 5 {
            issues.push(ModuleIssue {
                severity: IssueSeverity::Info,
                category: IssueCategory::ControlCoupling,
                message: format!(
                    "High conditional complexity in '{}': {} count/for_each patterns. This may indicate control coupling (制御結合).",
                    filename,
                    count_occurrences + for_each_occurrences
                ),
                file: Some(filename.clone()),
                line: None,
            });
        }
    }

    // Check naming conventions
    for filename in file_contents.keys() {
        if filename == "main.tf" && metrics.resource_count > 5 {
            issues.push(ModuleIssue {
                severity: IssueSeverity::Info,
                category: IssueCategory::NamingConvention,
                message: "Consider renaming 'main.tf' to reflect its actual purpose (e.g., 'vpc.tf', 'compute.tf'). 'main.tf' doesn't convey what resources it contains.".to_string(),
                file: Some(filename.clone()),
                line: None,
            });
        }
    }

    // Cohesion-based issues
    if cohesion.score < 50 {
        issues.push(ModuleIssue {
            severity: IssueSeverity::Warning,
            category: IssueCategory::LogicalCohesion,
            message: cohesion.explanation.clone(),
            file: None,
            line: None,
        });
    }

    issues
}

/// Generate recommendations based on detected issues
fn generate_recommendations(
    issues: &[ModuleIssue],
    metrics: &ModuleMetrics,
    cohesion: &CohesionAnalysis,
) -> Vec<String> {
    let mut recommendations = Vec::new();

    // Variable reduction recommendations
    if metrics.variable_count > MAX_RECOMMENDED_VARIABLES {
        recommendations.push(format!(
            "🔧 Reduce variable exposure: Group related variables into objects, use locals for derived values, and set sensible defaults. Target: ≤{} variables.",
            MAX_RECOMMENDED_VARIABLES
        ));
    }

    // Cohesion recommendations
    if cohesion.resource_type_groups.len() > 3 {
        let groups: Vec<_> = cohesion
            .resource_type_groups
            .iter()
            .map(|g| g.name.as_str())
            .collect();
        recommendations.push(format!(
            "🔧 Split module by function: Current categories ({}). Create separate modules for each distinct function.",
            groups.join(", ")
        ));
    }

    // Documentation recommendations
    if metrics.variables_without_description > 0 {
        recommendations.push(format!(
            "📝 Add descriptions to {} variables. Use terraform-docs to generate documentation automatically.",
            metrics.variables_without_description
        ));
    }

    // Hierarchy recommendations
    if metrics.hierarchy_depth > MAX_HIERARCHY_DEPTH {
        recommendations.push(
            "🏗️ Flatten module hierarchy: Prefer composition over deep nesting. Consider using module composition patterns instead of deep hierarchies.".to_string()
        );
    }

    // Public module recommendations
    let public_module_issues = issues
        .iter()
        .filter(|i| matches!(i.category, IssueCategory::PublicModuleRisk))
        .count();
    if public_module_issues > 0 {
        recommendations.push(
            "⚠️ Create organization wrappers for public modules: Public modules expose too many options. Create thin wrappers that expose only the options your organization needs.".to_string()
        );
    }

    // General best practices
    if issues.is_empty() {
        recommendations.push(
            "✅ Module structure looks healthy! Continue following current patterns.".to_string(),
        );
    }

    recommendations
}

/// Calculate overall health score
fn calculate_health_score(
    metrics: &ModuleMetrics,
    cohesion: &CohesionAnalysis,
    coupling: &CouplingAnalysis,
    issues: &[ModuleIssue],
) -> u8 {
    let mut score: i32 = 100;

    // Deduct for variable count
    if metrics.variable_count > CRITICAL_VARIABLES {
        score -= 30;
    } else if metrics.variable_count > WARNING_VARIABLES {
        score -= 15;
    } else if metrics.variable_count > MAX_RECOMMENDED_VARIABLES {
        score -= 5;
    }

    // Deduct for cohesion issues
    score -= ((100 - cohesion.score as i32) / 3).min(25);

    // Deduct for coupling issues
    score -= (coupling.score as i32 / 4).min(20);

    // Deduct for issues
    for issue in issues {
        match issue.severity {
            IssueSeverity::Critical => score -= 15,
            IssueSeverity::Warning => score -= 5,
            IssueSeverity::Info => score -= 1,
        }
    }

    // Deduct for hierarchy depth
    if metrics.hierarchy_depth > MAX_HIERARCHY_DEPTH {
        score -= 10;
    }

    // Ensure score is within bounds
    score.clamp(0, 100) as u8
}

/// Build resource dependency graph
pub fn build_dependency_graph(
    analysis: &TerraformAnalysis,
    file_contents: &HashMap<String, String>,
) -> ResourceDependencyGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // Build resource lookup map
    let mut resource_map: HashMap<String, usize> = HashMap::new();

    // Create nodes for each resource
    for resource in &analysis.resources {
        let id = format!("{}.{}", resource.resource_type, resource.name);
        resource_map.insert(id.clone(), nodes.len());

        nodes.push(ResourceNode {
            id: id.clone(),
            resource_type: resource.resource_type.clone(),
            resource_name: resource.name.clone(),
            module_path: analysis.project_directory.clone(),
            file: resource.file.clone(),
            provider: resource.provider.clone(),
        });
    }

    // Find dependencies by scanning file contents
    for (filename, content) in file_contents {
        // Find explicit depends_on
        for cap in DEPENDS_ON_REGEX.captures_iter(content) {
            let deps_str = &cap[1];
            for dep in deps_str.split(',') {
                let dep_trimmed = dep.trim().trim_matches(|c| c == '[' || c == ']');
                if resource_map.contains_key(dep_trimmed) {
                    // Find which resource this depends_on belongs to
                    // This is simplified - in practice we'd need more context
                    for node in &nodes {
                        if content.contains(&format!("resource \"{}\"", node.resource_type)) {
                            edges.push(ResourceEdge {
                                source: node.id.clone(),
                                target: dep_trimmed.to_string(),
                                dependency_type: DependencyType::Explicit,
                                attribute: Some("depends_on".to_string()),
                            });
                            break;
                        }
                    }
                }
            }
        }

        // Find implicit references
        for cap in REFERENCE_REGEX.captures_iter(content) {
            let ref_type = &cap[1];
            let ref_name = &cap[2];
            let ref_id = format!("{}.{}", ref_type, ref_name);

            if resource_map.contains_key(&ref_id) {
                // Find the resource that contains this reference
                for resource in &analysis.resources {
                    if resource.file == *filename {
                        let source_id = format!("{}.{}", resource.resource_type, resource.name);
                        if source_id != ref_id {
                            // Avoid duplicate edges
                            let edge_exists = edges.iter().any(|e| {
                                e.source == source_id
                                    && e.target == ref_id
                                    && matches!(e.dependency_type, DependencyType::Implicit)
                            });

                            if !edge_exists {
                                edges.push(ResourceEdge {
                                    source: source_id,
                                    target: ref_id.clone(),
                                    dependency_type: DependencyType::Implicit,
                                    attribute: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Create module boundary
    let module_boundaries = vec![ModuleBoundary {
        module_path: analysis.project_directory.clone(),
        resource_ids: nodes.iter().map(|n| n.id.clone()).collect(),
    }];

    ResourceDependencyGraph {
        nodes,
        edges,
        module_boundaries,
    }
}

/// Generate refactoring suggestions
pub fn suggest_refactoring(
    analysis: &TerraformAnalysis,
    health: &ModuleHealthAnalysis,
) -> Vec<RefactoringSuggestion> {
    let mut suggestions = Vec::new();

    // Suggest splitting if too many resource types
    if health.cohesion_analysis.resource_type_groups.len() > 2 {
        for group in &health.cohesion_analysis.resource_type_groups {
            if group.resource_count >= 2 {
                let affected_resources: Vec<String> = analysis
                    .resources
                    .iter()
                    .filter(|r| group.resource_types.contains(&r.resource_type))
                    .map(|r| format!("{}.{}", r.resource_type, r.name))
                    .collect();

                if !affected_resources.is_empty() {
                    suggestions.push(RefactoringSuggestion {
                        suggestion_type: RefactoringType::ExtractSubmodule,
                        priority: IssueSeverity::Warning,
                        description: format!(
                            "Extract '{}' resources into a dedicated module",
                            group.name
                        ),
                        affected_resources: affected_resources.clone(),
                        proposed_structure: Some(ProposedModuleStructure {
                            module_name: format!("modules/{}", group.name.replace('-', "_")),
                            resources: affected_resources,
                            variables: Vec::new(),
                            outputs: Vec::new(),
                        }),
                        migration_steps: vec![
                            format!("1. Create new module directory: modules/{}", group.name),
                            "2. Move related resources to new module".to_string(),
                            "3. Create variables.tf for required inputs".to_string(),
                            "4. Create outputs.tf for values needed by other resources".to_string(),
                            "5. Add 'moved' blocks to preserve state".to_string(),
                            "6. Run terraform plan to verify no changes".to_string(),
                        ],
                    });
                }
            }
        }
    }

    // Suggest wrapping public modules
    for dep in &health.coupling_analysis.dependencies {
        if dep.dependency_type == "public-registry" {
            suggestions.push(RefactoringSuggestion {
                suggestion_type: RefactoringType::WrapPublicModule,
                priority: IssueSeverity::Warning,
                description: format!(
                    "Create organization wrapper for public module: {}",
                    dep.target_module
                ),
                affected_resources: vec![dep.target_module.clone()],
                proposed_structure: Some(ProposedModuleStructure {
                    module_name: format!(
                        "modules/{}",
                        dep.target_module.split('/').last().unwrap_or("wrapper")
                    ),
                    resources: Vec::new(),
                    variables: vec!["# Expose only necessary variables".to_string()],
                    outputs: vec!["# Forward only needed outputs".to_string()],
                }),
                migration_steps: vec![
                    "1. Create wrapper module directory".to_string(),
                    "2. Define minimal variable interface".to_string(),
                    "3. Call public module with organization defaults".to_string(),
                    "4. Forward only necessary outputs".to_string(),
                    "5. Update callers to use wrapper module".to_string(),
                ],
            });
        }
    }

    // Suggest adding descriptions
    if health.metrics.variables_without_description > 0 {
        let vars_needing_desc: Vec<String> = analysis
            .variables
            .iter()
            .filter(|v| {
                v.description.is_none()
                    || v.description
                        .as_ref()
                        .map(|d| d.is_empty())
                        .unwrap_or(false)
            })
            .map(|v| v.name.clone())
            .collect();

        suggestions.push(RefactoringSuggestion {
            suggestion_type: RefactoringType::AddDescriptions,
            priority: IssueSeverity::Info,
            description: format!(
                "Add descriptions to {} undocumented variables",
                vars_needing_desc.len()
            ),
            affected_resources: vars_needing_desc,
            proposed_structure: None,
            migration_steps: vec![
                "1. Review each variable's purpose".to_string(),
                "2. Add description field with clear explanation".to_string(),
                "3. Include example values where helpful".to_string(),
                "4. Run terraform-docs to generate documentation".to_string(),
            ],
        });
    }

    // Suggest flattening hierarchy
    if health.metrics.hierarchy_depth > MAX_HIERARCHY_DEPTH {
        suggestions.push(RefactoringSuggestion {
            suggestion_type: RefactoringType::FlattenHierarchy,
            priority: IssueSeverity::Warning,
            description: format!(
                "Reduce module hierarchy from {} levels to ≤{}",
                health.metrics.hierarchy_depth, MAX_HIERARCHY_DEPTH
            ),
            affected_resources: Vec::new(),
            proposed_structure: None,
            migration_steps: vec![
                "1. Identify deeply nested modules".to_string(),
                "2. Consider inlining small modules".to_string(),
                "3. Use module composition instead of nesting".to_string(),
                "4. Maintain visibility of resource details".to_string(),
            ],
        });
    }

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terraform::model::{TerraformOutput, TerraformProvider, TerraformVariable};

    fn create_test_analysis() -> TerraformAnalysis {
        TerraformAnalysis {
            project_directory: "/test/module".to_string(),
            file_count: 3,
            resources: vec![
                crate::terraform::model::TerraformResource {
                    resource_type: "aws_vpc".to_string(),
                    name: "main".to_string(),
                    file: "vpc.tf".to_string(),
                    provider: "aws".to_string(),
                },
                crate::terraform::model::TerraformResource {
                    resource_type: "aws_subnet".to_string(),
                    name: "public".to_string(),
                    file: "vpc.tf".to_string(),
                    provider: "aws".to_string(),
                },
                crate::terraform::model::TerraformResource {
                    resource_type: "aws_instance".to_string(),
                    name: "web".to_string(),
                    file: "compute.tf".to_string(),
                    provider: "aws".to_string(),
                },
            ],
            variables: vec![
                TerraformVariable {
                    name: "vpc_cidr".to_string(),
                    description: Some("VPC CIDR block".to_string()),
                    type_: Some("string".to_string()),
                    default: None,
                },
                TerraformVariable {
                    name: "instance_type".to_string(),
                    description: None,
                    type_: Some("string".to_string()),
                    default: Some(serde_json::json!("t3.micro")),
                },
            ],
            outputs: vec![TerraformOutput {
                name: "vpc_id".to_string(),
                description: Some("The VPC ID".to_string()),
                value: None,
            }],
            providers: vec![TerraformProvider {
                name: "aws".to_string(),
                version: Some("~> 5.0".to_string()),
            }],
        }
    }

    #[test]
    fn test_resource_category() {
        assert_eq!(get_resource_category("aws_vpc"), "networking-core");
        assert_eq!(get_resource_category("aws_subnet"), "networking-core");
        assert_eq!(get_resource_category("aws_instance"), "compute");
        assert_eq!(get_resource_category("aws_s3_bucket"), "storage");
        assert_eq!(
            get_resource_category("aws_security_group"),
            "networking-security"
        );
        assert_eq!(
            get_resource_category("aws_vpn_gateway"),
            "networking-connectivity"
        );
    }

    #[test]
    fn test_calculate_metrics() {
        let analysis = create_test_analysis();
        let mut file_contents = HashMap::new();
        file_contents.insert(
            "vpc.tf".to_string(),
            r#"
            resource "aws_vpc" "main" {}
            resource "aws_subnet" "public" {}
            "#
            .to_string(),
        );

        let metrics = calculate_metrics(&analysis, &file_contents);
        assert_eq!(metrics.variable_count, 2);
        assert_eq!(metrics.resource_count, 3);
        assert_eq!(metrics.output_count, 1);
    }

    #[test]
    fn test_analyze_cohesion() {
        let analysis = create_test_analysis();
        let cohesion = analyze_cohesion(&analysis);

        // Should have moderate cohesion (networking + compute)
        assert!(cohesion.score > 0);
        assert!(!cohesion.resource_type_groups.is_empty());
    }

    #[test]
    fn test_health_score_bounds() {
        let analysis = create_test_analysis();
        let file_contents = HashMap::new();
        let health = analyze_module_health(&analysis, &file_contents);

        assert!(health.health_score <= 100);
    }
}
