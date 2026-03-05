#!/usr/bin/env bash
# Example 7: Session Inheritance Across Scripts
#
# Demonstrates that ROPT_SESSION is just an environment variable.
# A parent script begins a session, and child scripts inherit it
# automatically — no plumbing required.
#
# Run this file directly; it will invoke 07-child-a.sh and 07-child-b.sh.

set -euo pipefail

# Begin the session in the parent — ROPT_SESSION is exported automatically.
export ROPT_SESSION=$(ropt begin)

echo "Session: $ROPT_SESSION"
echo ""

# Parent owns the first prompt
ropt push select --name "env" --message "Which environment?" --render=picklist
  ropt append option --value "dev"     --label "Development"
  ropt append option --value "staging" --label "Staging"
  ropt append option --value "prod"    --label "Production"
ropt pop

# Child scripts add their own prompts to the SAME session.
# They don't call `ropt begin` — they inherit $ROPT_SESSION.
"$(dirname "$0")/07-child-a.sh"
"$(dirname "$0")/07-child-b.sh"

# Single ropt execute covers all prompts defined by parent + both children.
_ropt_out=$(ropt execute --format=sh --prefix=ropt_)
eval "$_ropt_out"

ropt end

echo ""
echo "--- Collected answers ---"
echo "  Environment:  $ropt_env"        # from parent
echo "  Component:    $ropt_component"  # from child-a
echo "  Log level:    $ropt_log_level"  # from child-b
echo "  Notify:       $ropt_notify"     # from child-b
