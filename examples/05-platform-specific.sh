#!/usr/bin/env bash
# Example 5: Platform-Specific Options via case Statement
#
# Demonstrates using a `case` statement to conditionally append options.
# The menu adapts to whatever OS the script runs on — ropt never inspects
# the condition; the shell resolves it before any ropt call is made.

set -euo pipefail

# Normalise the platform string: "linux", "darwin", or "unknown"
case "$OSTYPE" in
  linux*)  PLATFORM="linux"  ;;
  darwin*) PLATFORM="darwin" ;;
  *)       PLATFORM="unknown" ;;
esac

export ROPT_SESSION=$(ropt begin)

ropt push select --name "action" --message "Select an action for $PLATFORM" --render=picklist
  # Common actions available everywhere
  ropt append option --value "ping"       --label "Ping a host"
  ropt append option --value "show-disk"  --label "Show disk usage"

  # Platform-specific actions
  case "$PLATFORM" in
    linux)
      ropt append option --value "apt-update"        --label "apt update && apt upgrade"
      ropt append option --value "systemctl-restart"  --label "Restart a systemd service"
      ropt append option --value "journalctl-tail"    --label "Tail system journal"
      ;;
    darwin)
      ropt append option --value "brew-update"       --label "brew update && brew upgrade"
      ropt append option --value "launchctl-restart"  --label "Restart a launchd service"
      ropt append option --value "open-console"       --label "Open Console.app"
      ;;
    *)
      ropt append option --value "noop" --label "(no platform-specific actions)"
      ;;
  esac
ropt pop

_ropt_out=$(ropt execute --format=sh --prefix=ropt_)
eval "$_ropt_out"

ropt end

echo "Running action: $ropt_action (platform=$PLATFORM)"

case "$ropt_action" in
  ping)
    read -rp "Host to ping: " host
    ping -c 4 "$host"
    ;;
  show-disk)
    df -h
    ;;
  apt-update)
    sudo apt update && sudo apt upgrade -y
    ;;
  systemctl-restart)
    read -rp "Service name: " svc
    sudo systemctl restart "$svc"
    ;;
  journalctl-tail)
    journalctl -f
    ;;
  brew-update)
    brew update && brew upgrade
    ;;
  launchctl-restart)
    read -rp "Service label: " label
    launchctl kickstart -k "gui/$(id -u)/$label"
    ;;
  open-console)
    open -a Console
    ;;
  noop)
    echo "Nothing to do on this platform."
    ;;
esac
