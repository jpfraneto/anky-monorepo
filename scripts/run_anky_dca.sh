#!/usr/bin/env bash
set -euo pipefail

ROOT="/home/kithkui/anky"
LOCKFILE="/tmp/anky_dca.lock"
LOGFILE="${ROOT}/logs/anky_dca.log"
VENV="${ROOT}/.venv-dca/bin/python"
SCRIPT="${ROOT}/scripts/anky_dca_buy.py"

mkdir -p "${ROOT}/logs"
exec 9>"${LOCKFILE}"
flock -n 9 || exit 0

"${VENV}" "${SCRIPT}" >> "${LOGFILE}" 2>&1
