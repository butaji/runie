# Animation

Animations are demand-driven and actor-owned. A status actor advances a
deterministic frame counter; the render loop requests ticks only while a
phase needs animation.

Scrollback has an independent demand signal for running background/subagent
rows. Their bullet phase is reduced by `ScrollbackMsg::AdvanceAnimation` and
uses Grok's `⋅`, `:`, `⸬`, `⁙` sequence. The demand remains true after the
foreground status becomes idle and ends when the running row reaches a
terminal event. This is covered by the reducer test
`running_tool_bullet_advances_as_actor_owned_animation_state` and the YAML
background lifecycle fixture.

No test uses `sleep()`. Replay fixtures advance explicit animation events or
use deterministic snapshots.
