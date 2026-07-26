//! Refactoring suggestions derived from module health analysis.

use crate::terraform::analyzer::health::MAX_HIERARCHY_DEPTH;
use crate::terraform::model::core::TerraformAnalysis;
use crate::terraform::model::health::{IssueSeverity, ModuleHealthAnalysis};
use crate::terraform::model::refactoring::{
    ProposedModuleStructure, RefactoringSuggestion, RefactoringType,
};

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
                        dep.target_module
                            .split('/')
                            .next_back()
                            .unwrap_or("wrapper")
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
