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

assert_output_present() {
	local label=$1
	local monitors
	local output_re='"name"[[:space:]]*:[[:space:]]*"hyprpilot"'

	if ! monitors=$(hyprctl monitors -j 2>&1); then
		fail "${label}: monitors observes=erreur hyprctl (${monitors}); attendu=output hyprpilot"
		return 1
	fi
	if [[ ! ${monitors} =~ ${output_re} ]]; then
		fail "${label}: output observe=absent; attendu=hyprpilot present"
		return 1
	fi
}

find_client_address_by_title() {
	local destination=$1
	local wanted_title=$2
	local raw line current_address="" found_address=""

	raw=$(hyprctl clients 2>/dev/null) || return 1
	while IFS= read -r line; do
		if [[ ${line} =~ ^Window[[:space:]]+(0x)?([0-9a-fA-F]+)[[:space:]] ]]; then
			current_address=0x${BASH_REMATCH[2]}
		elif [[ ${line} =~ ^[[:space:]]*title:[[:space:]](.*)$ ]] &&
			[[ ${BASH_REMATCH[1]} == "${wanted_title}" ]]; then
			found_address=${current_address}
		fi
	done <<<"${raw}"
	[[ -n ${found_address} ]] || return 1
	printf -v "${destination}" '%s' "${found_address}"
}

read_client_state() {
	local address=$1
	local x_destination=$2
	local y_destination=$3
	local width_destination=$4
	local height_destination=$5
	local workspace_destination=$6
	local floating_destination=$7
	local monitor_destination=$8
	local label=$9
	local raw compact
	local client_re

	if ! raw=$(hyprctl clients -j 2>&1); then
		fail "${label}: clients observes=erreur hyprctl (${raw}); attendu=fenetre ${address}"
		return 1
	fi
	compact=${raw//[[:space:]]/}
	client_re="\"address\":\"${address}\"[^}]*\"at\":\\[(-?[0-9]+),(-?[0-9]+)\\][^}]*\"size\":\\[([0-9]+),([0-9]+)\\][^}]*\"workspace\":\\{[^}]*\"name\":\"([^\"]+)\"\\}[^}]*\"floating\":(true|false)[^}]*\"monitor\":(-?[0-9]+)"
	if [[ ! ${compact} =~ ${client_re} ]]; then
		fail "${label}: etat observe=absent ou invalide pour ${address}; attendu=at, size, workspace, floating, monitor"
		return 1
	fi
	printf -v "${x_destination}" '%s' "${BASH_REMATCH[1]}"
	printf -v "${y_destination}" '%s' "${BASH_REMATCH[2]}"
	printf -v "${width_destination}" '%s' "${BASH_REMATCH[3]}"
	printf -v "${height_destination}" '%s' "${BASH_REMATCH[4]}"
	printf -v "${workspace_destination}" '%s' "${BASH_REMATCH[5]}"
	printf -v "${floating_destination}" '%s' "${BASH_REMATCH[6]}"
	printf -v "${monitor_destination}" '%s' "${BASH_REMATCH[7]}"
}

read_monitor_origin() {
	local monitor_id=$1
	local x_destination=$2
	local y_destination=$3
	local label=$4
	local raw compact monitor_re

	if ! raw=$(hyprctl monitors -j 2>&1); then
		fail "${label}: monitors observes=erreur hyprctl (${raw}); attendu=monitor ${monitor_id}"
		return 1
	fi
	compact=${raw//[[:space:]]/}
	monitor_re="\"id\":${monitor_id},[^}]*\"x\":(-?[0-9]+),\"y\":(-?[0-9]+),"
	if [[ ! ${compact} =~ ${monitor_re} ]]; then
		fail "${label}: origine observe=absente pour monitor ${monitor_id}; attendu=x/y entiers"
		return 1
	fi
	printf -v "${x_destination}" '%s' "${BASH_REMATCH[1]}"
	printf -v "${y_destination}" '%s' "${BASH_REMATCH[2]}"
}

client_present() {
	local address=$1
	local raw compact

	raw=$(hyprctl clients -j 2>/dev/null) || return 2
	compact=${raw//[[:space:]]/}
	[[ ${compact} == *"\"address\":\"${address}\""* ]]
}

wait_client_gone() {
	local address=$1
	local label=$2
	local attempt status

	for ((attempt = 0; attempt < 30; attempt++)); do
		if client_present "${address}"; then
			sleep 0.1
			continue
		else
			status=$?
		fi
		if ((status == 1)); then
			return 0
		fi
		fail "${label}: clients observe=erreur hyprctl; attendu=liste permettant de verifier ${address}"
		return 1
	done
	fail "${label}: fenetre observe=${address} presente apres 3s; attendu=disparue"
	return 1
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
	local settle_focus settle_attempt stable_focus_reads=0

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

	# Au spawn froid, zenity GTK vole le focus au map et peut encore emettre
	# des evenements tardifs. Le guard doit mesurer un bureau stabilise.
	for ((settle_attempt = 0; settle_attempt < 50; settle_attempt++)); do
		read_active_address settle_focus "settle guard_click" || return 1
		if [[ ${settle_focus} == "${before_focus}" ]]; then
			((stable_focus_reads += 1))
			if ((stable_focus_reads == 5)); then
				break
			fi
		else
			stable_focus_reads=0
		fi
		sleep 0.1
	done
	if ((stable_focus_reads != 5)); then
		fail "settle guard_click: focus jamais stabilisé après session start; observe=${settle_focus:-<aucun>}; attendu=${before_focus} sur 5 lectures consecutives"
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

scenario_teardown_restore() (
	local cleanup_failed=0
	local zenity_pid="" window_address=""
	local title=hyprpilot-e2e-teardown-restore
	local command_output cleanup_output attempt
	local initial_x initial_y initial_width initial_height initial_workspace
	local initial_floating initial_monitor monitor_x monitor_y
	local target_x target_y target_width=520 target_height=300
	local restored_x restored_y restored_width restored_height restored_workspace
	local restored_floating
	local current_geometry previous_geometry="" floating_settled=0
	local delta_x delta_y position_settled=0

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_teardown_restore() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! cleanup_output=$("${HYPRPILOT}" teardown 2>&1); then
			if [[ ${cleanup_output} != *"no active session"* ]]; then
				fail "nettoyage teardown_restore: teardown observe=echec (${cleanup_output}); attendu=succes ou session deja demontee"
				cleanup_failed=1
			fi
		fi
		if [[ -n ${zenity_pid} ]] && kill -0 "${zenity_pid}" 2>/dev/null; then
			kill "${zenity_pid}" 2>/dev/null || cleanup_failed=1
			wait "${zenity_pid}" 2>/dev/null || true
		fi
		if ! assert_output_absent "nettoyage teardown_restore"; then
			cleanup_failed=1
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_teardown_restore EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	zenity --entry --title="${title}" >/dev/null 2>&1 &
	zenity_pid=$!
	for ((attempt = 0; attempt < 50; attempt++)); do
		if find_client_address_by_title window_address "${title}"; then
			break
		fi
		sleep 0.1
	done
	if [[ -z ${window_address} ]]; then
		fail "setup teardown_restore: fenetre observe=absente apres 5s; attendu=zenity ${title}"
		return 1
	fi

	read_client_state "${window_address}" initial_x initial_y initial_width initial_height \
		initial_workspace initial_floating initial_monitor "setup teardown_restore" || return 1
	if [[ ${initial_floating} != true ]]; then
		if ! command_output=$(hyprctl dispatch togglefloating "address:${window_address}" 2>&1) ||
			[[ ${command_output} != ok ]]; then
			fail "setup teardown_restore: togglefloating observe=${command_output}; attendu=ok"
			return 1
		fi
	fi

	# Le placement flottant Hyprland est asynchrone apres tiled→float et peut
	# ecraser un move trop precoce. Attendre deux geometries stables.
	for ((attempt = 0; attempt < 20; attempt++)); do
		read_client_state "${window_address}" initial_x initial_y initial_width initial_height \
			initial_workspace initial_floating initial_monitor "settle floating teardown_restore" ||
			return 1
		current_geometry=${initial_x},${initial_y},${initial_width},${initial_height}
		if [[ ${initial_floating} == true && ${current_geometry} == "${previous_geometry}" ]]; then
			floating_settled=1
			break
		fi
		if [[ ${initial_floating} == true ]]; then
			previous_geometry=${current_geometry}
		else
			previous_geometry=""
		fi
		sleep 0.15
	done
	if ((floating_settled == 0)); then
		fail "settle floating teardown_restore: observe=floating ${initial_floating}, geometrie (${initial_x}, ${initial_y}) ${initial_width}x${initial_height}; attendu=true et stable sur 2 lectures en 3s"
		return 1
	fi

	read_monitor_origin "${initial_monitor}" monitor_x monitor_y "setup teardown_restore" ||
		return 1
	target_x=$((monitor_x + 120))
	target_y=$((monitor_y + 140))
	if ! command_output=$(
		hyprctl dispatch movewindowpixel \
			"exact ${target_x} ${target_y},address:${window_address}" 2>&1
	) || [[ ${command_output} != ok ]]; then
		fail "setup teardown_restore: move observe=${command_output}; attendu=ok vers (${target_x}, ${target_y})"
		return 1
	fi
	if ! command_output=$(
		hyprctl dispatch resizewindowpixel \
			"exact ${target_width} ${target_height},address:${window_address}" 2>&1
	) || [[ ${command_output} != ok ]]; then
		fail "setup teardown_restore: resize observe=${command_output}; attendu=ok vers ${target_width}x${target_height}"
		return 1
	fi

	# Zenity GTK4 refuse le resize du compositeur : la geometrie relue est la
	# reference. Le resize du teardown reste couvert en unitaire ; cet E2E
	# couvre workspace et position.
	for ((attempt = 0; attempt < 20; attempt++)); do
		read_client_state "${window_address}" initial_x initial_y initial_width initial_height \
			initial_workspace initial_floating initial_monitor "settle position teardown_restore" ||
			return 1
		delta_x=$((initial_x - target_x))
		delta_y=$((initial_y - target_y))
		((delta_x < 0)) && delta_x=$((-delta_x))
		((delta_y < 0)) && delta_y=$((-delta_y))
		if ((delta_x <= 2 && delta_y <= 2)); then
			position_settled=1
			break
		fi
		sleep 0.15
	done
	if ((position_settled == 0)); then
		fail "settle position teardown_restore: observe=(${initial_x}, ${initial_y}); attendu=(${target_x}, ${target_y}) +/-2 px en 3s"
		return 1
	fi

	if ! command_output=$(
		"${HYPRPILOT}" session start --match-title "${title}" 2>&1
	); then
		fail "session start teardown_restore observe=echec (${command_output}); attendu=attachement reussi"
		return 1
	fi
	if ! command_output=$("${HYPRPILOT}" teardown 2>&1); then
		fail "teardown restore observe=echec (${command_output}); attendu=succes"
		return 1
	fi

	read_client_state "${window_address}" restored_x restored_y restored_width restored_height \
		restored_workspace restored_floating initial_monitor "apres teardown_restore" || return 1
	if [[ ${restored_workspace} != "${initial_workspace}" ]]; then
		fail "teardown restore workspace: observe=${restored_workspace}; attendu=${initial_workspace}"
		return 1
	fi
	if ((restored_x != initial_x || restored_y != initial_y ||
		restored_width != initial_width || restored_height != initial_height)); then
		fail "teardown restore geometrie: observe=(${restored_x}, ${restored_y}) ${restored_width}x${restored_height}; attendu=(${initial_x}, ${initial_y}) ${initial_width}x${initial_height}"
		return 1
	fi
	if [[ ${restored_floating} != true ]]; then
		fail "teardown restore floating: observe=${restored_floating}; attendu=true"
		return 1
	fi
	if ! kill -0 "${zenity_pid}" 2>/dev/null; then
		fail "teardown restore survie: process observe=disparu (${zenity_pid}); attendu=vivant"
		return 1
	fi
	assert_output_absent "teardown restore" || return 1

	kill "${zenity_pid}" 2>/dev/null || true
	wait "${zenity_pid}" 2>/dev/null || true
	zenity_pid=""
)

scenario_teardown_kill() (
	local cleanup_failed=0
	local title=hyprpilot-e2e-teardown-kill
	local command_output cleanup_output status_json spawned_pid="" window_address=""
	local status_window_re status_pid_re attempt process_gone=0

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_teardown_kill() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! cleanup_output=$("${HYPRPILOT}" teardown --kill 2>&1); then
			if [[ ${cleanup_output} != *"no active session"* ]]; then
				fail "nettoyage teardown_kill: teardown observe=echec (${cleanup_output}); attendu=succes ou session deja demontee"
				cleanup_failed=1
			fi
		fi
		if [[ -n ${spawned_pid} ]] && kill -0 -- "-${spawned_pid}" 2>/dev/null; then
			kill -- "-${spawned_pid}" 2>/dev/null || cleanup_failed=1
		fi
		if ! assert_output_absent "nettoyage teardown_kill"; then
			cleanup_failed=1
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_teardown_kill EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	if ! command_output=$(
		"${HYPRPILOT}" session start \
			--app "zenity --entry --title=${title}" \
			--match-title "${title}" 2>&1
	); then
		fail "session start teardown_kill observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	if ! status_json=$("${HYPRPILOT}" status 2>&1); then
		fail "status teardown_kill observe=echec (${status_json}); attendu=JSON de session"
		return 1
	fi
	status_window_re='"window"[[:space:]]*:[[:space:]]*\{[^}]*"address"[[:space:]]*:[[:space:]]*"([^"]+)"'
	status_pid_re='"spawned_pid"[[:space:]]*:[[:space:]]*([0-9]+)'
	if [[ ! ${status_json} =~ ${status_window_re} ]]; then
		fail "status teardown_kill: adresse observe=absente; attendu=status.window.address"
		return 1
	fi
	window_address=${BASH_REMATCH[1]}
	if [[ ! ${status_json} =~ ${status_pid_re} ]]; then
		fail "status teardown_kill: spawned_pid observe=absent; attendu=entier"
		return 1
	fi
	spawned_pid=${BASH_REMATCH[1]}

	if ! command_output=$("${HYPRPILOT}" teardown --kill 2>&1); then
		fail "teardown kill observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	for ((attempt = 0; attempt < 30; attempt++)); do
		if ! kill -0 -- "-${spawned_pid}" 2>/dev/null; then
			process_gone=1
			break
		fi
		sleep 0.1
	done
	if ((process_gone == 0)); then
		fail "teardown kill process: observe=groupe ${spawned_pid} present apres 3s; attendu=disparu"
		return 1
	fi
	if find_client_address_by_title window_address "${title}"; then
		fail "teardown kill fenetre: observe=${window_address}; attendu=disparue"
		return 1
	fi
	assert_output_absent "teardown kill"
)

scenario_teardown_corrupt() (
	local cleanup_failed=0
	local title=hyprpilot-e2e-teardown-corrupt
	local command_output status_json spawned_pid="" window_address="" session_file=""
	local status_window_re status_pid_re
	local before_x before_y before_width before_height before_workspace before_floating before_monitor
	local state_x state_y state_width state_height state_workspace state_floating state_monitor

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_teardown_corrupt() {
		local scenario_status=$?
		local monitors cleanup_output
		trap - EXIT INT TERM

		if [[ -n ${window_address} ]] && client_present "${window_address}"; then
			hyprctl dispatch closewindow "address:${window_address}" >/dev/null 2>&1 ||
				cleanup_failed=1
		fi
		if [[ -n ${spawned_pid} ]] && kill -0 -- "-${spawned_pid}" 2>/dev/null; then
			kill -- "-${spawned_pid}" 2>/dev/null || cleanup_failed=1
		fi
		if monitors=$(hyprctl monitors -j 2>/dev/null) &&
			[[ ${monitors} =~ \"name\"[[:space:]]*:[[:space:]]*\"hyprpilot\" ]]; then
			if ! cleanup_output=$(hyprctl output remove hyprpilot 2>&1) ||
				[[ ${cleanup_output} != ok ]]; then
				fail "nettoyage teardown_corrupt: output remove observe=${cleanup_output}; attendu=ok"
				cleanup_failed=1
			fi
		fi
		if [[ -n ${session_file} && -e ${session_file} ]]; then
			if [[ ${session_file} != "${XDG_RUNTIME_DIR}/hyprpilot/session.json" ]]; then
				fail "nettoyage teardown_corrupt: fichier observe=${session_file}; attendu=session runtime hyprpilot"
				cleanup_failed=1
			elif ! rm -- "${session_file}"; then
				cleanup_failed=1
			fi
		fi
		if ! assert_output_absent "nettoyage teardown_corrupt"; then
			cleanup_failed=1
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_teardown_corrupt EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	if [[ -z ${XDG_RUNTIME_DIR:-} ]]; then
		fail "teardown_corrupt: XDG_RUNTIME_DIR observe=vide; attendu=repertoire runtime"
		return 1
	fi
	if ! command_output=$(
		"${HYPRPILOT}" session start \
			--app "zenity --entry --title=${title}" \
			--match-title "${title}" 2>&1
	); then
		fail "session start teardown_corrupt observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	if ! status_json=$("${HYPRPILOT}" status 2>&1); then
		fail "status teardown_corrupt observe=echec (${status_json}); attendu=JSON de session"
		return 1
	fi
	status_window_re='"window"[[:space:]]*:[[:space:]]*\{[^}]*"address"[[:space:]]*:[[:space:]]*"([^"]+)"'
	status_pid_re='"spawned_pid"[[:space:]]*:[[:space:]]*([0-9]+)'
	if [[ ! ${status_json} =~ ${status_window_re} ]]; then
		fail "status teardown_corrupt: adresse observe=absente; attendu=status.window.address"
		return 1
	fi
	window_address=${BASH_REMATCH[1]}
	if [[ ! ${status_json} =~ ${status_pid_re} ]]; then
		fail "status teardown_corrupt: spawned_pid observe=absent; attendu=entier"
		return 1
	fi
	spawned_pid=${BASH_REMATCH[1]}
	read_client_state "${window_address}" before_x before_y before_width before_height \
		before_workspace before_floating before_monitor "avant corruption teardown_corrupt" ||
		return 1
	session_file=${XDG_RUNTIME_DIR}/hyprpilot/session.json
	if [[ ! -f ${session_file} ]]; then
		fail "corruption teardown_corrupt: fichier observe=absent (${session_file}); attendu=session.json"
		return 1
	fi
	printf '{broken\n' >"${session_file}"

	if command_output=$("${HYPRPILOT}" teardown 2>&1); then
		fail "teardown corrupt observe=succes (${command_output}); attendu=exit non nul"
		return 1
	fi
	if [[ ${command_output} != *"no output was removed"* ]]; then
		fail "teardown corrupt message observe=${command_output}; attendu=instruction no output was removed"
		return 1
	fi
	assert_output_present "teardown corrupt" || return 1
	read_client_state "${window_address}" state_x state_y state_width state_height state_workspace \
		state_floating state_monitor "teardown corrupt fenetre intacte" || return 1
	if ((state_x != before_x || state_y != before_y ||
		state_width != before_width || state_height != before_height)) ||
		[[ ${state_workspace} != "${before_workspace}" ||
			${state_floating} != "${before_floating}" ||
			${state_monitor} != "${before_monitor}" ]]; then
		fail "teardown corrupt fenetre: etat observe=(${state_x}, ${state_y}) ${state_width}x${state_height} workspace=${state_workspace} floating=${state_floating} monitor=${state_monitor}; attendu=(${before_x}, ${before_y}) ${before_width}x${before_height} workspace=${before_workspace} floating=${before_floating} monitor=${before_monitor}"
		return 1
	fi

	if ! command_output=$(
		hyprctl dispatch closewindow "address:${window_address}" 2>&1
	) || [[ ${command_output} != ok ]]; then
		fail "nettoyage manuel teardown_corrupt: closewindow observe=${command_output}; attendu=ok"
		return 1
	fi
	wait_client_gone "${window_address}" "nettoyage manuel teardown_corrupt" || return 1
	window_address=""
	if ! command_output=$(hyprctl output remove hyprpilot 2>&1) ||
		[[ ${command_output} != ok ]]; then
		fail "nettoyage manuel teardown_corrupt: output remove observe=${command_output}; attendu=ok"
		return 1
	fi
	if ! rm -- "${session_file}"; then
		fail "nettoyage manuel teardown_corrupt: rm observe=echec (${session_file}); attendu=fichier supprime"
		return 1
	fi
	session_file=""
	assert_output_absent "nettoyage manuel teardown_corrupt"
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
