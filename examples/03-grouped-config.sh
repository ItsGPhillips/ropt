#!/usr/bin/env bash
# Example 3: Multi-Step Configuration with Grouped Options
#
# Demonstrates:
#   - Multiple sequential prompts (input, select with groups, flag)
#   - `group` to visually organise related options inside a select
#   - `--render=input` for filterable type-ahead selection
#   - Sourcing all results at once with eval + --format=sh

set -euo pipefail

export ROPT_SESSION=$(ropt begin)

# Step 1: free-text input for the database hostname
ropt push input --name "db-host" --type string --description "Database hostname"
ropt pop

# Step 2: pick the database engine, organised into groups
ropt push select --name "engine" --message "Database engine" --render=input
  ropt push group --label "Relational"
    ropt append option --value "postgres"  --label "PostgreSQL"
    ropt append option --value "mysql"     --label "MySQL"
    ropt append option --value "sqlite"    --label "SQLite"
  ropt pop
  ropt push group --label "NoSQL"
    ropt append option --value "mongodb"   --label "MongoDB"
    ropt append option --value "redis"     --label "Redis"
    ropt append option --value "dynamodb"  --label "AWS DynamoDB"
  ropt pop
ropt pop

# Step 3: boolean flag — yes/no prompt
ropt append flag --name "ssl" --description "Enable SSL/TLS?"

# Step 4: numeric input with a sensible default
ropt push input --name "port" --type number --default-value "5432" --description "Port number"
ropt pop

# Collect all answers at once and expose as shell variables:
#   ropt_db_host, ropt_engine, ropt_ssl, ropt_port
_ropt_out=$(ropt execute --format=sh --prefix=ropt_)
eval "$_ropt_out"

ropt end

echo ""
echo "--- Connection summary ---"
echo "  Host:   $ropt_db_host"
echo "  Engine: $ropt_engine"
echo "  SSL:    $ropt_ssl"
echo "  Port:   $ropt_port"
echo ""
echo "Connection string: ${ropt_engine}://${ropt_db_host}:${ropt_port}?ssl=${ropt_ssl}"
