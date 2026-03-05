#!/usr/bin/env bash
# Example 7 (child B): Log level + notification flag
#
# Another child that contributes prompts to the inherited session.

set -euo pipefail

ropt push select --name "log_level" --message "Log level" --render=picklist
  ropt append option --value "error" --label "Errors only"
  ropt append option --value "warn"  --label "Warnings and errors"   --default
  ropt append option --value "info"  --label "Info"
  ropt append option --value "debug" --label "Debug (verbose)"
ropt pop

ropt append flag --name "notify" --description "Send Slack notification on completion?"
