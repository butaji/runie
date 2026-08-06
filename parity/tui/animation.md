# Animation

Animations are demand-driven and actor-owned. A status actor advances a
deterministic frame counter; the render loop requests ticks only while a
phase needs animation.

No test uses `sleep()`. Replay fixtures advance explicit animation events or
use deterministic snapshots.
