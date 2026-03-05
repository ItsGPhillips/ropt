#!/usr/bin/env bash
# Example 4: Dynamic Options from an Array (loop)
#
# Demonstrates building the option list at runtime from data — an array,
# a file listing, command output, etc.  ropt just sees individual
# `append option` calls; the loop is plain shell.
#
# Three techniques shown:
#   a) Static array
#   b) Command output (git branches)
#   c) Filesystem listing (config files)

set -euo pipefail

export ROPT_SESSION=$(ropt begin)

# --- (a) Region picker from a hardcoded array ---
REGIONS=("us-east-1" "us-west-2" "eu-west-1" "eu-central-1" "ap-southeast-1" "ap-northeast-1")

ropt push select --name "region" --message "Deployment region" --render=picklist
  for region in "${REGIONS[@]}"; do
    ropt append option --value "$region"
  done
ropt pop

# --- (b) Git branch picker (falls back gracefully if not in a git repo) ---
ropt push select --name "branch" --message "Target branch" --render=input
  if git rev-parse --git-dir &>/dev/null; then
    while IFS= read -r branch; do
      branch="${branch#  }"   # strip leading spaces / asterisk
      branch="${branch#\* }"
      [[ -n "$branch" ]] && ropt append option --value "$branch"
    done < <(git branch 2>/dev/null)
  else
    ropt append option --value "main"    --label "main (default)"
    ropt append option --value "develop" --label "develop (default)"
  fi
ropt pop

# --- (c) Config file picker from a directory ---
CONFIG_DIR="${CONFIG_DIR:-/etc}"
ropt push select --name "config" --message "Load config file" --render=input
  while IFS= read -r -d '' f; do
    name=$(basename "$f")
    ropt append option --value "$f" --label "$name"
  done < <(find "$CONFIG_DIR" -maxdepth 1 -name "*.conf" -print0 2>/dev/null)

  # Ensure at least one option if the dir is empty / unreadable
  if ! find "$CONFIG_DIR" -maxdepth 1 -name "*.conf" -print -quit 2>/dev/null | grep -q .; then
    ropt append option --value "none" --label "(no .conf files found)"
  fi
ropt pop

_ropt_out=$(ropt execute --format=sh --prefix=ropt_)
eval "$_ropt_out"

ropt end

echo ""
echo "Region:  $ropt_region"
echo "Branch:  $ropt_branch"
echo "Config:  $ropt_config"
