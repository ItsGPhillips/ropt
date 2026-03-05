# ropt: Next-Step Widget Ideas

This file collects concrete, scoped enhancements that fit the current design (stateful JSON tree, `push`/`append`/`pop`, `execute`, `show`) without turning ropt into a full TUI framework.

Each item is phrased as a “widget” or feature you could reasonably expose via new node kinds, flags, or renderers.

## 1. Choice Variants

- `confirm` widget (specialised flag)
  - Purpose: a richer yes/no with optional "Are you sure?" style confirmation.
  - API sketch:
    - `ropt append confirm --name "dangerous" --message "Delete everything?" --default=no --require-double-yes`
  - Behaviour:
    - Renders like `flag` but supports labels for yes/no, optional double-confirm for destructive actions, and a `--default` of `yes`/`no`.
  - Result type:
    - `ResultValue::Bool` under the hood.

- `multi_select` sugar
  - Purpose: ergonomics for common checkbox-style prompts without needing `--multiple`.
  - API sketch:
    - `ropt push multi-select --message "Enable features"`
    - Alias internally for `select --multiple`.
  - Behaviour:
    - Identical to `select` but biased UI copy and docs toward “checkbox list”.
  - Result type:
    - `ResultValue::Multiple(Vec<String>)`.

## 2. Text Input Variants

- `textarea` (multi-line input)
  - Purpose: capture longer free-form text (release notes, commit messages, descriptions).
  - API sketch:
    - `ropt append textarea --name "notes" --description "Release notes" --max-lines 20`
  - Behaviour:
    - Raw-mode line loop until blank line, `EOF` key combo, or `--max-lines` reached.
    - Reuses `InputType` validation where possible (length, custom regex) on the concatenated string.
  - Result type:
    - `ResultValue::Single(String)`.

- `password` / `secret` input sugar
  - Purpose: ergonomics around `--sensitive` for secrets.
  - API sketch:
    - `ropt append secret --name "api-key" --description "API key"`
    - Internally: `Input` node with `sensitive=true` and optional `input_type=string`.
  - Behaviour:
    - Uses existing `read_line` masking; may enforce non-empty by default.

## 3. Composite Widgets

- `toggle-group` (all-or-nothing flag + child options)
  - Purpose: common pattern of “enable feature X?” and, only if enabled, ask follow-up options.
  - API sketch:
    - `ropt push toggle-group --name "alerts" --description "Configure alerts"`
    -   `ropt append input --name "email" --type email`
    -   `ropt append select --name "severity" ...`
    - `ropt pop`
  - Behaviour:
    - Execution first prompts a `Flag` (“Configure alerts?”), then either walks or skips children depending on the answer.
  - Result type:
    - Parent stored as `Bool`, children stored as normal when enabled; possibly omitted or `null` in JSON when disabled.

- `step` / `wizard` grouping
  - Purpose: large multi-step flows with explicit step titles and progress output.
  - API sketch:
    - `ropt push step --label "Database"`
    - `ropt push step --label "Auth"`
  - Behaviour:
    - At execution, prints a step header (e.g., `Step 2/4: Auth`) before prompting contained nodes.
  - Implementation note:
    - Likely just a new `NodeKind::Step` that behaves like `Group` for validation but has dedicated rendering.

## 4. Display / Non-Interactive Nodes

- `note` / `info` node
  - Purpose: inject explanatory text or warnings into flows without producing a result value.
  - API sketch:
    - `ropt append note --message "These settings only affect staging."`
  - Behaviour:
    - Executor prints the message (styled) and continues; node never writes to `results`.

- `divider` / `spacer`
  - Purpose: visually separate sections in long interactive sessions.
  - API sketch:
    - `ropt append divider --label "Advanced"`
  - Behaviour:
    - Renders a simple rule or blank space with an optional label.

## 5. Enhanced Select Rendering

- `search-select` explicit renderer
  - Purpose: make the current "input with filter" mode addressable as a widget.
  - API sketch:
    - `ropt push select --message "Region" --render=search`
  - Behaviour:
    - Alias to existing `SelectRender::Input` initially.
  - Future:
    - Could add fuzzy matching (fzf-style) instead of substring.

- `paged-picklist`
  - Purpose: handle very long option lists more gracefully in TTY picklist mode.
  - API sketch:
    - `ropt push select --message "Choose" --render=picklist --page-size 15`
  - Behaviour:
    - Still keyboard-driven, but scrolls a window of N items instead of re-drawing all.

## 6. Result Shaping Widgets

- `map-select` (key/value mapping)
  - Purpose: support selections that should map to different shell variable names or structured results.
  - API sketch:
    - `ropt push map-select --name "target" --message "Target"`
    -   `ropt append option --value "debug"   --label "Debug"   --map-key "profile"`
    -   `ropt append option --value "release" --label "Release" --map-key "profile"`
    - `ropt pop`
  - Behaviour:
    - Executor writes `results["build.profile"] = "debug"`, etc., instead of `"build.target"`.
  - Notes:
    - Could be implemented as metadata on `Option` to influence key generation rather than a brand new node kind.

- `list-input` (repeatable input)
  - Purpose: collect a list of homogeneous values (ports, hostnames, tags) without writing the loop in shell.
  - API sketch:
    - `ropt push list-input --name "hosts" --type string`
    -   user enters many lines, blank line to finish
    - `ropt pop`
  - Result type:
    - `ResultValue::Multiple(Vec<String>)`.

## 7. UX Polish Widgets

- Per-node timeout override
  - Purpose: long-running prompts (e.g., free-form text) may need more than global `ROPT_TIMEOUT`.
  - API sketch:
    - `ropt append input --name "description" --timeout-secs 600`
  - Behaviour:
    - New field on `NodeDef` read by `executor::read_timeout`/`collect_results`.

- Default selection hints on `select`
  - Purpose: preselect first non-disabled option or a specific value by flag.
  - API sketch:
    - `ropt append option --value "debug" --default`
  - Behaviour:
    - Already supported for picklist; document and extend to filter mode (initial query seeded from default).

## 8. Debugging / Introspection Helpers

- `dry-run` execute mode
  - Purpose: preview what would be asked without prompting the user.
  - API sketch:
    - `ropt execute --dry-run --format=json`
  - Behaviour:
    - Walks the tree and emits a JSON description of prompts instead of asking them.

- `explain` command
  - Purpose: print a human-readable summary of what `execute` will ask.
  - API sketch:
    - `ropt explain` (or `ropt show --format=explain`)
  - Behaviour:
    - Similar to `show --format=tree`, but focused on "what will be asked" rather than structural details.
