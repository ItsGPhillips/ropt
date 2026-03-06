# ropt

Add interactive menus, text inputs, and yes/no prompts to any shell script — without writing any parsing logic yourself.

```
cargo install ropt
```

---

## What it is

Shell scripts that need user input usually end up doing one of three things: hard-coding values, reading positional arguments (and hoping the user gets the order right), or growing a wall of `getopts` boilerplate. ropt is a fourth option.

You describe what you want to ask — a pick list, a free-text input, a yes/no flag — using a series of `ropt` calls. When you run `ropt execute`, it renders the prompts interactively in the terminal. The user navigates with arrow keys, types, presses Enter, and when they're done, ropt hands back the results for your script to use.

All the interactive rendering happens on stderr. Results come out on stdout. So capturing output with `$()` works the way you'd expect.

---

## Quick look

```bash
#!/usr/bin/env bash
export ROPT_SESSION=$(ropt begin)

ropt push select --name "action" --message "What would you like to do?" --render=picklist
  ropt append option --value "deploy"   --label "Deploy application"
  ropt append option --value "rollback" --label "Rollback to previous version"
  ropt append option --value "status"   --label "Check deployment status"
ropt pop

action=$(ropt execute --format=raw)
ropt end

case "$action" in
  deploy)   echo "Deploying..." ;;
  rollback) echo "Rolling back..." ;;
  status)   echo "Checking status..." ;;
esac
```

What the user sees (on stderr, doesn't interfere with stdout):

```
? What would you like to do?
  ▶ Deploy application
    Rollback to previous version
    Check deployment status
```

After they pick "Deploy application" and press Enter, `$action` is `deploy`.

---

## How it works

Every `ropt` call is just a shell command. `push` opens a scope, `pop` closes it, `append` is both at once. You build up a structure describing your prompts — then `ropt execute` walks it, shows the prompts one after another, and outputs the answers.

The state between calls is tracked via `ROPT_SESSION`. You set it once at the top of your script:

```bash
export ROPT_SESSION=$(ropt begin)
```

Every subsequent `ropt` call in that shell process (or any subprocess) picks it up automatically. When you're done, `ropt end` cleans up.

Because push/append/pop are just shell commands running in sequence, you get conditional options for free:

```bash
ropt push select --name "target" --message "Select build target"
  ropt append option --value "debug"   --label "Debug"
  if [[ "$ENV" == "dev" ]]; then
    ropt append option --value "test" --label "Run test suite"
  fi
  ropt append option --value "release" --label "Release"
ropt pop
```

No special syntax. No DSL. The `if` block is just bash.

---

## Node types

| Type | What it does |
|------|-------------|
| `select` | Arrow-key menu or type-to-filter list. Contains `option` and `group` children. |
| `option` | A single selectable item inside a `select`. |
| `group` | A visual section header grouping related `option` nodes. |
| `flag` | A yes/no prompt. Result is `true` or `false`. |
| `input` | Free-text entry with optional type checking and validation. |

### select

```bash
ropt push select --name "env" --message "Target environment" [--render=auto|picklist|input] [--multiple]
  ropt append option --value "staging"    --label "Staging"
  ropt append option --value "production" --label "Production" --default
ropt pop
```

With fewer than 5 options, `--render=auto` (the default) uses an arrow-key picklist. With 5 or more it switches to a type-to-filter mode. You can force either with `--render=picklist` or `--render=input`.

Add `--multiple` to let the user pick more than one value. Results come back space-separated in raw mode, or as a bash array with `--format=sh`.

### option

```bash
ropt append option --value "the-value" --label "Display text" [--default] [--disabled]
```

`--default` pre-selects the option. `--disabled` shows it greyed out but won't let the user pick it.

### group

```bash
ropt push select --name "engine" --message "Database engine" --render=input
  ropt push group --label "Relational"
    ropt append option --value "postgres" --label "PostgreSQL"
    ropt append option --value "mysql"    --label "MySQL"
  ropt pop
  ropt push group --label "NoSQL"
    ropt append option --value "mongodb"  --label "MongoDB"
  ropt pop
ropt pop
```

Groups are display-only — the label appears as a non-selectable header in the list.

### flag

```bash
ropt append flag --name "verbose" --description "Enable verbose logging?"
# User sees:  ? Enable verbose logging? (y/n):
# Result:     true or false
```

### input

```bash
ropt append input --name "port" \
  --type number \
  --description "Port number" \
  --default-value "8080" \
  --validate-min 1 \
  --validate-max 65535
```

Supported types:

| Type | Validation |
|------|-----------|
| `string` | Any text; `--validate-min`/`--validate-max` set length bounds |
| `number` | Must parse as a number; min/max are numeric range |
| `email` | Must contain `@` with text on both sides |
| `path` | Must be non-empty |
| `regex:<pattern>` | Must fully match the embedded pattern |

Add `--validate-regex` on top of any type for an extra custom pattern check. Add `--sensitive` to mask characters as `*` (useful for passwords).

---

## Commands

```
ropt begin                                  Create a session, print its ID
ropt end          [--session=ID]            Delete the session
ropt push <type>  [options...]              Open a new scope
ropt append <type> [options...]             Add a node without changing scope
ropt pop          [--session=ID]            Close the current scope
ropt execute      [--format=raw|sh|json]    Run prompts, print results
                  [--prefix=PREFIX]
ropt read         --key <path>              Read one result value by key
ropt show         [--format=tree|json]      Debug: print the current structure
```

---

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ROPT_SESSION` | — | Active session ID. Set once with `export ROPT_SESSION=$(ropt begin)` and every subsequent call picks it up. Pass `--session=<id>` to any command to override it explicitly. |
| `ROPT_TIMEOUT` | `60` | Seconds before an unanswered prompt times out. |

---

## Examples

### 1. Simple pick list

```bash
#!/usr/bin/env bash
set -euo pipefail

export ROPT_SESSION=$(ropt begin)

ropt push select --name "action" --message "What would you like to do?" --render=picklist
  ropt append option --value "deploy"   --label "Deploy application"
  ropt append option --value "rollback" --label "Rollback to previous version"
  ropt append option --value "status"   --label "Check deployment status"
  ropt append option --value "quit"     --label "Exit"
ropt pop

action=$(ropt execute --format=raw)
ropt end

case "$action" in
  deploy)   echo "Deploying application..." ;;
  rollback) echo "Rolling back..." ;;
  status)   echo "Checking status..." ;;
  quit)     exit 0 ;;
esac
```

**What the user sees:**
```
? What would you like to do?
  ▶ Deploy application
    Rollback to previous version
    Check deployment status
    Exit
```

**What the script gets** (after selecting "Deploy application"):
```
deploy
```

---

### 2. Options that depend on runtime conditions

```bash
#!/usr/bin/env bash
set -euo pipefail

ENV="${ENV:-prod}"

export ROPT_SESSION=$(ropt begin)

ropt push select --name "target" --message "Select build target"
  ropt append option --value "debug"   --label "Debug"

  # Only appears when ENV=dev
  if [[ "$ENV" == "dev" ]]; then
    ropt append option --value "test" --label "Run test suite"
  fi

  ropt append option --value "release" --label "Release"

  if [[ "$ENV" != "prod" ]]; then
    ropt append option --value "staging" --label "Staging build"
  fi
ropt pop

target=$(ropt execute --format=raw)
ropt end

echo "Building: $target (env=$ENV)"
```

**With `ENV=dev`**, the user sees four options. **With `ENV=prod`**, they see two. No special ropt syntax — just an `if` block.

---

### 3. Text inputs and flags

When you have multiple prompts, `--format=raw` outputs one value per line sorted by key name. Assign them with `read`:

```bash
#!/usr/bin/env bash
set -euo pipefail

export ROPT_SESSION=$(ropt begin)

ropt push input --name "project-name" \
  --type string \
  --description "Project name (letters, numbers, hyphens only)" \
  --validate-regex '^[a-zA-Z0-9-]+$'
ropt pop

ropt push input --name "workers" \
  --type number \
  --description "Number of parallel workers" \
  --default-value "4" \
  --validate-min 1 \
  --validate-max 64
ropt pop

ropt append flag --name "dry-run" --description "Dry run (no side effects)?"

{ read -r dry_run; read -r project; read -r workers; } < <(ropt execute --format=raw)
ropt end

if [[ "$dry_run" == "true" ]]; then
  echo "[dry-run] Would build '$project' with $workers workers."
else
  echo "Building '$project' with $workers workers..."
fi
```

Raw output is sorted alphabetically by key name, so the order here is `dry-run`, `project-name`, `workers`. If that ordering feels fragile, see the [output formats](#output-formats) section — `--format=sh` and `--format=json` are better fits for multi-value results.

**What the user sees:**
```
? Project name (letters, numbers, hyphens only): my-app
? Number of parallel workers (default: 4): 8
? Dry run (no side effects)? (y/n): n
```

**What `ropt execute --format=raw` prints:**
```
false
my-app
8
```

---

### 4. Grouped options with type-ahead filtering

```bash
#!/usr/bin/env bash
set -euo pipefail

export ROPT_SESSION=$(ropt begin)

ropt push select --name "engine" --message "Database engine" --render=input
  ropt push group --label "Relational"
    ropt append option --value "postgres" --label "PostgreSQL"
    ropt append option --value "mysql"    --label "MySQL"
    ropt append option --value "sqlite"   --label "SQLite"
  ropt pop
  ropt push group --label "NoSQL"
    ropt append option --value "mongodb"  --label "MongoDB"
    ropt append option --value "redis"    --label "Redis"
    ropt append option --value "dynamodb" --label "AWS DynamoDB"
  ropt pop
ropt pop

engine=$(ropt execute --format=raw)
ropt end

echo "Selected: $engine"
```

**What the user sees** (they type "post" and the list filters live):
```
? Database engine: post
  ── Relational ──────────────────
  ▶ PostgreSQL
```

---

### 5. Options built from live data

```bash
#!/usr/bin/env bash
set -euo pipefail

export ROPT_SESSION=$(ropt begin)

# Build options from a git branch list
ropt push select --name "branch" --message "Target branch" --render=input
  while IFS= read -r branch; do
    branch="${branch#  }"
    branch="${branch#\* }"
    [[ -n "$branch" ]] && ropt append option --value "$branch"
  done < <(git branch 2>/dev/null)
ropt pop

branch=$(ropt execute --format=raw)
ropt end

echo "Deploying branch: $branch"
```

The option list is built at runtime from `git branch`. Any command, array, or file listing works the same way — you're just calling `ropt append option` in a loop.

---

## Output formats

`ropt execute` supports three output formats. The examples above use `--format=raw` for simplicity, but `--format=sh` and `--format=json` are often better when you have multiple prompts.

### `--format=raw`

One value per line, no keys, sorted alphabetically by key name:

```
deploy
false
8
```

Best for single-prompt scripts where you capture the result directly into a variable:

```bash
action=$(ropt execute --format=raw)
```

For multiple prompts it still works, but you need to know the sort order of your key names to assign values correctly with `read`.

### `--format=sh`

Shell variable assignments, safe to `eval`:

```bash
_ropt_out=$(ropt execute --format=sh --prefix=ropt_)
eval "$_ropt_out"
```

Variable names come from the `--name` you gave each node, with dots and hyphens replaced by underscores. The `--prefix` option prepends a string to all names, keeping ropt results from colliding with your own variables.

```bash
# --name "action" with --prefix ropt_:
ropt_action='deploy'

# --name "dry-run":
ropt_dry_run=false

# --multiple select --name "targets":
ropt_targets=('api' 'worker' 'scheduler')
```

Good choice when you have several prompts and want named variables without reaching for `jq`.

### `--format=json`

A JSON object, one key per prompt:

```bash
result=$(ropt execute --format=json)
target=$(echo "$result" | jq -r '.target')
```

```json
{
  "action": "deploy",
  "dry-run": false,
  "workers": "8"
}
```

Dot-separated `--name` paths nest into objects:

```json
{
  "build": {
    "target": "release",
    "workers": "4"
  }
}
```

Use this when you need structured access, or when the key names contain characters that are awkward to work with as shell variable names.

---

## License

MIT
