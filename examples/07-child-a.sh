#!/usr/bin/env bash
# Example 7 (child A): Component selector
#
# This script adds its prompts to whatever session it inherited.
# It must NOT call `ropt begin` or `ropt end` — the parent owns the lifecycle.
#
# If run standalone (no ROPT_SESSION), ropt will error with a clear message.

set -euo pipefail

ropt push select --name "component" --message "Which component to deploy?" --render=picklist
  ropt append option --value "api"      --label "API server"
  ropt append option --value "worker"   --label "Background worker"
  ropt append option --value "frontend" --label "Frontend assets"
  ropt append option --value "all"      --label "Everything"
ropt pop
