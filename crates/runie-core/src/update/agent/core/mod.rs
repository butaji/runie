use crate::labels::{tool_done, tool_running};
use crate::message::{now, Part};
use crate::metrics;
use crate::model::{AppState, ChatMessage, Role};
use crate::update::agent::thought::{plan_thought, take_reasoning_parts, ThoughtPlan};

impl AppState {
    pub(crate) fn set_thinking(&mut self, id: String) {
        // Idempotent: skip if already streaming with the same request_id.
        if self.agent_state().streaming
            && self.agent_state().current_request_id.as_deref() == Some(&id)
        {
            return;
        }
        // Update AgentState projection directly.
        let agent = self.agent_state_mut();
        agent.streaming = true;
        agent.current_request_id = Some(id);
        agent.thinking_started_at = Some(std::time::Instant::now());
        agent.turn_active = true;
        agent.current_action = Some("Thinking".to_owned());
        agent
            .turn_started_at
            .get_or_insert_with(std::time::Instant::now);
        // Reset streaming buffer for new turn
        agent.streaming_buffer.reset();
        // Reset the streaming think filter for new turn
        agent.think_filter.reset();
        // Init speed tracking for this turn
        agent.turn_tokens_out = 0;
        agent.last_speed_update = Some(std::time::Instant::now());
        agent.tokens_at_last_speed = agent.tokens_out;
        agent.speed_tps = 0.0;
        // Keep existing rolling window - it auto-evicts to 1000 tokens
        let tokens = agent.tokens_out;
        agent.speed_window.record(tokens);
        self.messages_changed();
    }

    pub(crate) fn add_thought(&mut self, id: String) {
        // Idempotent: skip if we've already created a thought for this (request_id, thought_seq) combination.
        // The thought_id format is "{request_id}#thought.{thought_seq}".
        // We check if a thought already exists at the current thought_seq to avoid duplicates
        // when the same event is processed twice (e.g., once via handle_agent_event,
        // once via handle_turn_events when TurnActor emits).
        let current_seq = self.agent_state().thought_seq;
        let thought_id = format!("{}#thought.{}", id, current_seq);
        let already_processed = self
            .session
            .messages
            .iter()
            .any(|m| m.role == Role::Thought && m.id == thought_id);
        if already_processed {
            // Already created a thought at this seq; just clear thinking state.
            self.agent_state_mut().current_action = None;
            self.agent_state_mut().thinking_started_at = None;
            return;
        }

        let duration = self.thinking_elapsed_secs().unwrap_or(0.0);
        // Anchor the thought's sort timestamp to when thinking STARTED, not
        // to ThoughtDone time: the answer text streams (and bumps the
        // assistant message timestamp) before ThoughtDone fires, so a
        // ThoughtDone-time timestamp would sort the thought BELOW the answer
        // in the timestamp-ordered feed. Grok renders the thought above the
        // answer.
        let thought_ts = self
            .agent_state()
            .thinking_started_at
            .map(|started| crate::message::now() - started.elapsed().as_secs_f64())
            .unwrap_or_else(crate::message::now);
        // Clear thinking state on AgentState.
        self.agent_state_mut().current_action = None;
        self.agent_state_mut().thinking_started_at = None;
        // Resolve any partial-tag tail held back by the streaming think
        // filter, then flush remaining buffered text before planning.
        let think_tail = self.agent_state_mut().think_filter.finish();
        if !think_tail.visible.is_empty() {
            self.agent_state_mut()
                .streaming_buffer
                .push_delta(&think_tail.visible);
        }
        self.flush_buffered_response(&id);
        // Reasoning split out of streamed deltas (empty when the stream
        // contained no `<think>` markup, e.g. the Event::Response path).
        // Taken (not copied) so multi-cycle turns don't repeat reasoning.
        let streamed_reasoning = self.agent_state_mut().think_filter.take_reasoning();

        let (insert_idx, plan) = if let Some(idx) = self.find_assistant_by_id(&id) {
            // Providers that stream native reasoning (ThinkingDelta /
            // reasoning_content) accumulate it in Part::Reasoning, which
            // ChatMessage::content() ignores. Without this fallback the
            // thought would be duration-only — rendering a dead `[+]` — and
            // the reasoning would be lost when an otherwise-empty assistant
            // message is cleaned up. Move the parts into the thought so the
            // reasoning renders exactly once, from the thought.
            let part_reasoning = if streamed_reasoning.is_empty() {
                take_reasoning_parts(&mut self.session_mut().messages[idx])
            } else {
                String::new()
            };
            let plan = if !streamed_reasoning.is_empty() {
                // Deltas were split mid-stream: the message already holds
                // only visible text, so keep it and report the reasoning.
                let visible_empty = self.session().messages[idx].content().trim().is_empty();
                ThoughtPlan::streamed(duration, &streamed_reasoning, visible_empty)
            } else if !part_reasoning.is_empty() {
                // Native reasoning parts: same shape as the streamed path.
                let visible_empty = self.session().messages[idx].content().trim().is_empty();
                ThoughtPlan::streamed(duration, &part_reasoning, visible_empty)
            } else {
                plan_thought(&self.session_mut().messages[idx].content(), duration)
            };
            if plan.remove_assistant {
                self.session_mut().messages.remove(idx);
                self.agent_state_mut().last_assistant_index = None;
            } else if let Some(visible) = &plan.visible_content {
                self.session_mut().messages[idx].set_text_part(visible.clone());
            }
            (idx, plan)
        } else {
            let plan = if streamed_reasoning.is_empty() {
                ThoughtPlan::plain(duration)
            } else {
                ThoughtPlan::streamed(duration, &streamed_reasoning, true)
            };
            (self.session_mut().messages.len(), plan)
        };

        // Increment thought_seq.
        let thought_id = format!("{}#thought.{}", id, self.agent_state().thought_seq);
        self.agent_state_mut().thought_seq += 1;

        self.session_mut().messages.insert(
            insert_idx,
            ChatMessage {
                role: Role::Thought,
                timestamp: thought_ts,
                id: thought_id,
                parts: vec![Part::Text {
                    content: plan.thought_content,
                }],
                ..Default::default()
            },
        );

        // Update last_assistant_index if affected.
        if !plan.remove_assistant && self.agent_state().last_assistant_index == Some(insert_idx) {
            self.agent_state_mut().last_assistant_index = Some(insert_idx + 1);
        }
        self.messages_changed();
    }

    pub(crate) fn start_tool(&mut self, id: String, name: String) {
        // Idempotent: skip if already running this tool.
        if self.agent_state().current_tool_name.as_deref() == Some(&name) {
            return;
        }
        // Update AgentState for all tool-related fields.
        let agent = self.agent_state_mut();
        agent.current_request_id = Some(id.clone());
        agent.current_tool_name = Some(name.clone());
        agent.thinking_started_at = None;
        agent.tool_started_at = Some(std::time::Instant::now());
        agent.intermediate_step_count += 1;
        agent.current_action = Some(format!("Running {}", name));

        let tool_id = format!("tool.{}.{}", id, agent.intermediate_step_count);

        self.session_mut().messages.push(ChatMessage {
            role: Role::Tool,
            timestamp: now(),
            id: tool_id,
            parts: vec![Part::Text {
                content: tool_running(&name),
            }],
            ..Default::default()
        });

        if self.config().telemetry_enabled() {
            metrics::record_tool_usage(&name);
        }
        self.messages_changed();
    }

    pub(crate) fn end_tool(&mut self, duration_secs: f64, output: String) {
        // Clear tool state on AgentState.
        let agent = self.agent_state_mut();
        agent.current_action = None;
        agent.tool_started_at = None;
        let tool_name = agent.current_tool_name.take();

        if let Some(name) = tool_name {
            if let Some(idx) = self
                .session
                .messages
                .iter()
                .rposition(|m| m.role == Role::Tool && m.content().contains("⠋ Running "))
            {
                let max_bytes = self.config().truncation.max_bytes;
                let max_lines = self.config().truncation.max_lines;
                if let Some(last) = self.session_mut().messages.get_mut(idx) {
                    let output = crate::tool::truncate_output(&output, max_bytes, max_lines);
                    last.set_text_part(if output.trim().is_empty() {
                        tool_done(&name, duration_secs)
                    } else {
                        format!("{}\n{}", tool_done(&name, duration_secs), output)
                    });
                    last.timestamp = now();
                }
            }
        }
        self.messages_changed();
    }

    pub(crate) fn append_response(&mut self, id: String, content: String) {
        self.track_response_tokens(&content);
        if let Some(idx) = self.find_cached_assistant_index(&id) {
            self.append_to_message(idx, &content);
            return;
        }
        if let Some(idx) = self.find_assistant_by_id(&id) {
            self.agent_state_mut().last_assistant_index = Some(idx);
            self.append_to_message(idx, &content);
            return;
        }
        self.create_assistant_message(id, content);
    }

    /// Test-only helper — production uses streaming pipeline.
    #[allow(dead_code, reason = "test helper for streaming pipeline")]
    pub(crate) fn append_response_delta(&mut self, id: String, content: String) {
        self.track_response_tokens(&content);
        // Use AgentState for streaming_buffer.
        self.agent_state_mut().streaming_buffer.push_delta(&content);

        // Find existing assistant message and append stable content, or create new one.
        let stable_lines = self.agent_state_mut().streaming_buffer.flush();
        if !stable_lines.is_empty() {
            let stable = stable_lines.join("");
            if let Some(idx) = self.find_cached_assistant_index(&id) {
                self.append_to_message(idx, &stable);
            } else if let Some(idx) = self.find_assistant_by_id(&id) {
                self.agent_state_mut().last_assistant_index = Some(idx);
                self.append_to_message(idx, &stable);
            } else {
                self.create_assistant_message(id.clone(), stable);
            }
        }
    }

    fn track_response_tokens(&mut self, content: &str) {
        if content.is_empty() {
            return;
        }
        let agent = self.agent_state_mut();
        let n = agent.token_tracker.estimate_output(content);
        agent.tokens_out += n;
        agent.turn_tokens_out += n;
    }

    pub(crate) fn find_cached_assistant_index(&mut self, id: &str) -> Option<usize> {
        let idx = self.agent_state().last_assistant_index?;
        let msg = self.session_mut().messages.get(idx)?;
        if msg.role == Role::Assistant && msg.id == id {
            Some(idx)
        } else {
            None
        }
    }

    pub(crate) fn find_assistant_by_id(&self, id: &str) -> Option<usize> {
        self.session()
            .messages
            .iter()
            .position(|m| m.role == Role::Assistant && m.id == id)
    }

    /// Returns a mutable reference to the last assistant message, if any.
    pub(crate) fn current_assistant_message_mut(&mut self) -> Option<&mut ChatMessage> {
        self.agent_state()
            .last_assistant_index
            .and_then(|idx| self.session_mut().messages.get_mut(idx))
            .filter(|m| m.role == Role::Assistant)
    }

    /// Handle LLM lifecycle events to populate `parts` during streaming.
    pub(crate) fn handle_llm_event(&mut self, event: crate::event::Event) {
        use crate::event::Event as E;
        match event {
            E::TextStart { .. } => self.on_text_start(),
            E::ResponseDelta { id, content } => self.on_response_delta(id, content),
            E::TextEnd { .. } => {}
            E::ThinkingStart { .. } => self.on_thinking_start(),
            E::ThinkingDelta { content, .. } => self.on_thinking_delta(content),
            E::ThinkingEnd { .. } => {} // intentionally ignored: thinking content is accumulated
            E::AssistantMessageReady { message } => self.on_assistant_message_ready(message),
            // intentionally ignored: other events fall through
            _ => {}
        }
    }

    fn on_text_start(&mut self) {
        if let Some(msg) = self.current_assistant_message_mut() {
            msg.parts.push(Part::Text {
                content: String::new(),
            });
        } else {
            self.start_assistant_message(Part::Text {
                content: String::new(),
            });
        }
    }

    fn on_thinking_start(&mut self) {
        if let Some(msg) = self.current_assistant_message_mut() {
            msg.parts.push(Part::Reasoning {
                content: String::new(),
            });
        } else {
            self.start_assistant_message(Part::Reasoning {
                content: String::new(),
            });
        }
    }

    /// Push a new assistant message for a new request cycle.
    fn start_assistant_message(&mut self, part: Part) {
        let id = self
            .agent_state()
            .current_request_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let provider = self.config().current_provider.clone();
        self.agent_state_mut().current_request_id = Some(id.clone());
        let idx = self.session_mut().messages.len();
        self.session_mut().messages.push(ChatMessage {
            role: Role::Assistant,
            timestamp: now(),
            id,
            provider,
            parts: vec![part],
            ..Default::default()
        });
        self.agent_state_mut().last_assistant_index = Some(idx);
        self.messages_changed();
    }

    fn on_thinking_delta(&mut self, content: String) {
        self.track_response_tokens(&content);
        if let Some(msg) = self.current_assistant_message_mut() {
            if let Some(Part::Reasoning { content: reasoning }) = msg.parts.last_mut() {
                reasoning.push_str(&content);
            }
        }
    }

    fn on_response_delta(&mut self, id: String, content: String) {
        self.track_response_tokens(&content);
        // Split `<think>` reasoning out before text reaches the visible
        // message; partial-tag tails stay buffered in the filter until later
        // deltas (or turn finish) resolve them.
        let visible = self
            .agent_state_mut()
            .think_filter
            .push_delta(&content)
            .visible;
        if visible.is_empty() {
            return;
        }
        let has_open_text = self
            .current_assistant_message_mut()
            .is_some_and(|msg| matches!(msg.parts.last(), Some(Part::Text { .. })));
        if has_open_text {
            self.append_delta_to_text_part(&visible);
            return;
        }
        self.agent_state_mut().streaming_buffer.push_delta(&visible);
        // Try flush first (fast path for chunked text with newlines).
        let stable = self.agent_state_mut().streaming_buffer.flush().join("");
        if !stable.is_empty() {
            if let Some(idx) = self.find_cached_assistant_index(&id) {
                self.append_to_message(idx, &stable);
            } else if let Some(idx) = self.find_assistant_by_id(&id) {
                self.agent_state_mut().last_assistant_index = Some(idx);
                self.append_to_message(idx, &stable);
            } else {
                self.create_assistant_message(id.clone(), stable);
            }
            return;
        }
        // Flush returned empty: either (a) debounce hasn't elapsed, or (b) the
        // text hasn't ended with a stable delimiter yet. Force-flush the tail
        // and route it exactly like stable content: append to the existing
        // assistant message for this request when there is one (a second
        // iteration of the same turn must not spawn a duplicate message),
        // create only when no message exists yet.
        let tail = self
            .agent_state_mut()
            .streaming_buffer
            .force_flush()
            .join("");
        if !tail.is_empty() {
            if let Some(idx) = self.find_cached_assistant_index(&id) {
                self.append_to_message(idx, &tail);
            } else if let Some(idx) = self.find_assistant_by_id(&id) {
                self.agent_state_mut().last_assistant_index = Some(idx);
                self.append_to_message(idx, &tail);
            } else {
                self.create_assistant_message(id.clone(), tail);
            }
        }
    }
}

#[cfg(test)]
mod tests;
