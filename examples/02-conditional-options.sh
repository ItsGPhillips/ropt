#!/usr/bin/env bash
# Example 2: Conditional Options via Bash Control Flow
#
# ropt has no built-in conditional syntax — you don't need it.
# Because every `ropt append/push/pop` is just a shell command mutating
# a session file, native bash if/else/for/case IS the conditional logic.
#
# Usage:
#   ./02-conditional-options.sh          # defaults to ENV=prod
#   ENV=dev ./02-conditional-options.sh  # shows extra dev-only options

set -euo pipefail

ENV="${ENV:-prod}"

export ROPT_SESSION=$(ropt begin)

ropt push select --name "target" --message "Select build target"
  ropt append option --value "debug"   --label "Debug"

  # This option only appears when ENV=dev — no special ropt syntax needed.
  if [[ "$ENV" == "dev" ]]; then
    ropt append option --value "test" --label "Run test suite"
  fi

  ropt append option --value "release" --label "Release"

  # Staging is only available in non-prod environments.
  if [[ "$ENV" != "prod" ]]; then
    ropt append option --value "staging" --label "Staging build"
  fi
ropt pop

result=$(ropt execute --format=json)

ropt end

target=$(echo "$result" | jq -r '.target.value')
echo "Building target: $target (env=$ENV)"
