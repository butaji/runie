# Streaming: event per chunk

LLM response chunks are emitted as individual events. AgentLoop calls the LLM, each chunk arrives as `AgentResponse { chunk }`, which flows to the bus. ChatAgent receives and accumulates.

No buffering or batching of chunks — every chunk is an event. Throughput is handled by the bus.

Orchestrator does not coalesce streaming chunks.
