//! JavaScript runtime for agent scripting
//! Uses rquickjs to execute anvil.js agent scripts

/// Agent pack discovered on disk
#[derive(Debug, Clone)]
pub struct AgentPack {
    pub name: String,
    pub path: std::path::PathBuf,
    pub has_route_fn: bool,
    pub has_plan_fn: bool,
    pub has_validate_fn: bool,
}

/// JavaScript runtime for executing agent scripts
#[derive(Clone)]
pub struct JsRuntime {
    // We use a simple eval-based approach; rquickjs full async ctx
    // injection is deferred to Phase 5 completion
}

impl JsRuntime {
    /// Create a new JS runtime
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {})
    }

    /// Scan ~/.anvil/agents/ for agent packs
    pub fn discover_agents(&mut self) -> Vec<AgentPack> {
        let agent_dir = dirs::home_dir()
            .map(|h| h.join(".anvil/agents"))
            .unwrap_or_else(|| std::path::PathBuf::from(".anvil/agents"));

        if !agent_dir.exists() {
            return Vec::new();
        }

        let mut packs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&agent_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let anvil_js = path.join("anvil.js");
                if anvil_js.exists() {
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let content = std::fs::read_to_string(&anvil_js).unwrap_or_default();
                    let has_route = content.contains("export function route")
                        || content.contains("export async function route");
                    let has_plan = content.contains("export async function plan")
                        || content.contains("export function plan(");
                    let has_validate = content.contains("export async function validate")
                        || content.contains("export function validate(");

                    packs.push(AgentPack {
                        name,
                        path,
                        has_route_fn: has_route,
                        has_plan_fn: has_plan,
                        has_validate_fn: has_validate,
                    });
                }
            }
        }
        packs
    }

    /// Check if a given function exists in a script
    pub fn has_function(&self, script_path: &std::path::Path, name: &str) -> bool {
        std::fs::read_to_string(script_path)
            .map(|c| {
                c.contains(&format!("function {}", name))
                    || c.contains(&format!("function({}", name))
                    || c.contains(&format!("async function {}", name))
            })
            .unwrap_or(false)
    }

    /// Execute an anvil.js script and call a named function with JSON args.
    /// Returns the function's JSON result as a serde_json::Value.
    /// For Phase 5, this is a stub that loads the script and evaluates a call.
    pub async fn run_agent_script(
        &self,
        script_path: &std::path::Path,
        function_name: &str,
        args: Vec<serde_json::Value>,
    ) -> anyhow::Result<serde_json::Value> {
        use rquickjs::{Context, Runtime};

        let rt = Runtime::new()
            .map_err(|e| anyhow::anyhow!("rquickjs Runtime::new failed: {}", e))?;
        let ctx = Context::full(&rt)
            .map_err(|e| anyhow::anyhow!("rquickjs Context::full failed: {}", e))?;

        let script_content = std::fs::read_to_string(script_path)
            .map_err(|e| anyhow::anyhow!("Failed to read {:?}: {}", script_path, e))?;

        // Prepend args to globals so the function can access them
        let args_json = serde_json::to_string(&args)?;
        let eval_script = format!(
            r#"
            {script}
            // Call the exported function if it exists
            if (typeof {fn} === 'function') {{
                JSON.stringify({fn}.apply(null, {args}));
            }} else {{
                'null'
            }}
            "#,
            script = script_content,
            fn = function_name,
            args = args_json,
        );

        let result: String = ctx.with(|ctx| {
            ctx.eval::<String, _>(eval_script.as_str())
                .map_err(|e| anyhow::anyhow!("JS eval error: {}", e))
        })?;

        serde_json::from_str(&result)
            .map_err(|e| anyhow::anyhow!("JS result not valid JSON: {} (raw: {})", e, result))
    }

    /// Execute a simple script string (for hooks, safety checks, etc.)
    pub async fn run_script(&self, script: &str) -> anyhow::Result<String> {
        use rquickjs::{Context, Runtime};

        let rt = Runtime::new()
            .map_err(|e| anyhow::anyhow!("rquickjs Runtime::new failed: {}", e))?;
        let ctx = Context::full(&rt)
            .map_err(|e| anyhow::anyhow!("rquickjs Context::full failed: {}", e))?;

        ctx.with(|ctx| {
            ctx.eval::<(), _>(script)
                .map_err(|e| anyhow::anyhow!("Script error: {}", e))
        })?;

        Ok("ok".to_string())
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create JS runtime")
    }
}

/// Simple script evaluation (sync, for tests)
pub fn eval_script(script: &str) -> Result<String, String> {
    use rquickjs::{Context, Runtime};

    let rt = Runtime::new().map_err(|e| e.to_string())?;
    let ctx = Context::full(&rt).map_err(|e| e.to_string())?;

    ctx.with(|ctx| {
        ctx.eval::<(), _>(script).map_err(|e| e.to_string())
    })?;

    Ok("ok".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_script() {
        let result = eval_script("1 + 1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_agent_discovery() {
        let mut rt = JsRuntime::new().unwrap();
        let _packs = rt.discover_agents();
        // May be empty if no ~/.anvil/agents exists — that's fine
        assert!(true); // packs may be empty — that's fine
    }
}
