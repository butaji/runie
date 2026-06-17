use runie_core::orchestrator::OrchestratorPlan;
use runie_core::trait_resolver::ModelResolver;

use crate::planner::error::PlannerError;

/// Validate a plan against the model resolver.
pub fn validate_plan(
    plan: &OrchestratorPlan,
    resolver: &ModelResolver,
) -> Result<(), PlannerError> {
    // Check synthesis trait
    resolver
        .resolve(plan.synthesis_trait)
        .map_err(|_| PlannerError::NoModelForTrait {
            trait_: plan.synthesis_trait,
        })?;

    // Check each task's model trait
    let mut seen = std::collections::HashSet::new();
    for task in &plan.tasks {
        if !seen.insert(task.id.clone()) {
            return Err(PlannerError::ValidationFailed(format!(
                "duplicate task id: '{}'",
                task.id
            )));
        }

        resolver
            .resolve(task.model_trait)
            .map_err(|_| PlannerError::NoModelForTrait {
                trait_: task.model_trait,
            })?;
    }

    Ok(())
}
