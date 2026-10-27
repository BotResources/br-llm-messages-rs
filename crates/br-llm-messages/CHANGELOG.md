# Changelog

All notable changes to `br-llm-messages` are documented here. The git tag
`br-llm-messages/v{version}` releases the crate; versions up to 0.2.0 were
released under the tag `v{version}`. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.
Release headings are plain `## X.Y.Z` — the release pipeline greps that exact
form to decide whether a version ships.

## 2.0.0 - 2026-10-26

Breaking: an input turn holds one or more incoming entries, in order, instead of exactly one; perspective projection returns a `ProjectedThread` of the same two turn kinds as a stored history.

- The stored conversation schema moves to `br-llm-messages/5`; documents of `br-llm-messages/4` (and earlier) do not load (`schema_mismatch`).
- The records, delta and draft schemas keep their identifiers. The records schema newly accepts an input turn with several entry rows; a 1.x reader refuses such a bundle with `invalid_record_structure`, and 1.x bundles load unchanged.
- A new document, the projected thread, has the schema `br-llm-messages-projected-thread/1`.

### Input turns

- **Breaking:** `InputTurn::new(id, entries: Vec<IncomingEntry>)` replaces `InputTurn::new(id, entry)`; an empty list is refused with `invalid_record_structure` (location `entries`), at build, on JSON reload and when rebuilt from records.
- **Breaking:** `InputTurn::entries() -> &[IncomingEntry]` replaces `InputTurn::entry()`; the JSON field `entry` becomes `entries` (array). A 1.x turn carrying `entry` is refused (`invalid_content`).
- Entry, block and citation ids stay unique across the whole conversation, including between the entries of one input turn. **Breaking:** a clash inside an input turn is `reused_identity` located at `entries[i]` of a standalone input turn and at `turns[i].entries[j]` in a conversation (1.x located it at `entry`).
- Record rebuild accepts several entry rows under one input turn, in row order, and refuses an input turn row with no entry row (`invalid_record_structure`, it was `invalid_content`).
- `Conversation::entry` and `Conversation::block` resolve every entry of a multi-entry input turn.
- **Breaking:** `EntryRef::Incoming(&IncomingEntry)` becomes `EntryRef::Incoming { entry, kind }`, a variant only the library builds (`#[non_exhaustive]`, match it with `{ entry, .. }`); `EntryRef::kind()` returns the recorded kind and cannot fail.
- **Breaking:** `IncomingEntry::kind()` returns `Option<TurnEntryKind>` (`None` for the projected-only entries) and `TurnEntry: From<IncomingEntry>` becomes `TryFrom<IncomingEntry>`.

### Projected thread

- **Breaking:** `Projection`, `ProjectedTurn`, `OwnTurn`, `OtherTurn` and `OtherItem` are replaced by `ProjectedThread`, which records the conversation identity and the target agent (`conversation()`, `agent()`) and holds `Turn::InputTurn` and `Turn::AITurn` only (`turns()`, `into_turns()`). The target agent's AI turns and all input turns stay as recorded; every other agent's AI turn becomes an input turn with the same turn identity, in the order of the foreign AI turn, holding one `AgentOutput` per completed model response (entry identity of the response, agent, `Author`, ordered blocks) or a `PartialAgentOutput` for a response still being produced, and the kept incoming entries of that turn (human messages, requests, mail, notifications) as recorded, with their kind, attribution, identities and blocks unchanged. Everything else (thinking, tool calls and results, closures, usage, other metadata) is excluded. An interrupted response and a response with no usable block give no entry; a foreign AI turn with no output and no kept incoming entry gives no input turn. No identity is minted. An empty thread is valid. A projection is a snapshot: a later projection may hold more entries under the same turn identity, so a consumer must not cache a projected turn by its identity.
- Output blocks may be `Text`, `Image` or `StructuredContent`. A response gives an entry if it has at least one such block among its root blocks (child blocks are never contributed; blank text is skipped). Each block keeps identity and content only: citations, replay material, item reference and observed metadata are dropped. `AgentOutput` and `PartialAgentOutput` have no `observed_at`. On reload of a projected thread, an output block that still carries any of them, or that is of another kind, is `disallowed_kind`; an output with no block, a blank text, or a block in raw-fragment state (no completed `value`) is `invalid_content`. A translated input turn, its outputs and their blocks carry no `observed_at` at projection; on reload an input turn is accepted with or without it, while an output entry or a block of one that carries it is `disallowed_kind`.
- A producing response contributes its blocks that are already final (text accumulated so far, images, completed structured content). A structured block still receiving JSON fragments is not a JSON value and is skipped, on every path (history, drafts, records).
- `Perspective::keeping(selection)` is new: an optional predicate evaluated once per incoming entry inside another agent's AI turn (default: keep all) and on no other entry, never on an input turn. The library decides nothing about visibility.
- `AgentOutput` and `PartialAgentOutput` (kinds `agent_output`, `partial_agent_output`) are new `IncomingEntry` variants that exist only in a projected thread; they have no public constructor. A stored history, an AI turn, standalone turn or entry JSON, a record row (`kind`) and `Perspective::project_turns` / `project_pending` (for an input turn given to them, located at `turns[i].entries[j]`) refuse them with `disallowed_kind`. Parsing either name with `str::parse::<TurnEntryKind>()` gives `variant_mismatch`, as does either name used as the discriminant of a block or a delta; deserializing either name as a `TurnEntryKind` with serde gives `disallowed_kind`.
- All outputs of one translated input turn share the agent and `Author` of the AI turn they come from (`invalid_record_structure` on reload otherwise, located at the first output that differs from the first output of the turn); the rule concerns outputs only. Outputs and recorded incoming entries may mix in one input turn, and `PartialAgentOutput` and `AgentOutput` may appear in any order and number. A projected thread refuses an AI turn of another agent (`disallowed_kind`, located at `turns[i]`) and an output of the target agent itself (`disallowed_kind`, located at `turns[i].entries[j]`).
- `Perspective::project(&Conversation)` stays infallible. **Breaking:** `Perspective::project_turns(conversation, turns)` now takes the conversation identity and returns a `Result`. `Perspective::project_pending(conversation, turns, pending)` is new. A `PendingResponse` is an opaque value built from a `ResponseDraft` (`PendingResponse::from(&draft)`) or a `ModelResponse` not recorded yet given with its AI turn identity (`PendingResponse::response(turn, &response)`): a producing draft gives a `PartialAgentOutput`, a completed draft or response gives an `AgentOutput` (accepted for a closed foreign turn too), an unstarted or interrupted draft gives nothing. They are placed at the end of the entries of their turn, in the order given, several per turn. An unknown turn (or an input turn) is `unknown_reference` and the target's own turn is `disallowed_kind`, both with the identity of the draft's entry. Every identity of a pending item that the projection has already received, or that two items share, is `reused_identity`; the owner leaves an older stored snapshot of the same response out. The identities of a pending item are its entry, its blocks (children included) and its citations; a draft in any phase and the response made from it (`PendingResponse::response(turn, &draft.response())`) reserve the same set. Giving the same item twice is `reused_identity` located at `pending[1]`.
- `RecordSet::into_projected_thread(conversation, agent)` and `RecordSet::into_projected_thread_pending(conversation, agent, pending)` build a thread from stored rows: full rows for input turns and the target agent's own AI turns (a row that is not complete is `disallowed_kind`), and for any other agent's AI turn its turn row, its model-response entry rows (complete, producing or interrupted) with their root `Text`, `Image` and `StructuredContent` block rows, and the rows of the incoming entries (with their blocks) the owner chose to keep. Any other row under another agent's turn is `disallowed_kind`, a child block row included. A kept block row drops citations, replay material, item reference and observed metadata silently. A producing `Text` row whose accumulated text is still blank is skipped, a producing `StructuredContent` row is skipped, and a response left with no usable block gives no entry.
- The payload of another agent's model-response row is validated as a response header: a complete row with the fields of a complete response, its `ending` optional there only (`{}` is accepted); a producing row as a producing header; an unknown field or a malformed value is `invalid_content`. Every row of an interrupted foreign response, of any kind or nesting, is ignored and its payload is not read; an interrupted row in the target's own turn or in an input turn is `disallowed_kind`.
- Identities are checked over every row received, interrupted and skipped rows included, and over the citation identities in the payload of an interrupted block row: a turn, entry or block identity given by two rows is `reused_identity`, so a reused identity never erases or changes the content of another row.
- An entry record loaded from JSON is checked exactly as `EntryRecord::check_local`, as in 1.x: a complete model-response row loaded from JSON needs its `ending` (`invalid_content`). The `ending` may be omitted only on a row of another agent's turn given to the records door, and such a row is built in memory (`EntryRecord::new`).
- **Breaking:** `RecordSet::to_json`, `to_value` and serde serialization run `RecordSet::check_local` first and return an error (same reason and location) for a set that `RecordSet::from_json` would refuse, such as a set holding a complete model-response row without `ending`: that set projects in memory but cannot be serialized. In 1.x these three never failed once the set was built.
- The in-memory path and the records door run one projection: each turns its input into the same projection input (kept turns, another agent's turns reduced to their kept incoming entries and their responses' usable blocks, the identities received) and then applies the same rules. The in-memory path receives only what a reduced record set would carry: the identities of another agent's thinking, tool calls and results, closures and unselected incoming entries are checked on neither path. For the same received content, selection and pending items, both paths give an equal thread or the same error reason and identity.
- The records door has no selection hook: `RecordSet::into_projected_thread` and `into_projected_thread_pending` take no predicate, so the caller sends only the incoming rows of another agent's turns it keeps (the in-memory counterpart is `Perspective::keeping`). When the rows sent are the ones `keeping` would keep, both paths give an equal thread.
- Error locations name the caller's own input. The in-memory path locates at `turns[i]` and `turns[i].entries[j]` of the turns it was given. The records door locates a fault of one row at that row (`turns[k]`, `entries[k]`, `blocks[k]`, followed by the path inside the row), a repeated identity at the row that repeats it, and a fault found when a turn or an entry is rebuilt from several rows at `turns[k].entries[j]`, where `k` is the turn row and `j` the position of the entry among the rows of that turn, as `RecordSet::into_conversation` does. A pending item is located at `pending[n]` on both paths. A projected thread loaded from JSON locates at the positions of that document; a repeated identity there is `reused_identity` located at `turns[i].entries[j]` (or `turns[i]` for a turn identity).
- `ProjectedThread` serializes with `to_json`/`to_value`/serde and reloads with `from_json`/`from_value` under the same rules (the constant `PROJECTED_THREAD_SCHEMA` names its schema); a conversation and a projected thread cannot be loaded as each other (`schema_mismatch`).

### Identity across the projection and the record

- A pending item (a draft or a response not recorded yet) is checked against the identities the projection received. It may equal an identity the projected view cannot see: a thinking block, a provider tool call or result (and its children), a citation of a hidden child block, a citation of a text block that is visible as an output (the output drops its citations), or a closure or entry of another agent's turn that the owner did not send. Uniqueness against the whole conversation is enforced when the response is recorded: `Conversation::push_entry`, `push_turn` and `from_parts` refuse the clash with `reused_identity` and the clashing identity (a test builds each case, projects it on both paths, then records it). The order rules of the turn run first, so a pending item for a closed turn can never be recorded.
- Interrupted identities are reserved on the projection doors: every row received, an interrupted foreign response, its blocks and the citations in its block payloads included, and every identity of a pending draft whatever its phase (entry, blocks, child blocks, citations), takes part in the uniqueness check, so a retry that reuses an identity of an interrupted response is `reused_identity` when both are given to one projection. A `Conversation` never holds an interrupted response, so recording a retry alone cannot see it; keeping a retry's identities fresh is the owner's rule.
- A block row names its entry by entry identity only; it carries no turn. The records door joins block rows to entry rows by that identity: the caller's query must select block rows by the store's unique entry key, an entry identity must be unique in the store (two entry rows with one identity are `reused_identity`), and a block row whose entry row was not sent is `invalid_record_structure` (located at `blocks[i].entry`).
- On the records door the `ending` of a complete model-response row is optional only for the rows of another agent's turn, which the owner builds in memory; every other row, and every row loaded from JSON, needs it (`invalid_content`). Such a set cannot be serialized (see above).

### Removed since 1.0.0

- `Usage::new` and `Usage::is_empty` (see Usage), `Projection`, `ProjectedTurn`, `OwnTurn`, `OtherTurn` and `OtherItem` (see Projected thread).
- **Breaking:** `Perspective` is no longer `UnwindSafe` nor `RefUnwindSafe`, because it holds the selection closure of `keeping`.

### Usage

- **Breaking:** `Usage::new()` and `Usage::is_empty()` are removed: an empty usage is no longer a value, so a builder cannot accept what the model refuses later. A usage starts from one observed count: `Usage::of_input_tokens`, `of_output_tokens`, `of_cache_read_input_tokens`, `of_cache_creation_input_tokens`, `of_thinking_tokens`, then the `with_*` methods add the others. JSON with no count (`{}`) is still `invalid_content` located at `usage`.

### Migration from 1.x

- `Usage::new().with_input_tokens(n)` becomes `Usage::of_input_tokens(n)` (same for the other counts).
- `InputTurn::new(id, entry)` becomes `InputTurn::new(id, vec![entry.into()])`; `turn.entry()` becomes `&turn.entries()[0]` or an iteration over `turn.entries()`.
- Stored `br-llm-messages/4` conversations are migrated by the consumer: wrap each input turn's `entry` in an `entries` array and set the schema to `br-llm-messages/5`. Record rows need no change.
- Code reading `Projection` turns reads `ProjectedThread::turns()` as `Turn` values; another agent's text is read from the `AgentOutput` entries of the input turns that carry its turn identity.
- `OwnTurn::unresolved_calls()` is replaced by the state of the target's own AI turn: `AITurn::state()` gives `is_closed()` and `outstanding_runtime_calls()`; a call outstanding on a closed turn is unresolved.
- `EntryRef::Incoming(entry)` patterns become `EntryRef::Incoming { entry, .. }`.
- A `RecordSet` holding foreign rows without `ending` is no longer serializable: pass it to `into_projected_thread` or `into_projected_thread_pending` and serialize the `ProjectedThread`.

## 1.0.0 - 2026-10-24

Breaking rewrite of the model: turns of typed entries and content blocks, UUIDv7 ids, streaming drafts, storage records, perspective projection.
Stored schemas are `br-llm-messages/4`, `br-llm-messages-delta/1`, `br-llm-messages-draft/1` and `br-llm-messages-records/1`; documents of earlier schemas do not load.
An explicit JSON `null` for an optional key is refused with `invalid_content` in every document (turns, entries, blocks, deltas, delta streams, draft checkpoints, record rows and payloads); an absent key is absent. This includes the `arguments` of a provider tool call, which can no longer hold the JSON value `null`.
Removed: `Step`, `TurnItem`, `UserInput`, `lighten`, `render`; tool offering is not this library's concern.

## 0.5.0 - 2026-10-15

Breaking: a user input carries a required id chosen by its creator, entry
ids (input ids and turn ids together) are unique in the conversation, and
the stored schema moves to `br-llm-messages/3`; documents stored under
`br-llm-messages/2` no longer load.

### Added

- `InputId`: the id of a user input, chosen by whoever creates the input
  (the library never makes one up), refused when empty
  (`Blank { field: "input_id" }`); one space is accepted; reached from the
  crate root.
- `UserInput::id()`.
- `Conversation::entry(&str) -> Option<&Entry>`: an entry, input or turn, is
  found by its id, beside `Conversation::turn(&TurnId)`. Input ids and turn
  ids are one space, so the lookup takes the id text.
- `Entry::id() -> &str`: the input id of an input, the turn id of a turn.
- `MessageError::DuplicateEntryId { id: String }`: an input or a turn whose
  id is already the id of an entry, input or turn.

### Changed

- **Breaking:** `UserInput::new(id, source, author, content)` takes the
  input's `InputId` first.
- **Breaking:** JSON: a user input is `{"id", "source", "author"?,
  "content"}`; `id` is required, so an input without id no longer loads, on
  its own or inside a conversation.
- **Breaking:** entry ids, input ids and turn ids together, are unique in the
  conversation: an input and a turn never share an id. `Conversation::push_input`
  now returns `Result<(), MessageError>` and refuses an input whose id is
  used by any entry; `Conversation::push_turn` refuses a turn whose id is
  used by any entry, input included. Both refuse with `DuplicateEntryId`,
  at construction (the conversation unchanged) and on load. A turn's entry
  id is checked before its tool call ids.
- **Breaking:** `MessageError::DuplicateTurnId { id: TurnId }` is replaced by
  `MessageError::DuplicateEntryId { id: String }`.
- **Breaking:** `SCHEMA_VERSION` is `br-llm-messages/3`; any other schema,
  `br-llm-messages/2` included, is refused on load. Entries are still loaded
  first, and the schema is checked before the duplicate entry id, so a wrong
  schema wins over a duplicate entry id.
- `Conversation::turn`, `push_step` and `push_result` reach a turn through an
  index of the entries by id; the id of an input reaches no turn
  (`TurnNotFound`).
- Lightening keeps user inputs and own turns as they are, ids included; the
  rustdoc of `lighten` states that a lightened conversation is a view for one
  render, never stored as the conversation. No behaviour change.
- Perspective render: unchanged. No entry id appears in a render: frames
  carry the author's name, the role and the kind, and wire messages carry no
  id.

### Tests

- `InputId`: empty refused, one space accepted, kept verbatim, at
  construction and on load; a user input without id or with an empty id
  refused on load; its JSON is `{"id", "source", "author"?, "content"}`.
- Entry ids: an input or a turn reusing the id of an input or a turn refused
  at construction (conversation unchanged) and on load, in all four pairings;
  the entry id reported before a reused call id; ids differing by whitespace
  distinct; lookup of inputs and turns by id; the id of an input reaches no
  turn.
- Schema: `br-llm-messages/3` loads with ids and re-serializes identically;
  `br-llm-messages/1` and `/2` refused as a schema mismatch; a wrong schema
  wins over a duplicate entry id; a refused entry wins over a wrong schema.
- Random walks mint distinct input ids. On every state: every entry is found
  by its id, an input and a turn reusing each entry id are refused with the
  conversation unchanged, and a stored conversation with an entry appended
  under a used id is refused on load.
- Piece by piece: a stored input carries its id and reloads equal; an input
  whose id is used is refused when pushed back; rebuilding by pushes still
  equals the original.
- Render: no entry id appears in the render of every kind of entry (inputs,
  own turns, another agent's turn, a relay) nor on any state of the walks,
  original or lightened, for every agent; renaming every entry leaves the
  render unchanged.
- Lightening: entry ids kept in order, on fixtures and on every state of the
  walks; a lightened conversation still finds entries by id and refuses a
  reused entry id.
- Public surface: `InputId` named from the crate root, unreachable through
  `value`; `SCHEMA_VERSION` is `br-llm-messages/3`.

## 0.4.0 - 2026-10-14

Breaking: stricter rules on tool names, tool call ids and tool results, a
`RuntimeKind` value, a public surface reached from the crate root only, and
JSON as the only stored form. Documents that break a new rule no longer load;
the schema stays `br-llm-messages/2`.

### Added

- `check_wire_list(&[WireMessage])`: a pure check, called by choice, of a
  list of wire messages from anywhere. It refuses a list that is empty
  (`WireListEmpty`), that does not start with a `User` message
  (`WireListNotStartingWithUser`), in which a tool call of an `Assistant`
  message is not answered exactly once, by id, in the next `User` message
  (`WireToolCallUnanswered`, `WireToolCallAnsweredTwice`), in which a tool
  result does not answer a tool call of the `Assistant` message just before
  it (`WireToolResultUnmatched`), or in which a tool call id appears more than
  once (`WireToolCallIdRepeated`). Each refusal carries the index of the
  message and, for a rule about a block, the index of the block; the first
  fault in message order, then block order, is reported. A list
  yielded by `render` always passes; loading a list from JSON still checks no
  rule of the list.
- `RuntimeKind`: the kind of a runtime input, refused when empty
  (`Blank { field: "runtime_kind" }`); one space is accepted.
- Piece by piece: a `UserInput`, a `Step` and a `ToolResult` each serialize
  and load on their own under every rule, and pushing them back in their
  order (an input, a turn started with its first step, a step or a result
  reaching its turn by id) rebuilds a conversation equal to the original,
  serializing to the same JSON. Positions are stable: entries are never
  removed or reordered, and a turn only receives steps and results at its
  end. No new type and no new JSON shape.

### Changed

- **Breaking:** `ToolName` is 1 to 64 ASCII characters, each a letter, a
  digit, `_` or `-`, the first a letter or `_`
  (`^[A-Za-z_][A-Za-z0-9_-]{0,63}$`), the rule every main provider accepts.
  Any other name, the empty name included, is refused with the new
  `MessageError::InvalidToolName` (formerly `Blank { field: "tool_name" }` for
  the empty name), at construction and on load.
- **Breaking:** `UserSource::Runtime { kind }` carries a `RuntimeKind`
  instead of a `String`, so an empty kind can no longer be built by hand.
  `UserSource::runtime(kind)` is unchanged; the JSON shape is unchanged.
- **Breaking:** tool call ids are unique within the turn: `Turn::push_step`
  (and `Conversation::push_step`) refuses a step whose call id is already
  used by a step of the turn (`DuplicateToolCallIdInTurn`), at construction
  and on load. 0.3 accepted the reuse.
- **Breaking:** tool call ids are unique in the conversation:
  `Conversation::push_turn` and `Conversation::push_step` refuse a step (a
  turn's first step and every step of a pushed turn included) when one of its
  call ids is already used by a call of any turn
  (`DuplicateToolCallIdInConversation`), at construction and on load. The turn
  rule is reported before the conversation rule.
- **Breaking:** a tool result is accepted only when its tool name is the name
  of the call it answers (`ToolNameMismatch { id, call, result }`), after the
  pending and once rules, at construction and on load.
- **Breaking:** a turn loaded with a `tool_results` item holding no result is
  refused (`EmptyToolResults`); 0.3 dropped the item. Consecutive non-empty
  items still merge into one.
- `Turn::push_results` documents that the batch is not atomic: it stops at
  the first refused result and the results accepted before it stay in the
  turn (unchanged behaviour).
- Stream draft: finishing the draft drops every blank text block (empty or
  whitespace only) instead of refusing the finish, and builds the step from
  the other blocks; a reply left with no block is refused as `EmptyStep`.
  `InvalidJson { field, .. }` and `EmptyStep` stay distinct refusals.
- **Breaking:** every public type is reached from the crate root only: the
  modules `block`, `conversation`, `error`, `render`, `step`, `stop_reason`,
  `stream`, `tool_results`, `turn`, `usage`, `user_input` and `value` are
  private (`br_llm_messages::block::Image` becomes `br_llm_messages::Image`).
- **Breaking:** `#[non_exhaustive]` on the enums that may grow:
  `MessageError`, `StreamEvent`, `BlockKind` and `AssistantBlock`
  (`ToolResultBlock` already was). Every other public enum is closed.
- **Breaking:** JSON is the only stored form: `Display` is removed from
  `Conversation`, `Turn` and `UserInput`; `Deserialize` is removed from
  `TurnState`, `Perspective` and `BlockKind`; `Serialize` is removed from
  `StreamEvent` (stream events still load from JSON, block kinds included).
- `MessageError` gains the variants `InvalidToolName`,
  `DuplicateToolCallIdInTurn`, `DuplicateToolCallIdInConversation`,
  `ToolNameMismatch`, `EmptyToolResults` and the six `Wire*` variants above.

### Tests

- `ToolName`: accepted and refused names (empty, one space, leading digit or
  `-`, dots, slashes, spaces, non-ASCII, 65 characters) at construction and
  on load.
- `RuntimeKind`: built by hand into `UserSource::Runtime`, one space
  accepted, an empty kind refused in a loaded user input.
- Tool call ids: reuse within a turn and across turns refused at
  construction (conversation unchanged) and on load; the turn rule and the
  turn state win over the conversation rule; a lightened conversation keeps
  the rule for the calls it holds.
- Tool results: a result naming another tool refused at construction (turn
  unchanged) and on load; an empty `tool_results` item refused on load after
  an awaiting step, after a finished step and between two items; consecutive
  items merged.
- Stream draft: blank text blocks dropped before, between and after other
  blocks (between two tool calls included); only blank blocks refused as
  `EmptyStep`; a cut structured value still `InvalidJson`.
- Piece by piece: seeded random walks (several open turns, interleaved
  results, every block kind) stored piece by piece and pushed back in their
  order, and loaded conversations decomposed entry by entry and pushed back,
  rebuild equal conversations with the same JSON; every push leaves every
  earlier position unchanged.
- Wire list check: every rule with its position; a list loaded from JSON
  that breaks the rules loads and is refused by the check only; the render of
  own turns with several rounds, framed entries, a relay and merged `User`
  messages passes, lightened or not, and so does the render of every state of
  the random walks for every agent (with coverage counters for tool calls,
  relays and merged results).
- Public surface (`tests/public_surface.rs`): every public type named from
  the root; closed enums matched without a wildcard; the enums that may grow
  need one; `Display`, `Serialize` and `Deserialize` probed on the stored,
  computed and transient types.
- Lightening law on the random walks: `render(lighten(c, p), p)` equals
  `render(c, p)`, refusals included, for every state and every agent.
- Only from the root (`src/only_from_the_root.rs`, doctests): naming an item
  through any module public up to 0.3 does not compile; CI now runs
  `cargo test --doc`.
- Stream events: every serialized `BlockKind` loads back through a block
  start.
- Split the lightening fixtures (`lighten_test_helpers.rs`) and the draft
  assembly (`stream/assemble.rs`) into their own files.

## 0.3.0 - 2026-10-14

Breaking: `Author` changes shape and the stored schema moves to
`br-llm-messages/2`; documents stored under `br-llm-messages/1` no longer load.

### Added

- Lightening (`lighten`): a pure function taking a conversation and a
  perspective and returning a lighter conversation whose render for that
  perspective equals the render of the original, or the same refusal. User
  inputs and the agent's own turns are kept as they are and entries keep their
  order. Every other turn keeps its id and its author and holds one finished
  step (`end_turn`) made of the texts and structured values of its steps, in
  order, or of one empty thinking block (no signature) when it has none; its
  thinking, redacted thinking, tool calls, tool results, usage and model ids
  are dropped.
- `AuthorId` and `AuthorName` value objects, each refused when empty
  (`Blank { field: "author_id" }` / `Blank { field: "author_name" }`); a value
  of one space is accepted.

### Changed

- **Breaking:** `Author` is an id and a name (`Author::new(id, name)`,
  `Author::from_parts`, `id()`, `name()`) instead of a single non-empty string;
  `Author::as_str` and the `Blank { field: "author" }` refusal are gone. Two
  authors are the same author when their ids are equal, whatever their names:
  `PartialEq`, `Eq` and `Hash` look at the id alone. `Display` shows the name.
- **Breaking:** JSON: an author is `{"id", "name"}` (still optional on user
  inputs and turns) instead of a string.
- **Breaking:** `SCHEMA_VERSION` is `br-llm-messages/2`; any other schema,
  `br-llm-messages/1` included, is refused on load.
- Perspective render: a turn is the perspective agent's own when its author's
  id equals the agent's id, so a renamed agent still owns its earlier turns and
  another agent bearing the same name does not. A turn without author is never
  own. In frames, `author="…"` is the author's name, omitted when there is no
  author.
- Repository is a Cargo workspace: the crate lives in `crates/br-llm-messages/`.
  The package name is unchanged.
- CI/CD: the libraries registry gate on pull requests, the release-line law
  (`main` and `release/<crate>-<major>.<minor>` are the only publishing
  branches), and a release chain that checks the registry target, computes the
  API listing, the API diff against the registry baseline and the
  dependencies, tags `br-llm-messages/v{version}`, creates the release and
  records it in the registry. Offline tests cover the wiring.

### Tests

- Author: id and name validation both ways, JSON shape, unknown fields
  dropped, same author by id across a rename (equality and hash), different
  ids with one name are different authors.
- Render: an own turn written under an old name stays own after a rename; a
  namesake with another id is framed; frames show the name, never the id.
- Schema: `br-llm-messages/2` loads with `{"id", "name"}` authors and
  re-serializes identically; `br-llm-messages/1` is refused as a schema
  mismatch.
- Lightening: inputs and own turns untouched; another agent's multi-step turn
  with thinking, redacted thinking, tool calls and results reduced to one
  `end_turn` step of texts and structured values in order, usage and model
  dropped; a turn of thinking and tool calls only reduced to one empty
  thinking step; a turn without author lightened as another agent's; entries
  keep their order; `render(lighten(c, p), p) == render(c, p)` on a plain
  conversation (two perspectives), a relay case, an own turn awaiting results
  (same `TurnAwaitingResults` refusal), a render starting with an own turn and
  an empty conversation; the lightened conversation reloads from JSON.

## 0.2.0 - 2026-10-08

### Changed

- Perspective render: nothing in a `<message>` frame is escaped any more. The
  body (a framed input or another agent's turn) and the `author` and `kind`
  attributes reach the model as written, markup, `<`, `>`, `&` and `"`
  included. The frame is a prompt convention marking message boundaries for
  the model, not an XML document; callers that frame untrusted text sanitize
  it themselves.

## 0.1.0 - 2026-09-21

### Added

- Repository scaffold: crate manifest, governance files (LICENSE,
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
