#!/usr/bin/env bash
# Example 6: Text Input and Flag Collection
#
# Demonstrates input types (string, number, email, path) and flags.
# Shows how --validate-regex and --default-value work, and how to use the
# JSON output format when you want structured access to results.

set -euo pipefail

export ROPT_SESSION=$(ropt begin)

# Plain string — required, no default
ropt push input --name "project-name" \
  --type string \
  --description "Project name (letters, numbers, hyphens only)" \
  --validate-regex '^[a-zA-Z0-9-]+$'
ropt pop

# Email input
ropt push input --name "owner-email" \
  --type email \
  --description "Owner email address"
ropt pop

# Path to an output directory — default provided so user can just hit Enter
ropt push input --name "output-dir" \
  --type path \
  --description "Output directory" \
  --default-value "./dist"
ropt pop

# Numeric input with bounds
ropt push input --name "workers" \
  --type number \
  --description "Number of parallel workers" \
  --default-value "4" \
  --validate-min 1 \
  --validate-max 64
ropt pop

# Boolean flags — each becomes a yes/no prompt
ropt append flag --name "verbose"   --description "Enable verbose logging?"
ropt append flag --name "dry-run"   --description "Dry run (no side effects)?"
ropt append flag --name "overwrite" --description "Overwrite existing output?"

# Capture as JSON so we can use jq for structured access
result=$(ropt execute --format=json)

ropt end

# Parse individual fields from JSON output
project=$(echo "$result"  | jq -r '.input_project_name')
email=$(echo "$result"    | jq -r '.input_owner_email')
outdir=$(echo "$result"   | jq -r '.input_output_dir')
workers=$(echo "$result"  | jq -r '.input_workers')
verbose=$(echo "$result"  | jq -r '.flag_verbose')
dry_run=$(echo "$result"  | jq -r '.flag_dry_run')
overwrite=$(echo "$result" | jq -r '.flag_overwrite')

echo ""
echo "--- Project configuration ---"
echo "  Name:      $project"
echo "  Owner:     $email"
echo "  Output:    $outdir"
echo "  Workers:   $workers"
echo "  Verbose:   $verbose"
echo "  Dry run:   $dry_run"
echo "  Overwrite: $overwrite"
echo ""

if [[ "$dry_run" == "true" ]]; then
  echo "[dry-run] Would build '$project' → $outdir with $workers workers."
else
  echo "Building '$project' → $outdir with $workers workers..."
  # ./build.sh --project "$project" --output "$outdir" --workers "$workers"
fi
