# Changelog

All notable changes to `br-llm-messages` are documented here. A single git tag
`v{version}` releases the crate. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## Unreleased

### Added

- Repository scaffold: crate manifest at `0.0.0`, governance files (LICENSE,
  CONTRIBUTING, SECURITY, SUPPORT, PR template, issue-template config),
  `.gitignore`, and `deny.toml`.
- CI (`ci.yml`): fmt + clippy + test, MSRV 1.89 build, `cargo-deny`,
  `cargo-machete`, `cargo semver-checks`, changelog + README-pin check,
  shellcheck, and trufflehog secret scan.
- CD (`release-tags.yml`): auto-tag and release the crate version on merge to
  `main`, inert while the version is the `0.0.0` scaffold.
- Conversation model (`SCHEMA_VERSION` `br-llm-messages/1`): validated value
  objects (`Text`, `ToolCallId`, `ToolName`, `Signature`, `Author`, `ModelId`,
  `TurnId`, `Base64Data`, `ImageMime`); content blocks (`UserBlock`,
  `AssistantBlock` with thinking/redacted-thinking/structured/tool-call,
  `ToolResultBlock`, `Image`, `ToolCall`, `ToolResult`); messages (`UserInput`
  with `UserSource`, `Step` with its four invariants, `ToolResults`);
  incremental `Turn` (`TurnItem`, `TurnState`) and `Conversation` (`Entry`,
  parallel open turns) with the same invariants re-checked on load.
- Streaming (`StreamEvent`, `BlockKind`, `StepDraft`): fold provider deltas into
  a `Step` through the domain constructor.
- Perspective render (`render`, `Perspective`, `WireMessage`, `WireUserBlock`):
  provider-neutral wire messages for one agent, own turns verbatim, every other
  entry framed as XML, consecutive user messages merged.
- Perspective render `WireMessage::Relay`: inputs that arrive after the last step
  of the agent's own open turn coalesce into a single provider-neutral relay
  message placed after that step's tool-results user message; a relayed input's
  images append to the preceding user-role message, never to the relay.
- `MessageError`: one crate-owned error type, one variant per refused rule.

### Changed

- Perspective render refuses (`TurnAwaitingResults`) when *any* of the agent's
  own turns still awaits tool results, not only the last one.
- Match arms over the `TurnItem` and `WireMessage` crate enums enumerate every
  variant instead of a wildcard `_`, so a new variant breaks the build.

### Tests

- Streaming draft: redacted-thinking assembly (`BlockStart` → `RedactedThinkingData`
  → `BlockEnd` → `Finish` preserves the data; an empty redacted block fails closed
  with `Blank`), `BlockEnd` refusals (no open block, wrong index), and a
  wrong-kind-delta table (`SignatureDelta`/`ThinkingDelta`/`RedactedThinkingData`
  on `Text`, `ToolCallArgumentsDelta` on `Structured`, `TextDelta` on
  `RedactedThinking`, `StructuredDelta` on a tool-call block).
- `AssistantBlock::RedactedThinking` valid serde round-trip.
- Perspective render: an intervening other-agent turn arriving while the agent's
  own turn is still open folds into the trailing `Relay`.
- Split `draft_tests` (protocol vs assembly) and `engine_tests` (frame vs relay,
  shared helpers) to keep each test file within the file-size budget.
- Perspective render: the mainline mid-turn continuation (the agent's own
  `AwaitingStep` turn as the last entry with no trailing input) ends the wire on
  its tool-results user message and emits no `Relay`; a non-perspective agent's
  turn left mid-flight (`AwaitingToolResults`) renders as an XML frame that drops
  its tool calls and thinking.
- Streaming draft: an empty `Structured` block fails closed with
  `InvalidJson { field: "structured" }` (contrast with an empty tool-argument
  fragment, which yields `{}`), an empty `Text` block fails closed with
  `Blank { field: "text" }`, and `finish()` re-runs the step invariants —
  `ToolCallsWithoutAwaiting` and `ToolCallNotAtTail` surface through the
  streaming path.
- `WireMessage::Relay`, `WireMessage::Assistant` and the three `TurnState`
  variants serde round-trip.
- `engine_relay_tests` match arms over `WireMessage` enumerate every variant
  instead of a wildcard `_`.
- Perspective render, own turns: two of the agent's own turns adjacent in the
  conversation render as two verbatim, adjacent `Assistant` messages (one per
  step). Coalescing consecutive same-role assistant messages — and honouring the
  provider's in-message block order (Anthropic's thinking-first rule and its
  thinking/signature pairing) — is the adapter's job, not the render's, since the
  correct merge varies by provider (design record §9). An own step carrying a
  `RedactedThinking` block and a `Structured` block renders byte-for-byte into
  its `Assistant` message, freezing the Anthropic redacted-thinking replay path.
- Perspective render, frames: another agent's `Structured` block renders as its
  JSON text inside the XML frame while thinking and tool calls are dropped; an
  intervening other-agent turn left mid-flight (`AwaitingToolResults`) folds into
  the trailing `Relay` with its unanswered tool calls dropped.
- `Turn` load refuses a partial tool-results item followed by a step
  (`TurnNotAwaitingStep`), matching the incremental `push_*` invariants.
- Split `draft_assembly_tests` into happy-path and error-path files to keep each
  test file within the file-size budget; the new own-turn render cases live in a
  dedicated `engine_own_turn_tests` file.
