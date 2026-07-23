#!/usr/bin/env bash
# Usage: hyprland-gate.sh <scenario>|all
# Requires a live Hyprland session plus hyprctl, grim, zenity, and hyprpilot.
# HYPRPILOT_BIN overrides the default target/release/hyprpilot binary.

set -euo pipefail

readonly SCRIPT_PATH=${BASH_SOURCE[0]}
if [[ ${SCRIPT_PATH} == */* ]]; then
	script_dir=${SCRIPT_PATH%/*}
else
	script_dir=.
fi
SCRIPT_DIR=$(cd -- "${script_dir}" && pwd -P)
readonly SCRIPT_DIR
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/../../.." && pwd -P)
readonly REPO_ROOT
if [[ ${HYPRPILOT_BIN+x} ]]; then
	readonly HYPRPILOT=${HYPRPILOT_BIN}
else
	readonly HYPRPILOT="${REPO_ROOT}/target/release/hyprpilot"
fi

SCENARIOS=()
SCENARIO_LINES=()

fail() {
	printf 'FAIL: %s\n' "$*"
}

skip() {
	printf 'SKIP: %s\n' "$*"
	exit 0
}

read_active_address() {
	local destination=$1
	local label=$2
	local raw compact
	local address_re='"address"[[:space:]]*:[[:space:]]*"([^"]*)"'
	local null_address_re='"address"[[:space:]]*:[[:space:]]*null'

	if ! raw=$(hyprctl activewindow -j 2>&1); then
		fail "${label}: focus observe=erreur hyprctl (${raw}); attendu=adresse ou aucun focus"
		return 1
	fi

	compact=${raw//[[:space:]]/}
	if [[ -z ${compact} || ${compact} == "{}" || ${compact} == "Invalid" || ${compact} == "null" ]]; then
		printf -v "${destination}" '%s' ""
	elif [[ ${raw} =~ ${address_re} ]]; then
		printf -v "${destination}" '%s' "${BASH_REMATCH[1]}"
	elif [[ ${raw} =~ ${null_address_re} ]]; then
		printf -v "${destination}" '%s' ""
	else
		fail "${label}: focus observe=JSON sans adresse (${raw}); attendu=adresse ou aucun focus"
		return 1
	fi
}

read_cursor() {
	local x_destination=$1
	local y_destination=$2
	local label=$3
	local raw x y extra

	if ! raw=$(hyprctl cursorpos 2>&1); then
		fail "${label}: curseur observe=erreur hyprctl (${raw}); attendu=X, Y"
		return 1
	fi
	IFS=, read -r x y extra <<<"${raw}"
	x=${x//[[:space:]]/}
	y=${y//[[:space:]]/}
	if [[ -n ${extra:-} || ! ${x} =~ ^-?[0-9]+$ || ! ${y} =~ ^-?[0-9]+$ ]]; then
		fail "${label}: curseur observe=${raw}; attendu=deux coordonnees entieres X, Y"
		return 1
	fi
	printf -v "${x_destination}" '%s' "${x}"
	printf -v "${y_destination}" '%s' "${y}"
}

assert_output_absent() {
	local label=$1
	local monitors
	local output_re='"name"[[:space:]]*:[[:space:]]*"hyprpilot"'

	if ! monitors=$(hyprctl monitors -j 2>&1); then
		fail "${label}: monitors observes=erreur hyprctl (${monitors}); attendu=liste sans output hyprpilot"
		return 1
	fi
	if [[ ${monitors} =~ ${output_re} ]]; then
		fail "${label}: output observe=hyprpilot; attendu=absent"
		return 1
	fi
}

scenario_guard_click() (
	local scenario_tmp=""
	local cleanup_failed=0
	local before_focus after_focus before_x before_y after_x after_y
	local delta_x delta_y command_output cleanup_output
	local status_json monitors_json clients_json window_address
	local out_x out_y target_x target_y actual_x actual_y
	local status_window_re monitor_re client_re
	local active_json active_compact active_address active_x active_y
	local active_width active_height center_x center_y centered_x centered_y
	local active_window_re

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_guard_click() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! cleanup_output=$("${HYPRPILOT}" teardown --kill 2>&1); then
			if [[ ${cleanup_output} != *"no active session"* ]]; then
				fail "nettoyage guard_click: teardown observe=echec (${cleanup_output}); attendu=succes ou session deja demontee"
				cleanup_failed=1
			fi
		fi
		if ! assert_output_absent "nettoyage guard_click"; then
			cleanup_failed=1
		fi

		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-guard.* ]]; then
				fail "nettoyage guard_click: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage guard_click: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_guard_click EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	if [[ -z ${XDG_RUNTIME_DIR:-} ]]; then
		fail "guard_click: XDG_RUNTIME_DIR observe=vide; attendu=repertoire runtime"
		return 1
	fi
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-guard.XXXXXX"); then
		fail "guard_click: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi
	export TMPDIR=${scenario_tmp}

	if ! active_json=$(hyprctl activewindow -j 2>&1); then
		fail "precondition guard_click: fenetre active observe=erreur hyprctl (${active_json}); attendu=fenetre active ou aucune"
		return 1
	fi
	active_compact=${active_json//[[:space:]]/}
	if [[ -z ${active_compact} || ${active_compact} == "{}" || ${active_compact} == "Invalid" || ${active_compact} == "null" ]]; then
		skip "aucune fenêtre active pour établir un état restaurable"
	fi
	active_window_re='"address"[[:space:]]*:[[:space:]]*"([^"]+)"[^}]*"at"[[:space:]]*:[[:space:]]*\[[[:space:]]*(-?[0-9]+),[[:space:]]*(-?[0-9]+)[[:space:]]*\][^}]*"size"[[:space:]]*:[[:space:]]*\[[[:space:]]*([0-9]+),[[:space:]]*([0-9]+)'
	if [[ ! ${active_json} =~ ${active_window_re} ]]; then
		fail "precondition guard_click: fenetre active observe=geometrie invalide (${active_json}); attendu=address, at et size"
		return 1
	fi
	active_address=${BASH_REMATCH[1]}
	active_x=${BASH_REMATCH[2]}
	active_y=${BASH_REMATCH[3]}
	active_width=${BASH_REMATCH[4]}
	active_height=${BASH_REMATCH[5]}
	if [[ ! ${active_address} =~ ^0x[0-9a-fA-F]+$ ]]; then
		fail "precondition guard_click: adresse observe=${active_address}; attendu=adresse Hyprland 0x..."
		return 1
	fi
	center_x=$((active_x + active_width / 2))
	center_y=$((active_y + active_height / 2))

	# Avec follow_mouse=1, focus != fenetre sous le curseur n'est pas
	# restaurable : le warp final refocalise cette fenetre. Le scenario mesure
	# donc la restauration d'un etat coherent, pas ce cas limite.
	if ! command_output=$(hyprctl dispatch movecursor "${center_x}" "${center_y}" 2>&1); then
		fail "precondition guard_click: movecursor observe=echec (${command_output}); attendu=centre (${center_x}, ${center_y}) de ${active_address}"
		return 1
	fi
	if [[ ${command_output} != "ok" ]]; then
		fail "precondition guard_click: movecursor observe=${command_output}; attendu=ok vers (${center_x}, ${center_y})"
		return 1
	fi
	read_cursor centered_x centered_y "precondition guard_click" || return 1
	delta_x=$((centered_x - center_x))
	delta_y=$((centered_y - center_y))
	((delta_x < 0)) && delta_x=$((-delta_x))
	((delta_y < 0)) && delta_y=$((-delta_y))
	if ((delta_x > 1 || delta_y > 1)); then
		fail "precondition guard_click: curseur observe=(${centered_x}, ${centered_y}); attendu=(${center_x}, ${center_y}) +/-1 px par axe"
		return 1
	fi

	read_active_address before_focus "avant guard_click" || return 1
	read_cursor before_x before_y "avant guard_click" || return 1

	if ! command_output=$(
		"${HYPRPILOT}" session start \
			--app "zenity --entry --title=hyprpilot-e2e-guard" \
			--match-title hyprpilot-e2e-guard 2>&1
	); then
		fail "session start guard_click observe=echec (${command_output}); attendu=succes"
		return 1
	fi

	# Bequille E2E pour HP-04 : le placement garanti arrive dans une slice
	# ulterieure ; ce deplacement interieur restera alors inoffensif.
	if ! status_json=$("${HYPRPILOT}" status 2>&1); then
		fail "setup guard_click: status observe=echec (${status_json}); attendu=JSON de session"
		return 1
	fi
	status_window_re='"window"[[:space:]]*:[[:space:]]*\{[^}]*"address"[[:space:]]*:[[:space:]]*"([^"]+)"'
	if [[ ! ${status_json} =~ ${status_window_re} ]]; then
		fail "setup guard_click: adresse observe=absente dans status; attendu=status.window.address"
		return 1
	fi
	window_address=${BASH_REMATCH[1]}
	if [[ ! ${window_address} =~ ^0x[0-9a-fA-F]+$ ]]; then
		fail "setup guard_click: adresse observe=${window_address}; attendu=adresse Hyprland 0x..."
		return 1
	fi

	if ! monitors_json=$(hyprctl monitors -j 2>&1); then
		fail "setup guard_click: monitors observes=erreur hyprctl (${monitors_json}); attendu=output hyprpilot"
		return 1
	fi
	monitor_re='"name"[[:space:]]*:[[:space:]]*"hyprpilot"[^}]*"x"[[:space:]]*:[[:space:]]*(-?[0-9]+)[^}]*"y"[[:space:]]*:[[:space:]]*(-?[0-9]+)'
	if [[ ! ${monitors_json} =~ ${monitor_re} ]]; then
		fail "setup guard_click: geometrie output observe=absente; attendu=hyprpilot avec x/y entiers"
		return 1
	fi
	out_x=${BASH_REMATCH[1]}
	out_y=${BASH_REMATCH[2]}
	target_x=$((out_x + 50))
	target_y=$((out_y + 50))

	if ! command_output=$(
		hyprctl dispatch movewindowpixel \
			"exact ${target_x} ${target_y},address:${window_address}" 2>&1
	); then
		fail "setup guard_click: movewindowpixel observe=echec (${command_output}); attendu=ok vers (${target_x}, ${target_y})"
		return 1
	fi
	if [[ ${command_output} != "ok" ]]; then
		fail "setup guard_click: movewindowpixel observe=${command_output}; attendu=ok vers (${target_x}, ${target_y})"
		return 1
	fi

	if ! clients_json=$(hyprctl clients -j 2>&1); then
		fail "setup guard_click: clients observes=erreur hyprctl (${clients_json}); attendu=fenetre ${window_address}"
		return 1
	fi
	client_re="\"address\"[[:space:]]*:[[:space:]]*\"${window_address}\"[^}]*\"at\"[[:space:]]*:[[:space:]]*\\[[[:space:]]*(-?[0-9]+),[[:space:]]*(-?[0-9]+)"
	if [[ ! ${clients_json} =~ ${client_re} ]]; then
		fail "setup guard_click: position observe=absente pour ${window_address}; attendu=(${target_x}, ${target_y})"
		return 1
	fi
	actual_x=${BASH_REMATCH[1]}
	actual_y=${BASH_REMATCH[2]}
	if ((actual_x != target_x || actual_y != target_y)); then
		fail "setup guard_click: position observe=(${actual_x}, ${actual_y}); attendu=(${target_x}, ${target_y})"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" click 20 20 2>&1); then
		fail "click guard_click observe=echec (${command_output}); attendu=succes aux coordonnees relatives (20, 20)"
		return 1
	fi

	read_cursor after_x after_y "apres click guard_click" || return 1
	read_active_address after_focus "apres click guard_click" || return 1

	delta_x=$((after_x - before_x))
	delta_y=$((after_y - before_y))
	((delta_x < 0)) && delta_x=$((-delta_x))
	((delta_y < 0)) && delta_y=$((-delta_y))
	if ((delta_x > 1 || delta_y > 1)); then
		fail "restauration curseur guard_click: observe=(${after_x}, ${after_y}); attendu=(${before_x}, ${before_y}) +/-1 px par axe"
		return 1
	fi
	if [[ ${after_focus} != "${before_focus}" ]]; then
		fail "restauration focus guard_click: observe=${after_focus:-<aucun>}; attendu=${before_focus:-<aucun>}"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" teardown --kill 2>&1); then
		fail "teardown guard_click observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	assert_output_absent "teardown guard_click"
)

discover_scenarios() {
	local function_name declared_name line source index
	local -a function_names=()

	shopt -s extdebug
	mapfile -t function_names < <(compgen -A function scenario_)
	for function_name in "${function_names[@]}"; do
		read -r declared_name line source < <(declare -F "${function_name}")
		[[ ${declared_name} == "${function_name}" && ${source} == "${SCRIPT_PATH}" ]] || continue

		index=${#SCENARIOS[@]}
		while ((index > 0 && line < SCENARIO_LINES[index - 1])); do
			SCENARIOS[index]=${SCENARIOS[index - 1]}
			SCENARIO_LINES[index]=${SCENARIO_LINES[index - 1]}
			index=$((index - 1))
		done
		SCENARIOS[index]=${function_name#scenario_}
		SCENARIO_LINES[index]=${line}
	done
	shopt -u extdebug
}

usage() {
	local scenario

	printf 'Usage: %s <scenario>|all\n' "${0##*/}" >&2
	printf 'Scenarios:\n' >&2
	for scenario in "${SCENARIOS[@]}"; do
		printf '  %s\n' "${scenario}" >&2
	done
	printf '  all\n' >&2
}

preflight() {
	local binary

	[[ -n ${HYPRLAND_INSTANCE_SIGNATURE:-} ]] ||
		skip "HYPRLAND_INSTANCE_SIGNATURE est vide"
	for binary in hyprctl grim zenity; do
		command -v "${binary}" >/dev/null 2>&1 ||
			skip "binaire ${binary} absent du PATH"
	done
	[[ -x ${HYPRPILOT} ]] ||
		skip "binaire hyprpilot absent ou non executable: ${HYPRPILOT}; lancez cargo build --release -p hyprpilot"
}

main() {
	local requested scenario
	local failures=0
	local requested_known=0

	discover_scenarios
	if (($# != 1)); then
		usage
		return 2
	fi
	requested=$1
	for scenario in "${SCENARIOS[@]}"; do
		[[ ${scenario} == "${requested}" ]] && requested_known=1
	done
	if [[ ${requested} != "all" && ${requested_known} -eq 0 ]]; then
		printf 'Scenario inconnu: %s\n' "${requested}" >&2
		usage
		return 2
	fi

	preflight
	if [[ ${requested} != "all" ]]; then
		"scenario_${requested}"
		return
	fi

	for scenario in "${SCENARIOS[@]}"; do
		if "scenario_${scenario}"; then
			printf 'PASS: %s\n' "${scenario}"
		else
			printf 'FAIL: %s\n' "${scenario}"
			failures=1
		fi
	done
	return "${failures}"
}

main "$@"
