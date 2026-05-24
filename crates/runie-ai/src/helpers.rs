//! Conversion helpers between runie and rig-core types.
//!
//! This module provides utility functions for converting between runie's
//! type system and rig-core's type system. These conversions are necessary
//! when integrating runie tools and schemas with rig-core's completion and
//! agent infrastructure.
//!
//! ## Tool Schema Conversion
//!
//! Runie uses [`ToolSchema`] to describe tools, while rig-core uses
//! `rig_core::completion::ToolDefinition`. These types have compatible
//! structures but different field names and ownership semantics.
//!
//! ### Example
//!
//! ```ignore
//! use runie_core::ToolRegistry;
//! use runie_ai::helpers::tool_schemas_to_rig_definitions;
//!
//! let registry = ToolRegistry::new();
//! // ... register some tools ...
//!
//! let schemas: Vec<_> = registry.schemas().collect();
//! let rig_defs = tool_schemas_to_rig_definitions(&schemas);
//! // Use rig_defs with rig-core's CompletionModel
//! ```
//!
//! ## See Also
//!
//! - [`runie_tools::rig_adapter::RunieToolAdapter`] for adapting runie Tool
//!   implementations to rig-core's ToolDyn trait

use runie_core::ToolSchema;

/// Converts a runie ToolSchema to a rig-core ToolDefinition.
///
/// This function creates a rig-core `ToolDefinition` with the same name,
/// description, and parameters as the provided runie `ToolSchema`.
/// The conversion is a simple field-by-field copy since both types
/// use compatible representations for tool metadata.
pub fn tool_schema_to_rig_definition(schema: &ToolSchema) -> rig_core::completion::ToolDefinition {
    rig_core::completion::ToolDefinition {
        name: schema.name.clone(),
        description: schema.description.clone(),
        parameters: schema.parameters.clone(),
    }
}

/// Converts a slice of runie ToolSchemas to a Vec of rig-core ToolDefinitions.
///
/// This is a convenience function that applies [`tool_schema_to_rig_definition`]
/// to each schema in the input slice, collecting the results into a Vec.
pub fn tool_schemas_to_rig_definitions(
    schemas: &[ToolSchema],
) -> Vec<rig_core::completion::ToolDefinition> {
    schemas.iter().map(tool_schema_to_rig_definition).collect()
}
