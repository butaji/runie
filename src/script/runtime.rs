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

    /// Run an anvil.js hook with the full ctx API injected.
    /// Builds the ctx object as plain JS (stubs) and evaluates the user script in that scope.
    /// Available to scripts: task, session, git, run, ui, router, safety, human.
    pub async fn run_with_ctx(
        &self,
        script: &str,
        ctx_api: &crate::script::api::AnvilContext,
    ) -> anyhow::Result<serde_json::Value> {
        use rquickjs::{Context, Runtime};

        let rt = Runtime::new()
            .map_err(|e| anyhow::anyhow!("rquickjs Runtime::new failed: {}", e))?;
        let ctx = Context::full(&rt)
            .map_err(|e| anyhow::anyhow!("rquickjs Context::full failed: {}", e))?;

        // Serialize ctx parts to JSON for JS
        let task_json = serde_json::to_string(&serde_json::json!({
            "id": ctx_api.task.id,
            "type": ctx_api.task.task_type,
            "intent": ctx_api.task.intent,
            "estimatedTokens": ctx_api.task.estimated_tokens,
            "severity": ctx_api.task.severity,
        }))?;

        let session_json = serde_json::to_string(&serde_json::json!({
            "cost": ctx_api.session.cost,
            "budget": ctx_api.session.budget,
            "elapsed": ctx_api.session.elapsed,
        }))?;

                // Build JS ctx object + safety helper
        // Embeds ctx as JSON strings parsed in JS to avoid Rust format conflicts
        // Build JS ctx string with format args. We escape ALL { and } as {{ and }}
        // so the format! macro treats them as literal characters. The three format
        // arguments (task, session, user_script) use single braces.
        let js_raw = r#"
(function() {
    var __task = JSON.parse("{task}");
    var __session = JSON.parse("{session}");
    function __checkPath(path) {
        var protected = [".env", "secrets", ".ssh", "key.pem", ".pem"];
        for (var i = 0; i < protected.length; i++) {
            if (path.indexOf(protected[i]) !== -1) return false;
        }
        return true;
    }
    var ctx = {
        task: __task,
        session: __session,
        git: {
            commit: function(msg) { return "Would commit: " + msg; },
            changedFiles: function() { return []; },
            worktree: null,
        },
        run: function(cmd) { return { "exitCode": 0, "stdout": "Would run: " + cmd, "stderr": "" }; },
        ui: {
            showError: function(title, detail) { console.error("[ERROR] " + title + ": " + detail); },
            showInfo: function(msg) { console.log("[INFO] " + msg); },
        },
        safety: {
            checkPath: __checkPath,
            requireApproval: function(reason) { return true; },
            pause: function(reason) {},
        },
        router: {
            estimateCost: function(tokens) { return (tokens / 1000000.0) * 3.0; },
            downgradeModel: function(task) {},
            models: {},
        },
        human: {
            confirm: function(opts) { return true; },
            select: function(opts) { return null; },
            input: function(prompt) { return ""; },
        },
    };
    {user_script}
})();
"#;
        let js = js_raw
            .replace("{task}", &task_json)
            .replace("{session}", &session_json)
            .replace("{user_script}", script);

        ctx.with(|ctx| {
            ctx.eval::<(), _>(js.as_str())
                .map_err(|e| anyhow::anyhow!("Script error: {}", e))
        })?;
        Ok(serde_json::json!({ "ok": true }))
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
    }
}
