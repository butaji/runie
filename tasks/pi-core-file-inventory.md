# Pi core file inventory

The executable source of truth is `scripts/source-inventory.sh`; the sections
below are a review snapshot and must not override its output. The current
source trees contain 48 agent files and 174 AI files. Every file is listed so
parity claims can be traced to a concrete source file. Provider catalogs,
OAuth, image, session, and harness files are inventory inputs and remain
out of Runie core scope unless promoted by p30.

## Inventory delta (2026-08-06)

The deterministic scan found 23 paths added or renamed upstream since this
snapshot and 7 paths no longer present. The complete delta is kept here until
this long-form review snapshot is regenerated:

- Added/current: `agent/harness/{reducer,result,session/context,session/index,session/jsonl,session/jsonl/codec,session/jsonl/errors,session/jsonl/repo,session/jsonl/storage,session/jsonl/types,session/memory,session/search,session/state,session/testing/conformance,session/testing/index,session/testing/types,session/types,telemetry}.ts`.
- Added/current: `ai/providers/{baseten,baseten.models,qwen-token-plan-individual,qwen-token-plan-individual.models}.ts` and `ai/utils/abort.ts`.
- Removed/renamed from this snapshot: `agent/harness/session/{array-session-reader,fork,jsonl-store,keyed-operation-queue,memory-store,repository,search-backend}.ts`.

## packages/agent/src (48 files; snapshot pending refresh)

- `/Users/admin/Code/agents/pi/packages/agent/src/agent-loop.ts` — agent lifecycle
- `/Users/admin/Code/agents/pi/packages/agent/src/agent.ts` — agent lifecycle
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/agent-harness.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/compaction/branch-summarization.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/compaction/compaction.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/compaction/utils.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/env/nodejs.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/messages.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/prompt-templates.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/session/array-session-reader.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/session/fork.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/session/jsonl-store.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/session/keyed-operation-queue.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/session/memory-store.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/session/repository.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/session/search-backend.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/session/session.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/skills.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/system-prompt.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/tools/bash.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/tools/edit-diff.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/tools/edit.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/tools/file-mutation-queue.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/tools/image.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/tools/index.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/tools/path-utils.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/tools/read.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/tools/tool-context.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/tools/write.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/types.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/utils/shell-output.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/harness/utils/truncate.ts` — harness boundary
- `/Users/admin/Code/agents/pi/packages/agent/src/index.ts` — agent-core support
- `/Users/admin/Code/agents/pi/packages/agent/src/node.ts` — agent-core support
- `/Users/admin/Code/agents/pi/packages/agent/src/proxy.ts` — agent-core support
- `/Users/admin/Code/agents/pi/packages/agent/src/stream-fn.ts` — agent-core support
- `/Users/admin/Code/agents/pi/packages/agent/src/types.ts` — wire/state types

## packages/ai/src (174 files; snapshot pending refresh)

- `/Users/admin/Code/agents/pi/packages/ai/src/api/anthropic-messages.lazy.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/anthropic-messages.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/azure-openai-responses.lazy.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/azure-openai-responses.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/bedrock-converse-stream.lazy.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/bedrock-converse-stream.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/cloudflare.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/constrained-sampling.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/github-copilot-headers.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/google-generative-ai.lazy.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/google-generative-ai.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/google-shared.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/google-vertex.lazy.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/google-vertex.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/lazy.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/mistral-conversations.lazy.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/mistral-conversations.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/openai-codex-responses.lazy.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/openai-codex-responses.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/openai-completions.lazy.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/openai-completions.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/openai-prompt-cache.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/openai-responses-shared.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/openai-responses.lazy.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/openai-responses.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/openrouter-images.lazy.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/openrouter-images.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/pi-messages.lazy.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/pi-messages.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/simple-options.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/api/transform-messages.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/context.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/credential-store.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/helpers.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/oauth/anthropic.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/oauth/device-code.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/oauth/github-copilot.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/oauth/kimi-coding.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/oauth/load.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/oauth/oauth-page.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/oauth/openai-codex.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/oauth/openrouter.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/oauth/pkce.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/oauth/radius.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/oauth/xai.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/resolve.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/auth/types.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/bedrock-provider.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/bun-oauth.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/cli.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/compat.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/compat/extension-oauth-types.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/env-api-keys.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/image-models.generated.ts` — image boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/image-models.ts` — image boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/images-api-registry.ts` — image boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/images-models.ts` — image boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/images.ts` — image boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/index.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/legacy-api-aliases.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/model-catalog.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/models-store.ts` — model/usage contract
- `/Users/admin/Code/agents/pi/packages/ai/src/models.generated.ts` — model/usage contract
- `/Users/admin/Code/agents/pi/packages/ai/src/models.ts` — model/usage contract
- `/Users/admin/Code/agents/pi/packages/ai/src/oauth.ts` — auth boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/all.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/amazon-bedrock.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/amazon-bedrock.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/ant-ling.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/ant-ling.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/anthropic.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/anthropic.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/azure-openai-responses.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/azure-openai-responses.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/cerebras.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/cerebras.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/cloudflare-ai-gateway.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/cloudflare-ai-gateway.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/cloudflare-auth.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/cloudflare-stream.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/cloudflare-workers-ai.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/cloudflare-workers-ai.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/data-json.d.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/deepseek.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/deepseek.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/faux.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/fireworks.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/fireworks.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/github-copilot.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/github-copilot.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/google-vertex.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/google-vertex.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/google.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/google.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/groq.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/groq.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/huggingface.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/huggingface.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/images/register-builtins.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/kimi-coding.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/kimi-coding.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/minimax-cn.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/minimax-cn.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/minimax.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/minimax.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/mistral.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/mistral.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/moonshotai-cn.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/moonshotai-cn.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/moonshotai.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/moonshotai.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/nvidia.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/nvidia.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/openai-codex.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/openai-codex.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/openai.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/openai.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/opencode-go.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/opencode-go.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/opencode.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/opencode.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/openrouter-images.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/openrouter.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/openrouter.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/qwen-token-plan-cn.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/qwen-token-plan-cn.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/qwen-token-plan.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/qwen-token-plan.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/radius-config.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/radius.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/together.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/together.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/vercel-ai-gateway.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/vercel-ai-gateway.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/xai.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/xai.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/xiaomi-token-plan-ams.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/xiaomi-token-plan-ams.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/xiaomi-token-plan-cn.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/xiaomi-token-plan-cn.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/xiaomi-token-plan-sgp.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/xiaomi-token-plan-sgp.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/xiaomi.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/xiaomi.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/zai-coding-cn.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/zai-coding-cn.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/zai.models.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/providers/zai.ts` — provider/API boundary
- `/Users/admin/Code/agents/pi/packages/ai/src/session-resources.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/types.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/abort-signals.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/deferred-tools.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/diagnostics.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/error-body.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/estimate.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/event-stream.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/hash.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/headers.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/json-parse.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/node-http-proxy.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/overflow.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/provider-env.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/provider-retry.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/retry.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/sanitize-unicode.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/text.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/typebox-helpers.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/uuid.ts` — AI core support
- `/Users/admin/Code/agents/pi/packages/ai/src/utils/validation.ts` — AI core support

## Review rule

Each implementation change must cite the relevant inventory path and either
add a Runie mapping plus replay fixture, or record the capability as outside
the pi-core scope in p30. This file is intentionally source-path based; it
does not treat file presence as proof of behavioral parity.
