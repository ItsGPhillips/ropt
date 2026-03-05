# ropt

**Interactive CLI option configuration tool.**

Define prompts declaratively in shell scripts; let `ropt` handle the interactive rendering.

```
cargo install ropt
```

---

## What it does

Instead of writing complex argument-parsing logic, you define your CLI structure with a sequence of `ropt` calls and then run `ropt execute`. The tool presents keyboard-navigable menus, text inputs, and yes/no prompts, then outputs the results in whichever format your script needs.

```bash
export ROPT_SESSION=$(ropt begin)

ropt push select --message "Build target"
  ropt append option --value "debug"   --label "Debug"
  ropt append option --value "release" --label "Release"
ropt pop

eval "$(ropt execute --format=sh)"
ropt end

echo "Building: $select_0"
```

---

## Core concepts

### Session lifecycle

Every `ropt` invocation is stateless on its own; state is kept in a small JSON file. Sessions are explicit:

```bash
export ROPT_SESSION=$(ropt begin)   # create state file, export ID
# ... push/append/pop calls ...
ropt execute                         # drive interactive prompts
ropt end                             # delete state file
```

The session ID flows automatically via `ROPT_SESSION`, or you can pass `--session=<id>` on any command.

### Stack-based structure

`push` opens a scope; `pop` closes it; `append` is `push + pop` in one step.

```bash
ropt push command --name "deploy"
  ropt push select --message "Environment"
    ropt append option --value "staging"
    ropt append option --value "production"
  ropt pop
  ropt append flag --name "dry-run" --description "Preview only"
ropt pop
```

### Native shell control flow

Because each call mutates the session file, ordinary `if`/`for`/`case` statements work as conditional option builders:

```bash
ropt push select --message "Actions"
  for env in "${ENVS[@]}"; do
    ropt append option --value "$env"
  done
  if [[ "$ADMIN" == "true" ]]; then
    ropt append option --value "nuke" --label "Destroy everything"
  fi
ropt pop
```

---

## Node types

| Type | Description |
|------|-------------|
| `select` | Interactive option picker. Contains `option` and `group` children. |
| `option` | Selectable choice inside a `select` or `group`. |
| `group` | Visual grouping of `option` nodes (like `<optgroup>`). |
| `flag` | Boolean yes/no prompt. |
| `input` | Free-text input with optional type and length validation. |
| `argument` | Named container grouping related nodes. |
| `command` | Top-level sub-command container. |

### select

```bash
ropt push select --message "Pick one" [--render=auto|picklist|input] [--multiple]
  ropt append option --value "a" --label "Option A"
  ropt append option --value "b" --label "Option B" --default
  ropt push group --label "Advanced"
    ropt append option --value "c" --disabled
  ropt pop
ropt pop
```

Rendering is automatic by default: fewer than 5 options → arrow-key picklist; 5 or more → type-to-filter input.

### option

```bash
ropt append option --value "val" --label "Display text" [--default] [--disabled]
```

### flag

```bash
ropt append flag --name "verbose" --short v --description "Verbose output"
# Result: true or false
```

### input

```bash
ropt append input --name "port" --type number --default-value "8080" \
  --validate-min 1 --validate-max 65535
# --type: string (default), number, email, path, regex:<pattern>
```

---

## Commands

```
ropt begin                          Print a new session ID; create its state file
ropt end [--session=ID]             Delete the session state file
ropt push <type> [options...]       Open a new scope
ropt append <type> [options...]     Add a node without changing scope (push + pop)
ropt pop [--session=ID]             Close current scope
ropt execute [--format=json|sh|raw] Run prompts; print results
ropt read --key <path>              Print one result value
ropt show [--format=tree|json]      Debug: display current definition structure
```

### Output formats

| Format | Use case |
|--------|----------|
| `json` | `result=$(ropt execute --format=json)` then `jq` |
| `sh`   | `eval "$(ropt execute --format=sh)"` — sets shell variables |
| `raw`  | Plain values, one per line |

Shell variable names are derived from the result key path: `build.output-dir` → `build_output_dir`.

---

## Result key paths

Results are keyed by the dot-separated path of node names/indices:

```bash
ropt push command --name "build"
  ropt append select --name "target" --message "Target"
ropt pop
# Key: "build.target"
target=$(ropt read --key "build.target")
```

---

## Security

- Session files stored in `$XDG_RUNTIME_DIR/ropt/` or `~/.ropt/tmp/` with `0600` permissions.
- Ownership verified on every read.
- Integrity checksum validated on every read; tampered files are rejected.
- Exclusive file lock held during all write operations.
- Hard limits: depth 10, 1 000 options per select, 10 KB input, 1 MB session file.
- Prompt timeout: 60 s (override with `ROPT_TIMEOUT=<seconds>`).

---

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ROPT_SESSION` | — | Active session ID (set by `export ROPT_SESSION=$(ropt begin)`) |
| `ROPT_TIMEOUT` | `60` | Prompt timeout in seconds |
| `XDG_RUNTIME_DIR` | — | Preferred location for session files |

---

## License

MIT
