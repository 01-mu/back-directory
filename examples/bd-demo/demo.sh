#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
export PATH="$SCRIPT_DIR:$PATH"

DEMO_HOME=$(cd -- "$SCRIPT_DIR/../.." && pwd -P)

# Ensure bd is available in this bash session.
if [[ -f "$SCRIPT_DIR/../../scripts/bd.bash" ]]; then
  # shellcheck source=/dev/null
  source "$SCRIPT_DIR/../../scripts/bd.bash"
fi

if [[ -x "$SCRIPT_DIR/../../target/release/bd-core" ]]; then
  export BD_CORE_BIN="$SCRIPT_DIR/../../target/release/bd-core"
fi

# shellcheck source=/dev/null
source "$SCRIPT_DIR/demo-magic.sh"

TYPE_SPEED=32
NO_WAIT=true
PROMPT_TIMEOUT=0

pause() {
  sleep "$1"
}

show_pwd() {
  p "pwd"
  run_cmd "pwd | sed \"s|$DEMO_HOME|~|\""
}

show_pwd
pause 0.7

run_cmd "mkdir -p workspace/project/frontend/app/dashboard/settings/profile workspace/project/frontend/app/dashboard/settings/security workspace/project/backend/src/domain/user workspace/project/backend/src/domain/order workspace/project/infra/envs/prod"
pause 0.7

pe "cd workspace/project"
_bd_record >/dev/null 2>&1
pause 0.4
pe "cd frontend/app/dashboard/settings/profile"
_bd_record >/dev/null 2>&1
pause 0.4
show_pwd
pause 0.7

pe "cd ../../../../../infra/envs/prod"
_bd_record >/dev/null 2>&1
pause 0.4
show_pwd
pause 0.7

pe "bd 4"
_bd_record >/dev/null 2>&1
pause 0.4
show_pwd
pause 0.7

pe "bd 2"
pause 0.4
show_pwd
pause 0.7
pe "bd c"
_bd_record >/dev/null 2>&1
pause 0.4
show_pwd
pause 0.7

pe "bd 3"
_bd_record >/dev/null 2>&1
pause 1.2

pe "exit"
