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
	local raw found_address

	raw=$(hyprctl clients -j 2>/dev/null) || return 1
	found_address=$(
		jq -er --arg title "${wanted_title}" \
			'[.[] | select(.title == $title) | .address] | last // empty' <<<"${raw}"
	) || return 1
	printf -v "${destination}" '%s' "${found_address}"
}

wait_client_addresses_by_title() {
	local destination=$1
	local wanted_title=$2
	local expected_count=$3
	local label=$4
	local raw addresses count previous="" attempt stable_reads=0

	for ((attempt = 0; attempt < 50; attempt++)); do
		if ! raw=$(hyprctl clients -j 2>&1); then
			fail "${label}: clients observe=erreur hyprctl (${raw}); attendu=${expected_count} fenêtre(s)"
			return 1
		fi
		if ! addresses=$(
			jq -c --arg title "${wanted_title}" \
				'[.[] | select(.title == $title) | .address] | sort' <<<"${raw}"
		); then
			fail "${label}: clients observe=JSON invalide; attendu=tableau Hyprland filtrable"
			return 1
		fi
		count=$(jq 'length' <<<"${addresses}")
		if ((count == expected_count)); then
			if [[ ${addresses} == "${previous}" ]]; then
				((stable_reads += 1))
			else
				stable_reads=1
				previous=${addresses}
			fi
			if ((stable_reads == 2)); then
				printf -v "${destination}" '%s' "${addresses}"
				return 0
			fi
		else
			stable_reads=0
			previous=""
		fi
		sleep 0.1
	done
	fail "${label}: adresses observe=${addresses}; attendu=${expected_count} fenêtre(s) stables en 5s"
	return 1
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

read_stable_client_state() {
	local address=$1
	local x_destination=$2
	local y_destination=$3
	local width_destination=$4
	local height_destination=$5
	local workspace_destination=$6
	local floating_destination=$7
	local monitor_destination=$8
	local label=$9
	local observed_x observed_y observed_width observed_height observed_workspace
	local observed_floating observed_monitor current previous="" attempt

	for ((attempt = 0; attempt < 30; attempt++)); do
		read_client_state "${address}" observed_x observed_y observed_width observed_height \
			observed_workspace observed_floating observed_monitor "${label}" || return 1
		current=${observed_x},${observed_y},${observed_width},${observed_height},${observed_workspace},${observed_floating},${observed_monitor}
		if [[ ${current} == "${previous}" ]]; then
			printf -v "${x_destination}" '%s' "${observed_x}"
			printf -v "${y_destination}" '%s' "${observed_y}"
			printf -v "${width_destination}" '%s' "${observed_width}"
			printf -v "${height_destination}" '%s' "${observed_height}"
			printf -v "${workspace_destination}" '%s' "${observed_workspace}"
			printf -v "${floating_destination}" '%s' "${observed_floating}"
			printf -v "${monitor_destination}" '%s' "${observed_monitor}"
			return 0
		fi
		previous=${current}
		sleep 0.1
	done
	fail "${label}: état observe=${current}; attendu=deux lectures identiques en 3s"
	return 1
}

wait_client_state_equals() {
	local address=$1
	local expected_x=$2
	local expected_y=$3
	local expected_width=$4
	local expected_height=$5
	local expected_workspace=$6
	local expected_floating=$7
	local expected_monitor=$8
	local label=$9
	local actual_x actual_y actual_width actual_height actual_workspace
	local actual_floating actual_monitor attempt

	for ((attempt = 0; attempt < 30; attempt++)); do
		read_client_state "${address}" actual_x actual_y actual_width actual_height \
			actual_workspace actual_floating actual_monitor "${label}" || return 1
		if ((actual_x == expected_x && actual_y == expected_y &&
			actual_width == expected_width && actual_height == expected_height)) &&
			[[ ${actual_workspace} == "${expected_workspace}" &&
				${actual_floating} == "${expected_floating}" &&
				${actual_monitor} == "${expected_monitor}" ]]; then
			return 0
		fi
		sleep 0.1
	done
	fail "${label}: état observe=(${actual_x}, ${actual_y}) ${actual_width}x${actual_height}, workspace=${actual_workspace}, floating=${actual_floating}, monitor=${actual_monitor}; attendu=(${expected_x}, ${expected_y}) ${expected_width}x${expected_height}, workspace=${expected_workspace}, floating=${expected_floating}, monitor=${expected_monitor}"
	return 1
}

assert_parking_not_toggled() {
	local label=$1
	local monitors

	if ! monitors=$(hyprctl monitors -j 2>&1); then
		fail "${label}: monitors observes=erreur hyprctl (${monitors}); attendu=parking non affiché"
		return 1
	fi
	if ! jq -e --arg parking "special:hyprpilot-parked" \
		'all(.[]; (.specialWorkspace.name // "") != $parking)' \
		<<<"${monitors}" >/dev/null; then
		fail "${label}: special observe=hyprpilot-parked affiché; attendu=aucun toggle"
		return 1
	fi
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

read_output_geometry() {
	local x_destination=$1
	local y_destination=$2
	local width_destination=$3
	local height_destination=$4
	local label=$5
	local raw values x y width height extra

	if ! raw=$(hyprctl monitors -j 2>&1); then
		fail "${label}: monitors observe=erreur hyprctl (${raw}); attendu=output hyprpilot"
		return 1
	fi
	if ! values=$(
		jq -er '
			[.[] | select(.name == "hyprpilot")]
			| select(length == 1)
			| .[0]
			| [.x, .y, .width, .height]
			| select(all(.[]; type == "number" and floor == .))
			| @tsv
		' <<<"${raw}"
	); then
		fail "${label}: geometrie observe=absente ou invalide; attendu=x/y/width/height entiers"
		return 1
	fi
	IFS=$'\t' read -r x y width height extra <<<"${values}"
	if [[ -n ${extra:-} || ! ${x} =~ ^-?[0-9]+$ || ! ${y} =~ ^-?[0-9]+$ ||
		! ${width} =~ ^[1-9][0-9]*$ || ! ${height} =~ ^[1-9][0-9]*$ ]]; then
		fail "${label}: geometrie observe=${values}; attendu=x/y entiers et taille positive"
		return 1
	fi
	printf -v "${x_destination}" '%s' "${x}"
	printf -v "${y_destination}" '%s' "${y}"
	printf -v "${width_destination}" '%s' "${width}"
	printf -v "${height_destination}" '%s' "${height}"
}

read_png_size() {
	local path=$1
	local width_destination=$2
	local height_destination=$3
	local label=$4
	local width height
	local -a bytes=()

	if [[ ! -s ${path} ]]; then
		fail "${label}: PNG observe=absent ou vide (${path}); attendu=image non vide"
		return 1
	fi
	read -r -a bytes < <(od -An -t u1 -j 16 -N 8 -- "${path}")
	if ((${#bytes[@]} != 8)); then
		fail "${label}: IHDR observe=illisible (${path}); attendu=8 octets de dimensions PNG"
		return 1
	fi
	width=$(((bytes[0] << 24) | (bytes[1] << 16) | (bytes[2] << 8) | bytes[3]))
	height=$(((bytes[4] << 24) | (bytes[5] << 16) | (bytes[6] << 8) | bytes[7]))
	if ((width <= 0 || height <= 0)); then
		fail "${label}: taille observe=${width}x${height}; attendu=dimensions PNG positives"
		return 1
	fi
	printf -v "${width_destination}" '%s' "${width}"
	printf -v "${height_destination}" '%s' "${height}"
}

read_layout_right_bound() {
	local destination=$1
	local label=$2
	local raw compact remaining width height x right
	local found=0 max_right=-2147483648
	local monitor_re='"width":([0-9]+),"height":([0-9]+),[^}]*"x":(-?[0-9]+),"y":-?[0-9]+,'

	if ! raw=$(hyprctl monitors -j 2>&1); then
		fail "${label}: monitors observes=erreur hyprctl (${raw}); attendu=geometrie du layout"
		return 1
	fi
	compact=${raw//[[:space:]]/}
	remaining=${compact}
	while [[ ${remaining} =~ ${monitor_re} ]]; do
		width=${BASH_REMATCH[1]}
		height=${BASH_REMATCH[2]}
		x=${BASH_REMATCH[3]}
		# width+height couvre conservativement les outputs transformes.
		right=$((x + width + height))
		((right > max_right)) && max_right=${right}
		found=1
		remaining=${remaining#*\"x\":${x},}
	done
	if ((found == 0)); then
		fail "${label}: geometrie observe=illisible (${raw}); attendu=au moins un monitor"
		return 1
	fi
	printf -v "${destination}" '%s' "${max_right}"
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

scenario_start_offscreen() (
	local cleanup_failed=0
	local scenario_tmp="" zenity_pid="" window_address=""
	local title="hyprpilot-e2e-start-offscreen-$$"
	local command_output cleanup_output shot_output
	local user_address user_x user_y user_width user_height user_workspace
	local user_floating user_monitor center_x center_y
	local window_x window_y window_width window_height window_workspace
	local window_floating window_monitor current_geometry previous_geometry=""
	local layout_right target_x target_y offscreen_gap=100000
	local origin_x origin_y origin_width origin_height origin_workspace
	local origin_floating origin_monitor
	local restored_x restored_y restored_width restored_height restored_workspace
	local restored_floating restored_monitor
	local before_focus after_focus before_x before_y after_x after_y
	local settle_focus settle_x settle_y stable_reads=0
	local delta_x delta_y attempt floating_settled=0 position_settled=0 restored=0

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_start_offscreen() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! cleanup_output=$("${HYPRPILOT}" teardown 2>&1); then
			if [[ ${cleanup_output} != *"no active session"* ]]; then
				fail "nettoyage start_offscreen: teardown observe=echec (${cleanup_output}); attendu=succes ou session deja demontee"
				cleanup_failed=1
			fi
		fi
		if [[ -n ${zenity_pid} ]] && kill -0 "${zenity_pid}" 2>/dev/null; then
			kill "${zenity_pid}" 2>/dev/null || cleanup_failed=1
			wait "${zenity_pid}" 2>/dev/null || true
		fi
		if ! assert_output_absent "nettoyage start_offscreen"; then
			cleanup_failed=1
		fi
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-start-offscreen.* ]]; then
				fail "nettoyage start_offscreen: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage start_offscreen: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_start_offscreen EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	if [[ -z ${XDG_RUNTIME_DIR:-} ]]; then
		fail "start_offscreen: XDG_RUNTIME_DIR observe=vide; attendu=repertoire runtime"
		return 1
	fi
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-start-offscreen.XXXXXX"); then
		fail "start_offscreen: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi

	read_active_address user_address "precondition start_offscreen" || return 1
	if [[ -z ${user_address} ]]; then
		skip "aucune fenêtre active pour établir un état restaurable"
	fi
	read_client_state "${user_address}" user_x user_y user_width user_height user_workspace \
		user_floating user_monitor "precondition start_offscreen" || return 1
	center_x=$((user_x + user_width / 2))
	center_y=$((user_y + user_height / 2))

	zenity --entry --title="${title}" >/dev/null 2>&1 &
	zenity_pid=$!
	for ((attempt = 0; attempt < 50; attempt++)); do
		if find_client_address_by_title window_address "${title}"; then
			break
		fi
		sleep 0.1
	done
	if [[ -z ${window_address} ]]; then
		fail "setup start_offscreen: fenetre observe=absente apres 5s; attendu=zenity ${title}"
		return 1
	fi

	read_client_state "${window_address}" window_x window_y window_width window_height \
		window_workspace window_floating window_monitor "setup start_offscreen" || return 1
	if [[ ${window_floating} != true ]]; then
		if ! command_output=$(hyprctl dispatch togglefloating "address:${window_address}" 2>&1) ||
			[[ ${command_output} != ok ]]; then
			fail "setup start_offscreen: togglefloating observe=${command_output}; attendu=ok"
			return 1
		fi
	fi
	for ((attempt = 0; attempt < 20; attempt++)); do
		read_client_state "${window_address}" window_x window_y window_width window_height \
			window_workspace window_floating window_monitor "settle floating start_offscreen" ||
			return 1
		current_geometry=${window_x},${window_y},${window_width},${window_height}
		if [[ ${window_floating} == true && ${current_geometry} == "${previous_geometry}" ]]; then
			floating_settled=1
			break
		fi
		if [[ ${window_floating} == true ]]; then
			previous_geometry=${current_geometry}
		else
			previous_geometry=""
		fi
		sleep 0.15
	done
	if ((floating_settled == 0)); then
		fail "settle floating start_offscreen: observe=floating ${window_floating}, geometrie (${window_x}, ${window_y}) ${window_width}x${window_height}; attendu=true et stable sur 2 lectures en 3s"
		return 1
	fi

	read_layout_right_bound layout_right "setup start_offscreen" || return 1
	target_x=$((layout_right + offscreen_gap))
	target_y=${window_y}
	if ! command_output=$(
		hyprctl dispatch movewindowpixel \
			"exact ${target_x} ${target_y},address:${window_address}" 2>&1
	) || [[ ${command_output} != ok ]]; then
		fail "setup start_offscreen: movewindowpixel observe=${command_output}; attendu=ok vers (${target_x}, ${target_y})"
		return 1
	fi
	for ((attempt = 0; attempt < 20; attempt++)); do
		read_client_state "${window_address}" window_x window_y window_width window_height \
			window_workspace window_floating window_monitor "settle offscreen start_offscreen" ||
			return 1
		delta_x=$((window_x - target_x))
		delta_y=$((window_y - target_y))
		((delta_x < 0)) && delta_x=$((-delta_x))
		((delta_y < 0)) && delta_y=$((-delta_y))
		if ((delta_x <= 2 && delta_y <= 2)); then
			position_settled=1
			break
		fi
		sleep 0.15
	done
	if ((position_settled == 0)); then
		fail "settle offscreen start_offscreen: observe=(${window_x}, ${window_y}); attendu=(${target_x}, ${target_y}) +/-2 px en 3s"
		return 1
	fi
	read_client_state "${window_address}" origin_x origin_y origin_width origin_height \
		origin_workspace origin_floating origin_monitor "origine start_offscreen" || return 1

	if ! command_output=$(
		hyprctl dispatch focuswindow "address:${user_address}" 2>&1
	) || [[ ${command_output} != ok ]]; then
		fail "precondition start_offscreen: focuswindow observe=${command_output}; attendu=ok vers ${user_address}"
		return 1
	fi
	if ! command_output=$(hyprctl dispatch movecursor "${center_x}" "${center_y}" 2>&1) ||
		[[ ${command_output} != ok ]]; then
		fail "precondition start_offscreen: movecursor observe=${command_output}; attendu=ok vers (${center_x}, ${center_y})"
		return 1
	fi
	for ((attempt = 0; attempt < 50; attempt++)); do
		read_active_address settle_focus "settle start_offscreen" || return 1
		read_cursor settle_x settle_y "settle start_offscreen" || return 1
		delta_x=$((settle_x - center_x))
		delta_y=$((settle_y - center_y))
		((delta_x < 0)) && delta_x=$((-delta_x))
		((delta_y < 0)) && delta_y=$((-delta_y))
		if [[ ${settle_focus} == "${user_address}" ]] && ((delta_x <= 1 && delta_y <= 1)); then
			((stable_reads += 1))
			if ((stable_reads == 5)); then
				break
			fi
		else
			stable_reads=0
		fi
		sleep 0.1
	done
	if ((stable_reads != 5)); then
		fail "settle start_offscreen: observe=focus ${settle_focus:-<aucun>}, curseur (${settle_x}, ${settle_y}); attendu=${user_address} et (${center_x}, ${center_y}) +/-1 sur 5 lectures consecutives"
		return 1
	fi
	read_active_address before_focus "avant start_offscreen" || return 1
	read_cursor before_x before_y "avant start_offscreen" || return 1

	if ! command_output=$(
		"${HYPRPILOT}" session start --match-title "${title}" 2>&1
	); then
		fail "session start start_offscreen observe=echec (${command_output}); attendu=ready"
		return 1
	fi
	if [[ ${command_output} != *"ready"* ]]; then
		fail "session start start_offscreen observe=${command_output}; attendu=message ready"
		return 1
	fi
	if ! shot_output=$(
		"${HYPRPILOT}" shot start-offscreen --out "${scenario_tmp}" 2>&1
	); then
		fail "shot immediat start_offscreen observe=echec (${shot_output}); attendu=PNG capturable apres ready"
		return 1
	fi
	if [[ ! -s ${scenario_tmp}/start-offscreen.png ]]; then
		fail "shot immediat start_offscreen observe=${shot_output}; attendu=${scenario_tmp}/start-offscreen.png non vide"
		return 1
	fi

	read_active_address after_focus "apres shot start_offscreen" || return 1
	read_cursor after_x after_y "apres shot start_offscreen" || return 1
	delta_x=$((after_x - before_x))
	delta_y=$((after_y - before_y))
	((delta_x < 0)) && delta_x=$((-delta_x))
	((delta_y < 0)) && delta_y=$((-delta_y))
	if [[ ${after_focus} != "${before_focus}" ]] || ((delta_x > 1 || delta_y > 1)); then
		fail "invariant start_offscreen apres shot: observe=focus ${after_focus:-<aucun>}, curseur (${after_x}, ${after_y}); attendu=focus ${before_focus:-<aucun>}, curseur (${before_x}, ${before_y})"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" teardown 2>&1); then
		fail "teardown start_offscreen observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	for ((attempt = 0; attempt < 20; attempt++)); do
		read_client_state "${window_address}" restored_x restored_y restored_width restored_height \
			restored_workspace restored_floating restored_monitor "restore start_offscreen" ||
			return 1
		if ((restored_x == origin_x && restored_y == origin_y &&
			restored_width == origin_width && restored_height == origin_height)) &&
			[[ ${restored_workspace} == "${origin_workspace}" &&
				${restored_floating} == "${origin_floating}" &&
				${restored_monitor} == "${origin_monitor}" ]]; then
			restored=1
			break
		fi
		sleep 0.15
	done
	if ((restored == 0)); then
		fail "teardown start_offscreen geometrie: observe=(${restored_x}, ${restored_y}) ${restored_width}x${restored_height}, workspace=${restored_workspace}, floating=${restored_floating}, monitor=${restored_monitor}; attendu=(${origin_x}, ${origin_y}) ${origin_width}x${origin_height}, workspace=${origin_workspace}, floating=${origin_floating}, monitor=${origin_monitor}"
		return 1
	fi
	# Hyprland recentre le curseur sur le moniteur restant lors de
	# `output remove`; ce comportement verifie est hors du contrat T3.
	read_active_address after_focus "apres teardown start_offscreen" || return 1
	if [[ ${after_focus} != "${before_focus}" ]]; then
		fail "invariant start_offscreen apres teardown: focus observe=${after_focus:-<aucun>}; attendu=${before_focus:-<aucun>}"
		return 1
	fi
	assert_output_absent "teardown start_offscreen" || return 1

	kill "${zenity_pid}" 2>/dev/null || true
	wait "${zenity_pid}" 2>/dev/null || true
	zenity_pid=""
)

scenario_windows_ambiguous() (
	local cleanup_failed=0
	local zenity_one_pid="" zenity_two_pid=""
	local title=hyprpilot-e2e-ambiguous
	local addresses_json="" command_output="" cleanup_output="" last_line=""
	local windows_json="" active_address="" session_file=""

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_windows_ambiguous() {
		local scenario_status=$?
		local pid
		trap - EXIT INT TERM

		if [[ -n ${session_file} && -e ${session_file} ]]; then
			if ! cleanup_output=$("${HYPRPILOT}" teardown 2>&1); then
				fail "nettoyage windows_ambiguous: teardown observe=echec (${cleanup_output}); attendu=succes"
				cleanup_failed=1
			fi
		fi
		for pid in "${zenity_one_pid}" "${zenity_two_pid}"; do
			if [[ -n ${pid} ]] && kill -0 "${pid}" 2>/dev/null; then
				kill "${pid}" 2>/dev/null || cleanup_failed=1
				wait "${pid}" 2>/dev/null || true
			fi
		done
		if ! assert_output_absent "nettoyage windows_ambiguous"; then
			cleanup_failed=1
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_windows_ambiguous EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	if [[ -z ${XDG_RUNTIME_DIR:-} ]]; then
		fail "windows_ambiguous: XDG_RUNTIME_DIR observe=vide; attendu=repertoire runtime"
		return 1
	fi
	session_file=${XDG_RUNTIME_DIR}/hyprpilot/session.json
	if [[ -e ${session_file} ]]; then
		fail "precondition windows_ambiguous: session observe=presente (${session_file}); attendu=absente"
		return 1
	fi
	assert_output_absent "precondition windows_ambiguous" || return 1

	zenity --entry --title="${title}" >/dev/null 2>&1 &
	zenity_one_pid=$!
	wait_client_addresses_by_title addresses_json "${title}" 1 \
		"settle premier spawn windows_ambiguous" || return 1

	zenity --entry --title="${title}" >/dev/null 2>&1 &
	zenity_two_pid=$!
	wait_client_addresses_by_title addresses_json "${title}" 2 \
		"settle second spawn windows_ambiguous" || return 1

	if command_output=$(
		"${HYPRPILOT}" session start --match-title "${title}" 2>&1
	); then
		fail "session start windows_ambiguous observe=succes (${command_output}); attendu=exit non nul"
		return 1
	fi
	if [[ ${command_output} != *$'\n'* ||
		${command_output%%$'\n'*} != *"multiple windows match"* ]]; then
		fail "session start windows_ambiguous message observe=${command_output}; attendu=message humain puis JSON"
		return 1
	fi
	last_line=${command_output##*$'\n'}
	if ! jq -e --argjson expected "${addresses_json}" \
		'type == "array" and length == 2 and
		 ([.[].address] | sort) == ($expected | sort)' \
		<<<"${last_line}" >/dev/null; then
		fail "session start windows_ambiguous candidats observe=${last_line}; attendu=${addresses_json}"
		return 1
	fi

	if ! windows_json=$("${HYPRPILOT}" windows); then
		fail "windows windows_ambiguous observe=echec; attendu=tableau JSON"
		return 1
	fi
	if ! jq -e --arg title "${title}" --argjson expected "${addresses_json}" \
		'type == "array" and
		 ([.[] | select(.title == $title) | .address] | sort) == ($expected | sort) and
		 all(.[]; .tracked == false and .active == false)' \
		<<<"${windows_json}" >/dev/null; then
		fail "windows windows_ambiguous observe=${windows_json}; attendu=deux fenêtres non suivies"
		return 1
	fi
	read_active_address active_address "windows windows_ambiguous" || return 1
	if ! jq -e --arg active "${active_address}" \
		'all(.[]; .focused == (.address == $active))' \
		<<<"${windows_json}" >/dev/null; then
		fail "windows windows_ambiguous focused observe=${windows_json}; attendu=coherent avec ${active_address:-aucun}"
		return 1
	fi
	if [[ -e ${session_file} ]]; then
		fail "windows_ambiguous session observe=presente (${session_file}); attendu=aucun état créé"
		return 1
	fi
	assert_output_absent "windows_ambiguous" || return 1

	kill "${zenity_one_pid}" "${zenity_two_pid}" 2>/dev/null || true
	wait "${zenity_one_pid}" 2>/dev/null || true
	wait "${zenity_two_pid}" 2>/dev/null || true
	zenity_one_pid=""
	zenity_two_pid=""
)

scenario_target_lifecycle() (
	local cleanup_failed=0
	local scenario_tmp="" session_file="" cleanup_output="" command_output="" shot_output=""
	local a_pid="" b_pid="" a_address="" b_address="" addresses_json=""
	local a_title="hyprpilot-e2e-target-a-$$"
	local b_title="hyprpilot-e2e-target-b-$$"
	local a_x a_y a_width a_height a_workspace a_floating a_monitor
	local b_x b_y b_width b_height b_workspace b_floating b_monitor
	local state_x state_y state_width state_height state_workspace state_floating state_monitor

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_target_lifecycle() {
		local scenario_status=$?
		local pid
		trap - EXIT INT TERM

		if [[ -n ${session_file} && -e ${session_file} ]]; then
			if ! cleanup_output=$("${HYPRPILOT}" teardown 2>&1); then
				fail "nettoyage target_lifecycle: teardown observe=echec (${cleanup_output}); attendu=succes"
				cleanup_failed=1
			fi
		fi
		for pid in "${a_pid}" "${b_pid}"; do
			if [[ -n ${pid} ]] && kill -0 "${pid}" 2>/dev/null; then
				kill "${pid}" 2>/dev/null || cleanup_failed=1
				wait "${pid}" 2>/dev/null || true
			fi
		done
		if ! assert_output_absent "nettoyage target_lifecycle"; then
			cleanup_failed=1
		fi
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-target-lifecycle.* ]]; then
				fail "nettoyage target_lifecycle: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_target_lifecycle EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	if [[ -z ${XDG_RUNTIME_DIR:-} ]]; then
		fail "target_lifecycle: XDG_RUNTIME_DIR observe=vide; attendu=repertoire runtime"
		return 1
	fi
	session_file=${XDG_RUNTIME_DIR}/hyprpilot/session.json
	if [[ -e ${session_file} ]]; then
		fail "precondition target_lifecycle: session observe=presente; attendu=absente"
		return 1
	fi
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-target-lifecycle.XXXXXX"); then
		fail "target_lifecycle: repertoire temporaire observe=creation impossible; attendu=mktemp reussi"
		return 1
	fi

	zenity --entry --title="${a_title}" >/dev/null 2>&1 &
	a_pid=$!
	wait_client_addresses_by_title addresses_json "${a_title}" 1 \
		"settle spawn A target_lifecycle" || return 1
	a_address=$(jq -er '.[0]' <<<"${addresses_json}") || return 1
	read_stable_client_state "${a_address}" a_x a_y a_width a_height a_workspace \
		a_floating a_monitor "origine A target_lifecycle" || return 1
	if [[ ${a_floating} != true ]]; then
		if ! command_output=$(
			hyprctl dispatch togglefloating "address:${a_address}" 2>&1
		) || [[ ${command_output} != ok ]]; then
			fail "setup A target_lifecycle: togglefloating observe=${command_output}; attendu=ok"
			return 1
		fi
		read_stable_client_state "${a_address}" a_x a_y a_width a_height a_workspace \
			a_floating a_monitor "origine flottante A target_lifecycle" || return 1
	fi

	if ! command_output=$(
		"${HYPRPILOT}" session start --match-title "${a_title}" 2>&1
	); then
		fail "session start target_lifecycle observe=echec (${command_output}); attendu=succes"
		return 1
	fi

	zenity --entry --title="${b_title}" >/dev/null 2>&1 &
	b_pid=$!
	wait_client_addresses_by_title addresses_json "${b_title}" 1 \
		"settle spawn B target_lifecycle" || return 1
	b_address=$(jq -er '.[0]' <<<"${addresses_json}") || return 1
	read_stable_client_state "${b_address}" b_x b_y b_width b_height b_workspace \
		b_floating b_monitor "origine B target_lifecycle" || return 1
	if [[ ${b_floating} != true ]]; then
		if ! command_output=$(
			hyprctl dispatch togglefloating "address:${b_address}" 2>&1
		) || [[ ${command_output} != ok ]]; then
			fail "setup B target_lifecycle: togglefloating observe=${command_output}; attendu=ok"
			return 1
		fi
		read_stable_client_state "${b_address}" b_x b_y b_width b_height b_workspace \
			b_floating b_monitor "origine flottante B target_lifecycle" || return 1
	fi

	if ! command_output=$(
		"${HYPRPILOT}" target --untracked --match-title "${b_title}" --wait 5s 2>&1
	); then
		fail "target B target_lifecycle observe=echec (${command_output}); attendu=adoption"
		return 1
	fi
	read_client_state "${b_address}" state_x state_y state_width state_height state_workspace \
		state_floating state_monitor "target B actif target_lifecycle" || return 1
	if [[ ${state_workspace} != hyprpilot ]]; then
		fail "target B target_lifecycle: workspace B observe=${state_workspace}; attendu=hyprpilot"
		return 1
	fi
	read_client_state "${a_address}" state_x state_y state_width state_height state_workspace \
		state_floating state_monitor "target A parque target_lifecycle" || return 1
	if [[ ${state_workspace} != special:hyprpilot-parked ]]; then
		fail "target B target_lifecycle: workspace A observe=${state_workspace}; attendu=special:hyprpilot-parked"
		return 1
	fi
	assert_parking_not_toggled "target B target_lifecycle" || return 1
	if ! shot_output=$(
		"${HYPRPILOT}" shot target-b --out "${scenario_tmp}" 2>&1
	) || [[ ! -s ${scenario_tmp}/target-b.png ]]; then
		fail "shot B target_lifecycle observe=${shot_output}; attendu=PNG immédiat non vide"
		return 1
	fi

	if ! command_output=$(
		"${HYPRPILOT}" target --match-title "${a_title}" 2>&1
	); then
		fail "target A target_lifecycle observe=echec (${command_output}); attendu=switch"
		return 1
	fi
	read_client_state "${a_address}" state_x state_y state_width state_height state_workspace \
		state_floating state_monitor "target A actif target_lifecycle" || return 1
	if [[ ${state_workspace} != hyprpilot ]]; then
		fail "target A target_lifecycle: workspace A observe=${state_workspace}; attendu=hyprpilot"
		return 1
	fi
	read_client_state "${b_address}" state_x state_y state_width state_height state_workspace \
		state_floating state_monitor "target B parque target_lifecycle" || return 1
	if [[ ${state_workspace} != special:hyprpilot-parked ]]; then
		fail "target A target_lifecycle: workspace B observe=${state_workspace}; attendu=special:hyprpilot-parked"
		return 1
	fi
	assert_parking_not_toggled "target A target_lifecycle" || return 1
	if ! shot_output=$(
		"${HYPRPILOT}" shot target-a --out "${scenario_tmp}" 2>&1
	) || [[ ! -s ${scenario_tmp}/target-a.png ]]; then
		fail "shot A target_lifecycle observe=${shot_output}; attendu=PNG immédiat non vide"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" teardown 2>&1); then
		fail "teardown target_lifecycle observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	wait_client_state_equals "${a_address}" "${a_x}" "${a_y}" "${a_width}" "${a_height}" \
		"${a_workspace}" "${a_floating}" "${a_monitor}" "restore A target_lifecycle" || return 1
	wait_client_state_equals "${b_address}" "${b_x}" "${b_y}" "${b_width}" "${b_height}" \
		"${b_workspace}" "${b_floating}" "${b_monitor}" "restore B target_lifecycle" || return 1
	assert_output_absent "teardown target_lifecycle" || return 1

	kill "${a_pid}" "${b_pid}" 2>/dev/null || true
	wait "${a_pid}" 2>/dev/null || true
	wait "${b_pid}" 2>/dev/null || true
	a_pid=""
	b_pid=""
)

scenario_target_close() (
	local cleanup_failed=0
	local session_file="" cleanup_output="" command_output=""
	local a_pid="" b_pid="" a_address="" b_address="" addresses_json=""
	local a_title="hyprpilot-e2e-target-close-a-$$"
	local b_title="hyprpilot-e2e-target-close-b-$$"
	local a_x a_y a_width a_height a_workspace a_floating a_monitor
	local b_x b_y b_width b_height b_workspace b_floating b_monitor
	local attempt process_gone=0

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_target_close() {
		local scenario_status=$?
		local pid
		trap - EXIT INT TERM

		if [[ -n ${session_file} && -e ${session_file} ]]; then
			if ! cleanup_output=$("${HYPRPILOT}" teardown 2>&1); then
				fail "nettoyage target_close: teardown observe=echec (${cleanup_output}); attendu=succes"
				cleanup_failed=1
			fi
		fi
		for pid in "${a_pid}" "${b_pid}"; do
			if [[ -n ${pid} ]] && kill -0 "${pid}" 2>/dev/null; then
				kill "${pid}" 2>/dev/null || cleanup_failed=1
				wait "${pid}" 2>/dev/null || true
			fi
		done
		if ! assert_output_absent "nettoyage target_close"; then
			cleanup_failed=1
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_target_close EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	if [[ -z ${XDG_RUNTIME_DIR:-} ]]; then
		fail "target_close: XDG_RUNTIME_DIR observe=vide; attendu=repertoire runtime"
		return 1
	fi
	session_file=${XDG_RUNTIME_DIR}/hyprpilot/session.json
	if [[ -e ${session_file} ]]; then
		fail "precondition target_close: session observe=presente; attendu=absente"
		return 1
	fi

	zenity --entry --title="${a_title}" >/dev/null 2>&1 &
	a_pid=$!
	wait_client_addresses_by_title addresses_json "${a_title}" 1 \
		"settle spawn A target_close" || return 1
	a_address=$(jq -er '.[0]' <<<"${addresses_json}") || return 1
	read_stable_client_state "${a_address}" a_x a_y a_width a_height a_workspace \
		a_floating a_monitor "origine A target_close" || return 1
	if [[ ${a_floating} != true ]]; then
		if ! command_output=$(
			hyprctl dispatch togglefloating "address:${a_address}" 2>&1
		) || [[ ${command_output} != ok ]]; then
			fail "setup A target_close: togglefloating observe=${command_output}; attendu=ok"
			return 1
		fi
		read_stable_client_state "${a_address}" a_x a_y a_width a_height a_workspace \
			a_floating a_monitor "origine flottante A target_close" || return 1
	fi
	if ! command_output=$(
		"${HYPRPILOT}" session start --match-title "${a_title}" 2>&1
	); then
		fail "session start target_close observe=echec (${command_output}); attendu=succes"
		return 1
	fi

	zenity --entry --title="${b_title}" >/dev/null 2>&1 &
	b_pid=$!
	wait_client_addresses_by_title addresses_json "${b_title}" 1 \
		"settle spawn B target_close" || return 1
	b_address=$(jq -er '.[0]' <<<"${addresses_json}") || return 1
	read_stable_client_state "${b_address}" b_x b_y b_width b_height b_workspace \
		b_floating b_monitor "origine B target_close" || return 1
	if ! command_output=$(
		"${HYPRPILOT}" target --untracked --match-title "${b_title}" \
			--on-teardown close 2>&1
	); then
		fail "target B target_close observe=echec (${command_output}); attendu=adoption close"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" teardown 2>&1); then
		fail "teardown target_close observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	wait_client_gone "${b_address}" "teardown target_close fenêtre B" || return 1
	for ((attempt = 0; attempt < 30; attempt++)); do
		if ! kill -0 "${b_pid}" 2>/dev/null; then
			process_gone=1
			break
		fi
		sleep 0.1
	done
	if ((process_gone == 0)); then
		fail "teardown target_close process B observe=${b_pid} vivant; attendu=disparu"
		return 1
	fi
	wait "${b_pid}" 2>/dev/null || true
	b_pid=""
	wait_client_state_equals "${a_address}" "${a_x}" "${a_y}" "${a_width}" "${a_height}" \
		"${a_workspace}" "${a_floating}" "${a_monitor}" "restore A target_close" || return 1
	assert_output_absent "teardown target_close" || return 1

	kill "${a_pid}" 2>/dev/null || true
	wait "${a_pid}" 2>/dev/null || true
	a_pid=""
)

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

scenario_focus_type() (
	local scenario_tmp="" zenity_pid="" window_address=""
	local cleanup_failed=0
	local title="hyprpilot-e2e-focus-type-$$"
	local typed_text="focus-type-$$"
	local stdout_file="" expected_output="" actual_output=""
	local before_focus before_x before_y
	local user_x user_y user_width user_height user_workspace user_floating user_monitor
	local center_x center_y addresses_json="" command_output="" cleanup_output=""

	assert_focus_type_user_state() {
		local label=$1
		local observed_focus observed_x observed_y delta_x delta_y attempt stable_reads=0

		for ((attempt = 0; attempt < 50; attempt++)); do
			read_active_address observed_focus "${label}" || return 1
			read_cursor observed_x observed_y "${label}" || return 1
			delta_x=$((observed_x - before_x))
			delta_y=$((observed_y - before_y))
			((delta_x < 0)) && delta_x=$((-delta_x))
			((delta_y < 0)) && delta_y=$((-delta_y))
			if [[ ${observed_focus} == "${before_focus}" ]] &&
				((delta_x <= 1 && delta_y <= 1)); then
				((stable_reads += 1))
				if ((stable_reads == 5)); then
					return 0
				fi
			else
				stable_reads=0
			fi
			sleep 0.1
		done
		fail "${label}: observe=focus ${observed_focus:-<aucun>}, curseur (${observed_x}, ${observed_y}); attendu=focus ${before_focus}, curseur (${before_x}, ${before_y}) +/-1 sur 5 lectures consecutives"
		return 1
	}

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_focus_type() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! cleanup_output=$("${HYPRPILOT}" teardown 2>&1); then
			if [[ ${cleanup_output} != *"no active session"* ]]; then
				fail "nettoyage focus_type: teardown observe=echec (${cleanup_output}); attendu=succes ou session deja demontee"
				cleanup_failed=1
			fi
		fi
		if [[ -n ${zenity_pid} ]] && kill -0 "${zenity_pid}" 2>/dev/null; then
			kill "${zenity_pid}" 2>/dev/null || cleanup_failed=1
			wait "${zenity_pid}" 2>/dev/null || true
		fi
		if ! assert_output_absent "nettoyage focus_type"; then
			cleanup_failed=1
		fi
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-focus-type.* ]]; then
				fail "nettoyage focus_type: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage focus_type: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_focus_type EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	if [[ -z ${XDG_RUNTIME_DIR:-} ]]; then
		fail "focus_type: XDG_RUNTIME_DIR observe=vide; attendu=repertoire runtime"
		return 1
	fi
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-focus-type.XXXXXX"); then
		fail "focus_type: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi
	stdout_file=${scenario_tmp}/zenity.stdout

	read_active_address before_focus "precondition focus_type" || return 1
	if [[ -z ${before_focus} ]]; then
		skip "aucune fenêtre active pour établir un état restaurable"
	fi
	read_client_state "${before_focus}" user_x user_y user_width user_height user_workspace \
		user_floating user_monitor "precondition focus_type" || return 1
	center_x=$((user_x + user_width / 2))
	center_y=$((user_y + user_height / 2))
	if ! command_output=$(hyprctl dispatch movecursor "${center_x}" "${center_y}" 2>&1) ||
		[[ ${command_output} != ok ]]; then
		fail "precondition focus_type: movecursor observe=${command_output}; attendu=ok vers (${center_x}, ${center_y})"
		return 1
	fi
	read_cursor before_x before_y "precondition focus_type" || return 1
	assert_focus_type_user_state "settle precondition focus_type" || return 1

	zenity --entry --title="${title}" >"${stdout_file}" 2>/dev/null &
	zenity_pid=$!
	wait_client_addresses_by_title addresses_json "${title}" 1 \
		"settle spawn focus_type" || return 1
	window_address=$(jq -er '.[0]' <<<"${addresses_json}") || return 1

	if ! command_output=$(
		hyprctl dispatch focuswindow "address:${before_focus}" 2>&1
	) || [[ ${command_output} != ok ]]; then
		fail "precondition focus_type: focuswindow observe=${command_output}; attendu=ok vers ${before_focus}"
		return 1
	fi
	if ! command_output=$(hyprctl dispatch movecursor "${before_x}" "${before_y}" 2>&1) ||
		[[ ${command_output} != ok ]]; then
		fail "precondition focus_type apres spawn: movecursor observe=${command_output}; attendu=ok vers (${before_x}, ${before_y})"
		return 1
	fi
	assert_focus_type_user_state "settle spawn focus_type" || return 1

	if ! command_output=$(
		"${HYPRPILOT}" session start --match-title "${title}" 2>&1
	); then
		fail "session start focus_type observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	assert_focus_type_user_state "settle session start focus_type" || return 1

	if ! command_output=$("${HYPRPILOT}" click --focus 20 20 2>&1); then
		fail "click --focus focus_type observe=echec (${command_output}); attendu=champ zenity clique"
		return 1
	fi
	assert_focus_type_user_state "apres click --focus focus_type" || return 1

	if ! command_output=$("${HYPRPILOT}" type --focus "${typed_text}" 2>&1); then
		fail "type --focus focus_type observe=echec (${command_output}); attendu=${typed_text}"
		return 1
	fi
	assert_focus_type_user_state "apres type --focus focus_type" || return 1

	if ! command_output=$("${HYPRPILOT}" key --focus Return 2>&1); then
		fail "key --focus focus_type observe=echec (${command_output}); attendu=Return accepte"
		return 1
	fi
	assert_focus_type_user_state "apres key --focus focus_type" || return 1

	if ! wait "${zenity_pid}"; then
		fail "zenity focus_type observe=sortie non nulle; attendu=validation par Return"
		return 1
	fi
	zenity_pid=""
	wait_client_gone "${window_address}" "apres Return focus_type" || return 1

	expected_output="${typed_text}"$'\n'
	IFS= read -r -d '' actual_output <"${stdout_file}" || true
	if [[ ${actual_output} != "${expected_output}" ]]; then
		fail "stdout zenity focus_type observe=${actual_output@Q}; attendu=${expected_output@Q}"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" teardown 2>&1); then
		fail "teardown focus_type observe=echec (${command_output}); attendu=succes avec fenetre deja disparue"
		return 1
	fi
	assert_output_absent "teardown focus_type"
)

scenario_resize() (
	local cleanup_failed=0
	local scenario_tmp="" zenity_pid="" window_address=""
	local title="hyprpilot-e2e-resize-$$"
	local command_output cleanup_output shot_output status_json addresses_json
	local origin_x origin_y origin_width origin_height origin_workspace
	local origin_floating origin_monitor
	local window_x window_y window_width window_height window_workspace
	local window_floating window_monitor
	local output_x output_y output_width output_height
	local png_width png_height attempt stable_reads=0

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_resize() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! cleanup_output=$("${HYPRPILOT}" teardown 2>&1); then
			if [[ ${cleanup_output} != *"no active session"* ]]; then
				fail "nettoyage resize: teardown observe=echec (${cleanup_output}); attendu=succes ou session deja demontee"
				cleanup_failed=1
			fi
		fi
		if [[ -n ${zenity_pid} ]] && kill -0 "${zenity_pid}" 2>/dev/null; then
			kill "${zenity_pid}" 2>/dev/null || cleanup_failed=1
			wait "${zenity_pid}" 2>/dev/null || true
		fi
		if ! assert_output_absent "nettoyage resize"; then
			cleanup_failed=1
		fi
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-resize.* ]]; then
				fail "nettoyage resize: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage resize: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_resize EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	if [[ -z ${XDG_RUNTIME_DIR:-} ]]; then
		fail "resize: XDG_RUNTIME_DIR observe=vide; attendu=repertoire runtime"
		return 1
	fi
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-resize.XXXXXX"); then
		fail "resize: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi

	zenity --entry --title="${title}" >/dev/null 2>&1 &
	zenity_pid=$!
	wait_client_addresses_by_title addresses_json "${title}" 1 "settle spawn resize" || return 1
	window_address=$(jq -er '.[0]' <<<"${addresses_json}") || return 1

	read_stable_client_state "${window_address}" origin_x origin_y origin_width origin_height \
		origin_workspace origin_floating origin_monitor "settle origine resize" || return 1
	if [[ ${origin_floating} != true ]]; then
		if ! command_output=$(hyprctl dispatch togglefloating "address:${window_address}" 2>&1) ||
			[[ ${command_output} != ok ]]; then
			fail "setup resize: togglefloating observe=${command_output}; attendu=ok"
			return 1
		fi
		read_stable_client_state "${window_address}" origin_x origin_y origin_width origin_height \
			origin_workspace origin_floating origin_monitor "settle floating resize" || return 1
	fi
	if [[ ${origin_floating} != true ]]; then
		fail "setup resize: floating observe=${origin_floating}; attendu=true"
		return 1
	fi

	if ! command_output=$(
		"${HYPRPILOT}" session start --match-title "${title}" --size 300x200 2>&1
	); then
		fail "session start resize observe=echec (${command_output}); attendu=succes oversized"
		return 1
	fi
	if [[ ${command_output} != *"larger than output"* ]]; then
		fail "session start resize avertissement observe=${command_output}; attendu=warning oversized"
		return 1
	fi
	read_output_geometry output_x output_y output_width output_height \
		"apres session start resize" || return 1
	if ((output_width != 300 || output_height != 200)); then
		fail "session start resize output observe=${output_width}x${output_height}; attendu=300x200"
		return 1
	fi
	if ! shot_output=$(
		"${HYPRPILOT}" shot resize-clamped --out "${scenario_tmp}" 2>&1
	); then
		fail "shot oversized resize observe=echec (${shot_output}); attendu=capture clamp reussie"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" session resize 1200x800 2>&1); then
		fail "session resize observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	for ((attempt = 0; attempt < 30; attempt++)); do
		read_output_geometry output_x output_y output_width output_height \
			"settle output resize" || return 1
		if ((output_width == 1200 && output_height == 800)); then
			((stable_reads += 1))
			if ((stable_reads == 2)); then
				break
			fi
		else
			stable_reads=0
		fi
		sleep 0.1
	done
	if ((stable_reads != 2)); then
		fail "settle output resize observe=${output_width}x${output_height}; attendu=1200x800 stable"
		return 1
	fi

	if ! status_json=$("${HYPRPILOT}" status 2>&1); then
		fail "status resize observe=echec (${status_json}); attendu=JSON sans mismatch"
		return 1
	fi
	if ! jq -e '
		.configured_size == [1200, 800]
		and .effective_size == [1200, 800]
		and .size_mismatch == false
	' <<<"${status_json}" >/dev/null; then
		fail "status resize observe=${status_json}; attendu=configured/effective 1200x800 sans mismatch"
		return 1
	fi

	read_stable_client_state "${window_address}" window_x window_y window_width window_height \
		window_workspace window_floating window_monitor "settle fenetre resize" || return 1
	if [[ ${window_workspace} != hyprpilot ]]; then
		fail "placement resize workspace observe=${window_workspace}; attendu=hyprpilot"
		return 1
	fi
	if ((window_x < output_x || window_y < output_y ||
		window_x + window_width > output_x + output_width ||
		window_y + window_height > output_y + output_height)); then
		fail "placement resize observe=fenetre (${window_x}, ${window_y}) ${window_width}x${window_height}, output (${output_x}, ${output_y}) ${output_width}x${output_height}; attendu=fenetre entierement contenue"
		return 1
	fi

	if ! shot_output=$(
		"${HYPRPILOT}" shot resize-full-window --out "${scenario_tmp}" 2>&1
	); then
		fail "shot apres resize observe=echec (${shot_output}); attendu=capture complete"
		return 1
	fi
	read_png_size "${scenario_tmp}/resize-full-window.png" png_width png_height \
		"shot apres resize" || return 1
	if ((png_width != window_width || png_height != window_height)); then
		fail "shot apres resize taille observe=${png_width}x${png_height}; attendu=fenetre ${window_width}x${window_height}"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" teardown 2>&1); then
		fail "teardown resize observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	wait_client_state_equals "${window_address}" "${origin_x}" "${origin_y}" "${origin_width}" \
		"${origin_height}" "${origin_workspace}" "${origin_floating}" "${origin_monitor}" \
		"restauration resize" || return 1
	assert_output_absent "teardown resize" || return 1

	kill "${zenity_pid}" 2>/dev/null || true
	wait "${zenity_pid}" 2>/dev/null || true
	zenity_pid=""
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
	for binary in hyprctl grim jq zenity; do
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
