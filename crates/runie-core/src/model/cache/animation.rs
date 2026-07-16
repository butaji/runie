//! Animation state and helpers for view updates.

use crate::model::state::AppState;

impl AppState {
    /// Advance animation state on each tick.
    pub fn tick_animation(&mut self) {
        let mut changed = false;
        // Tick whenever the turn is active, a tool is running, a subagent is running,
        // or a permission prompt is waiting for user input — all of these animate.
        let agent = self.agent_state_mut();
        let any_work_active = agent.turn_active
            || agent.current_tool_name.is_some()
            || agent
                .pattern_workers
                .iter()
                .any(|w| w.status == crate::model::PatternWorkerStatus::Running);
        drop(agent);
        if any_work_active {
            self.view_mut().animation_frame = self.view_mut().animation_frame.wrapping_add(1);
            self.update_speed();
            changed = true;
        }
        if self.input_mut().input_flash > 0 {
            self.input_mut().input_flash -= 1;
            changed = true;
        }
        if self.clear_expired_transient() {
            changed = true;
        }
        if self.animate_tokens() {
            changed = true;
        }
        if changed {
            self.view_mut().dirty = true;
        }
    }

    /// Animate token display values toward their actual values.
    fn animate_tokens(&mut self) -> bool {
        if self.agent_state_mut().tokens_in != self.agent_state_mut().tokens_in_prev {
            self.agent_state_mut().tokens_in_prev = self.agent_state_mut().tokens_in;
        }
        if self.agent_state_mut().tokens_out != self.agent_state_mut().tokens_out_prev {
            self.agent_state_mut().tokens_out_prev = self.agent_state_mut().tokens_out;
        }
        let c1 = Self::animate_token_value(
            self.agent_state_mut().tokens_in,
            &mut self.agent_state_mut().tokens_in_display,
        );
        let c2 = Self::animate_token_value(
            self.agent_state_mut().tokens_out,
            &mut self.agent_state_mut().tokens_out_display,
        );
        c1 || c2
    }

    fn animate_token_value(target: usize, display: &mut f64) -> bool {
        let t = target as f64;
        let d = t - *display;
        if d.abs() < 0.5 {
            let changed = display.round() as usize != target;
            if changed {
                *display = t;
            }
            changed
        } else {
            *display += d * 0.15;
            true
        }
    }

    /// Update streaming speed using rolling window of last 1000 tokens.
    pub fn update_speed(&mut self) {
        let now = std::time::Instant::now();
        let (elapsed, tokens_out, tokens_at_last_speed) = {
            let agent = self.agent_state_mut();
            let last = agent.last_speed_update.get_or_insert(now);
            (
                now.duration_since(*last).as_secs_f64(),
                agent.tokens_out,
                agent.tokens_at_last_speed,
            )
        };
        if elapsed < 0.05 {
            return;
        }
        let delta_tokens = tokens_out.saturating_sub(tokens_at_last_speed);
        if delta_tokens > 0 {
            self.agent_state_mut().speed_window.record(tokens_out);
            self.agent_state_mut().tokens_at_last_speed = tokens_out;
            self.agent_state_mut().speed_tps = self.agent_state_mut().speed_window.speed();
            if let Some(last) = self.agent_state_mut().last_speed_update.as_mut() {
                *last = now;
            }
        } else if elapsed > 1.0 {
            self.agent_state_mut().speed_tps *= 0.5;
            if self.agent_state_mut().speed_tps < 0.1 {
                self.agent_state_mut().speed_tps = 0.0;
            }
        }
    }

    fn clear_expired_transient(&mut self) -> bool {
        if let Some(until) = *self.transient_until_mut() {
            if std::time::Instant::now() > until {
                *self.transient_message_mut() = None;
                *self.transient_until_mut() = None;
                *self.transient_level_mut() = None;
                return true;
            }
        }
        false
    }
}
