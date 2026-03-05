#!/usr/bin/env bash
# Example 1: Simple Interactive Menu
#
# The most basic ropt usage: present a picklist and act on the result.
# No arguments, no conditions — just a menu.
#
# Key point: give the select a --name so the shell variable is "action",
# not "0" (bare integers are invalid bash variable names).

set -euo pipefail

export ROPT_SESSION=$(ropt begin)

ropt push select --name "action" --message "What would you like to do?" --render=picklist
  ropt append option --value "deploy"   --label "Deploy application"
  ropt append option --value "rollback" --label "Rollback to previous version"
  ropt append option --value "status"   --label "Check deployment status"
  ropt append option --value "quit"     --label "Exit"
ropt pop

# Execute prompts and source results as shell variables.
# Capture first so set -e can see ropt's exit code; eval of an empty string
# always returns 0 and would silently swallow failures.
# With --prefix=ropt_, the variable is: ropt_action='deploy'
_ropt_out=$(ropt execute --format=sh --prefix=ropt_)
eval "$_ropt_out"

ropt end

case "$ropt_action" in
  deploy)
    echo "Deploying application..."
    # ./deploy.sh
    ;;
  rollback)
    echo "Rolling back..."
    # ./rollback.sh
    ;;
  status)
    echo "Checking status..."
    # ./status.sh
    ;;
  quit)
    echo "Goodbye."
    exit 0
    ;;
esac
