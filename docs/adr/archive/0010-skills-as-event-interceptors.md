# Skills as event interceptors

Skills subscribe to the event bus and can intercept or modify events before they reach other actors. A skill can:

- Inject context into AgentResponse events
- Modify tool call arguments before ToolStart
- Preprocess user input on Submit
- Register custom events

Skills are self-describing modules that register themselves at startup. They do not require a full actor implementation — they are lightweight interceptors.

This enables extensibility without modifying core actors.

**Status:** Deferred to R2/R3. The core event bus must be stable before adding
interceptor hooks. MVP and R1 focus on solidifying the bus and state model.
