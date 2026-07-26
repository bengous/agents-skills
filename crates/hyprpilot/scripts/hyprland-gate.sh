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

note() {
	printf 'NOTE: %s\n' "$*"
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
	local observed_width observed_height
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
	observed_width=$(((bytes[0] << 24) | (bytes[1] << 16) | (bytes[2] << 8) | bytes[3]))
	observed_height=$(((bytes[4] << 24) | (bytes[5] << 16) | (bytes[6] << 8) | bytes[7]))
	if ((observed_width <= 0 || observed_height <= 0)); then
		fail "${label}: taille observe=${observed_width}x${observed_height}; attendu=dimensions PNG positives"
		return 1
	fi
	printf -v "${width_destination}" '%s' "${observed_width}"
	printf -v "${height_destination}" '%s' "${observed_height}"
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
		remaining=${remaining#*\"x\":"${x}",}
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

# --- Helpers du mode isolé (bureaux agents) --------------------------------

session_dir_path() {
	printf '%s/hyprpilot/sessions/%s' "${XDG_RUNTIME_DIR}" "$1"
}

session_file_path() {
	printf '%s/session.json' "$(session_dir_path "$1")"
}

# Lecture silencieuse d'un champ d'etat : les traps s'en servent sans polluer
# la sortie quand la session a deja disparu.
state_field() {
	local session=$1
	local field=$2
	local file value

	file=$(session_file_path "${session}")
	[[ -f ${file} ]] || return 1
	value=$(jq -r "${field}" <"${file}" 2>/dev/null) || return 1
	[[ ${value} != null ]] || return 1
	printf '%s' "${value}"
}

read_state_field() {
	local session=$1
	local field=$2
	local destination=$3
	local label=$4
	local file value

	file=$(session_file_path "${session}")
	if [[ ! -f ${file} ]]; then
		fail "${label}: etat observe=absent (${file}); attendu=session.json de ${session}"
		return 1
	fi
	if ! value=$(jq -r "${field}" <"${file}" 2>/dev/null) || [[ ${value} == null ]]; then
		fail "${label}: champ observe=absent ou nul (${field}); attendu=valeur dans ${file}"
		return 1
	fi
	printf -v "${destination}" '%s' "${value}"
}

assert_state_field() {
	local session=$1
	local field=$2
	local expected=$3
	local label=$4
	local observed

	read_state_field "${session}" "${field}" observed "${label}" || return 1
	if [[ ${observed} != "${expected}" ]]; then
		fail "${label}: ${field} observe=${observed}; attendu=${expected}"
		return 1
	fi
}

require_isolated_support() {
	[[ -n ${XDG_RUNTIME_DIR:-} ]] ||
		skip "XDG_RUNTIME_DIR est vide: aucun bureau agent possible"
	command -v Hyprland >/dev/null 2>&1 ||
		skip "binaire Hyprland absent du PATH: aucun bureau agent possible"
}

snapshot_hypr_signatures() {
	local destination=$1
	local entry joined=""

	if [[ -d ${XDG_RUNTIME_DIR}/hypr ]]; then
		for entry in "${XDG_RUNTIME_DIR}"/hypr/*; do
			[[ -d ${entry} ]] || continue
			joined+=${entry##*/}$'\n'
		done
	fi
	printf -v "${destination}" '%s' "${joined}"
}

# Signatures apparues depuis le snapshot, hote exclu : la seule identification
# d'instance nested qu'un trap peut utiliser sans lire l'etat de l'outil.
new_hypr_signatures() {
	local destination=$1
	local before=$2
	local current="" signature joined=""

	snapshot_hypr_signatures current
	while IFS= read -r signature; do
		[[ -n ${signature} ]] || continue
		[[ ${signature} != "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]] || continue
		[[ $'\n'${before} != *$'\n'"${signature}"$'\n'* ]] || continue
		joined+=${signature}$'\n'
	done <<<"${current}"
	printf -v "${destination}" '%s' "${joined}"
}

nested_process_is_hyprland() {
	local pid=$1
	local cmdline

	[[ ${pid} =~ ^[0-9]+$ ]] || return 1
	[[ -r /proc/${pid}/cmdline ]] || return 1
	cmdline=$(tr '\0' ' ' </proc/"${pid}"/cmdline) || return 1
	[[ ${cmdline} == *Hyprland* ]]
}

# Un compositeur imbrique apparait comme client class aquamarine de l'hote
# (fait §2.5) ; l'hote n'est jamais son propre client. Garde des chemins qui
# signalent un PID : un etat errone ne peut pas viser la session utilisateur.
nested_pid_is_console() {
	local pid=$1
	local raw

	[[ ${pid} =~ ^[0-9]+$ ]] || return 1
	raw=$(hyprctl clients -j 2>/dev/null) || return 1
	jq -e --argjson pid "${pid}" \
		'any(.[]; .pid == $pid and .class == "aquamarine")' <<<"${raw}" >/dev/null
}

nested_instance_alive() {
	hyprctl -i "$1" version >/dev/null 2>&1
}

wait_nested_instance_gone() {
	local signature=$1
	local label=$2
	local attempt

	for ((attempt = 0; attempt < 50; attempt++)); do
		nested_instance_alive "${signature}" || return 0
		sleep 0.1
	done
	fail "${label}: instance observe=vivante (${signature}); attendu=terminee en 5s"
	return 1
}

# Termine une instance nested et efface son socket sans passer par hyprpilot :
# utilisable dans un trap apres n'importe quel echec. Le PID n'est signale
# qu'apres confirmation par /proc, jamais cherche par nom de binaire.
kill_nested_signature() {
	local signature=$1
	local pid=$2
	local label=$3
	local socket_dir attempt status=0

	if [[ -z ${signature} || ${signature} == "${HYPRLAND_INSTANCE_SIGNATURE:-}" ||
		! ${signature} =~ ^[A-Za-z0-9_.-]+$ ]]; then
		fail "${label}: signature observe=${signature:-<vide>}; attendu=signature nested distincte de l'hote"
		return 1
	fi
	socket_dir=${XDG_RUNTIME_DIR}/hypr/${signature}
	if nested_instance_alive "${signature}"; then
		hyprctl -i "${signature}" dispatch exit >/dev/null 2>&1 || true
		for ((attempt = 0; attempt < 50; attempt++)); do
			nested_instance_alive "${signature}" || break
			sleep 0.1
		done
	fi
	if nested_instance_alive "${signature}" && [[ -n ${pid} ]] &&
		nested_process_is_hyprland "${pid}" && nested_pid_is_console "${pid}"; then
		kill -TERM "${pid}" 2>/dev/null || true
		for ((attempt = 0; attempt < 30; attempt++)); do
			kill -0 "${pid}" 2>/dev/null || break
			sleep 0.1
		done
		if kill -0 "${pid}" 2>/dev/null; then
			kill -KILL "${pid}" 2>/dev/null || true
		fi
	fi
	if nested_instance_alive "${signature}"; then
		fail "${label}: instance observe=vivante (${signature}); attendu=terminee"
		status=1
	fi
	if [[ -e ${socket_dir} ]] && ! rm -rf -- "${socket_dir}"; then
		fail "${label}: socket observe=present (${socket_dir}); attendu=supprime"
		status=1
	fi
	return "${status}"
}

named_output_present() {
	local output=$1
	local monitors

	monitors=$(hyprctl monitors -j 2>/dev/null) || return 2
	jq -e --arg name "${output}" 'any(.[]; .name == $name)' <<<"${monitors}" >/dev/null
}

assert_named_output_absent() {
	local output=$1
	local label=$2
	local monitors

	if ! monitors=$(hyprctl monitors -j 2>&1); then
		fail "${label}: monitors observe=erreur hyprctl (${monitors}); attendu=liste sans ${output}"
		return 1
	fi
	if jq -e --arg name "${output}" 'any(.[]; .name == $name)' <<<"${monitors}" >/dev/null; then
		fail "${label}: output observe=${output} present; attendu=absent"
		return 1
	fi
}

wait_named_output_absent() {
	local output=$1
	local label=$2
	local attempt

	for ((attempt = 0; attempt < 30; attempt++)); do
		named_output_present "${output}" || break
		sleep 0.1
	done
	assert_named_output_absent "${output}" "${label}"
}

remove_named_output() {
	local output=$1
	local label=$2
	local command_output

	if [[ ${output} != hyprpilot-e2e-* ]]; then
		fail "${label}: output observe=${output}; attendu=output e2e cree par le scenario"
		return 1
	fi
	named_output_present "${output}" || return 0
	if ! command_output=$(hyprctl output remove "${output}" 2>&1) ||
		[[ ${command_output} != ok ]]; then
		fail "${label}: output remove observe=${command_output}; attendu=ok pour ${output}"
		return 1
	fi
}

remove_session_dir() {
	local session=$1
	local label=$2
	local directory

	directory=$(session_dir_path "${session}")
	if [[ ${directory} != "${XDG_RUNTIME_DIR}"/hyprpilot/sessions/e2e-* ]]; then
		fail "${label}: dossier observe=${directory}; attendu=session e2e sous ${XDG_RUNTIME_DIR}"
		return 1
	fi
	[[ -e ${directory} ]] || return 0
	if ! rm -rf -- "${directory}"; then
		fail "${label}: dossier observe=present (${directory}); attendu=supprime"
		return 1
	fi
}

# Nettoyage brut d'un bureau agent : instances apparues pendant le scenario,
# output headless, dossier de session. Ne depend d'aucune commande de
# hyprpilot et reste correct si le scenario a echoue avant de tout creer.
# signatures_ready=0 => le snapshot n'a pas ete pris, aucune instance touchee.
isolated_raw_cleanup() {
	local session=$1
	local signatures_ready=$2
	local signatures_before=$3
	local label=$4
	local status=0 signatures="" signature pid=""
	local -a fresh=()

	if ((signatures_ready)); then
		new_hypr_signatures signatures "${signatures_before}"
		while IFS= read -r signature; do
			[[ -n ${signature} ]] || continue
			fresh+=("${signature}")
		done <<<"${signatures}"
		if ((${#fresh[@]} == 1)); then
			pid=$(state_field "${session}" '.instance.pid') || pid=""
		fi
		for signature in "${fresh[@]}"; do
			kill_nested_signature "${signature}" "${pid}" "${label}" || status=1
		done
	fi
	remove_named_output "hyprpilot-${session}" "${label}" || status=1
	remove_session_dir "${session}" "${label}" || status=1
	return "${status}"
}

restore_cursor_best_effort() {
	local x=$1
	local y=$2

	[[ ${x} =~ ^-?[0-9]+$ && ${y} =~ ^-?[0-9]+$ ]] || return 0
	hyprctl dispatch movecursor "${x}" "${y}" >/dev/null 2>&1 || true
}

read_named_output() {
	local output=$1
	local width_destination=$2
	local height_destination=$3
	local scale_destination=$4
	local workspace_destination=$5
	local label=$6
	local raw values width height scale workspace extra

	if ! raw=$(hyprctl monitors -j 2>&1); then
		fail "${label}: monitors observe=erreur hyprctl (${raw}); attendu=output ${output}"
		return 1
	fi
	if ! values=$(
		jq -er --arg name "${output}" '
			[.[] | select(.name == $name)]
			| select(length == 1)
			| .[0]
			| [.width, .height, .scale, (.activeWorkspace.name // "")]
			| @tsv
		' <<<"${raw}"
	); then
		fail "${label}: output observe=absent ou duplique (${output}); attendu=un seul moniteur ${output}"
		return 1
	fi
	IFS=$'\t' read -r width height scale workspace extra <<<"${values}"
	if [[ -n ${extra:-} || ! ${width} =~ ^[1-9][0-9]*$ || ! ${height} =~ ^[1-9][0-9]*$ ]]; then
		fail "${label}: geometrie observe=${values}; attendu=taille positive pour ${output}"
		return 1
	fi
	printf -v "${width_destination}" '%s' "${width}"
	printf -v "${height_destination}" '%s' "${height}"
	printf -v "${scale_destination}" '%s' "${scale}"
	printf -v "${workspace_destination}" '%s' "${workspace}"
}

# Console d'une instance nested cote hote : workspace attendu + class
# aquamarine (fait §2.5), jamais le titre. Silencieux: appele en boucle.
find_console_window() {
	local workspace=$1
	local address_destination=$2
	local pid_destination=$3
	local raw values address pid

	raw=$(hyprctl clients -j 2>/dev/null) || return 1
	values=$(
		jq -er --arg workspace "${workspace}" '
			[.[] | select(.workspace.name == $workspace and .class == "aquamarine")]
			| select(length == 1)
			| .[0]
			| [.address, .pid]
			| @tsv
		' <<<"${raw}"
	) || return 1
	IFS=$'\t' read -r address pid <<<"${values}"
	printf -v "${address_destination}" '%s' "${address}"
	printf -v "${pid_destination}" '%s' "${pid}"
}

wait_console_workspace() {
	local session_workspace=$1
	local console_address=$2
	local label=$3
	local workspace attempt stable_reads=0
	# shellcheck disable=SC2034 # Sorties obligatoires de read_client_state (printf -v), seul workspace est relu.
	local x y width height floating monitor

	for ((attempt = 0; attempt < 50; attempt++)); do
		read_client_state "${console_address}" x y width height workspace floating monitor \
			"${label}" || return 1
		if [[ ${workspace} == "${session_workspace}" ]]; then
			((stable_reads += 1))
			if ((stable_reads == 2)); then
				return 0
			fi
		else
			stable_reads=0
		fi
		sleep 0.1
	done
	fail "${label}: workspace de la console observe=${workspace}; attendu=${session_workspace} stable"
	return 1
}

wait_nested_addresses_by_title() {
	local signature=$1
	local destination=$2
	local wanted_title=$3
	local expected_count=$4
	local label=$5
	local raw addresses count previous="" attempt stable_reads=0

	for ((attempt = 0; attempt < 50; attempt++)); do
		if ! raw=$(hyprctl -i "${signature}" clients -j 2>&1); then
			fail "${label}: clients de l'instance observe=erreur hyprctl (${raw}); attendu=${expected_count} fenêtre(s)"
			return 1
		fi
		if ! addresses=$(
			jq -c --arg title "${wanted_title}" \
				'[.[] | select(.title == $title) | .address] | sort' <<<"${raw}"
		); then
			fail "${label}: clients de l'instance observe=JSON invalide; attendu=tableau filtrable"
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
	fail "${label}: adresses de l'instance observe=${addresses}; attendu=${expected_count} fenêtre(s) stables en 5s"
	return 1
}

read_nested_active_address() {
	local signature=$1
	local destination=$2
	local label=$3
	local raw address

	if ! raw=$(hyprctl -i "${signature}" activewindow -j 2>&1); then
		fail "${label}: fenetre active de l'instance observe=erreur hyprctl (${raw}); attendu=JSON"
		return 1
	fi
	if ! address=$(jq -er '.address' <<<"${raw}" 2>/dev/null); then
		fail "${label}: fenetre active de l'instance observe=${raw}; attendu=une adresse"
		return 1
	fi
	printf -v "${destination}" '%s' "${address}"
}

read_nested_client_size() {
	local signature=$1
	local address=$2
	local width_destination=$3
	local height_destination=$4
	local label=$5
	local raw values width height extra

	if ! raw=$(hyprctl -i "${signature}" clients -j 2>&1); then
		fail "${label}: clients de l'instance observe=erreur hyprctl (${raw}); attendu=fenetre ${address}"
		return 1
	fi
	if ! values=$(
		jq -er --arg address "${address}" '
			[.[] | select(.address == $address)]
			| select(length == 1)
			| .[0].size
			| @tsv
		' <<<"${raw}"
	); then
		fail "${label}: taille observe=absente pour ${address}; attendu=size de la fenetre dans l'instance"
		return 1
	fi
	IFS=$'\t' read -r width height extra <<<"${values}"
	if [[ -n ${extra:-} || ! ${width} =~ ^[1-9][0-9]*$ || ! ${height} =~ ^[1-9][0-9]*$ ]]; then
		fail "${label}: taille observe=${values}; attendu=deux entiers positifs"
		return 1
	fi
	printf -v "${width_destination}" '%s' "${width}"
	printf -v "${height_destination}" '%s' "${height}"
}

read_nested_output_size() {
	local signature=$1
	local width_destination=$2
	local height_destination=$3
	local label=$4
	local raw values width height extra

	if ! raw=$(hyprctl -i "${signature}" monitors -j 2>&1); then
		fail "${label}: monitors de l'instance observe=erreur hyprctl (${raw}); attendu=un output"
		return 1
	fi
	if ! values=$(
		jq -er 'select(length == 1) | .[0] | [.width, .height] | @tsv' <<<"${raw}"
	); then
		fail "${label}: monitors de l'instance observe=${raw}; attendu=exactement un output"
		return 1
	fi
	IFS=$'\t' read -r width height extra <<<"${values}"
	if [[ -n ${extra:-} || ! ${width} =~ ^[1-9][0-9]*$ || ! ${height} =~ ^[1-9][0-9]*$ ]]; then
		fail "${label}: taille de l'output nested observe=${values}; attendu=deux entiers positifs"
		return 1
	fi
	printf -v "${width_destination}" '%s' "${width}"
	printf -v "${height_destination}" '%s' "${height}"
}

# §5 : en isolé, `target` ne parque rien. Aucun client de l'instance ne doit
# se retrouver sur un workspace special.
assert_nested_no_parking() {
	local signature=$1
	local label=$2
	local raw

	if ! raw=$(hyprctl -i "${signature}" clients -j 2>&1); then
		fail "${label}: clients de l'instance observe=erreur hyprctl (${raw}); attendu=liste sans workspace special"
		return 1
	fi
	if ! jq -e 'all(.[]; (.workspace.name // "") | startswith("special:") | not)' \
		<<<"${raw}" >/dev/null; then
		fail "${label}: workspaces de l'instance observe=$(jq -c '[.[].workspace.name]' <<<"${raw}"); attendu=aucun workspace special"
		return 1
	fi
}

read_active_workspace() {
	local destination=$1
	local label=$2
	local raw name

	if ! raw=$(hyprctl activeworkspace -j 2>&1); then
		fail "${label}: workspace actif observe=erreur hyprctl (${raw}); attendu=JSON"
		return 1
	fi
	if ! name=$(jq -er '.name' <<<"${raw}" 2>/dev/null); then
		fail "${label}: workspace actif observe=${raw}; attendu=champ name"
		return 1
	fi
	printf -v "${destination}" '%s' "${name}"
}

# Les variables internes des lecteurs sont prefixees observed_ : une destination
# qui porterait le meme nom qu'un local serait masquee par lui, et `printf -v`
# ecrirait dans le local au lieu de la variable de l'appelant.
read_host_addresses() {
	local destination=$1
	local label=$2
	local raw observed_addresses

	if ! raw=$(hyprctl clients -j 2>&1); then
		fail "${label}: clients observe=erreur hyprctl (${raw}); attendu=liste des fenêtres hôte"
		return 1
	fi
	if ! observed_addresses=$(jq -c '[.[].address] | sort' <<<"${raw}"); then
		fail "${label}: clients observe=JSON invalide; attendu=tableau d'adresses"
		return 1
	fi
	printf -v "${destination}" '%s' "${observed_addresses}"
}

# Projection stable des monitors : les champs volatils (focus, dpms, format)
# changeraient sans qu'un output ait bouge. Le workspace actif PAR moniteur en
# fait partie (§10 G2.2) : un renommage ou une bascule sur un moniteur non
# focalise est invisible pour `activeworkspace`, qui ne parle que du focus.
read_host_monitors_shape() {
	local destination=$1
	local label=$2
	local raw shape

	if ! raw=$(hyprctl monitors -j 2>&1); then
		fail "${label}: monitors observe=erreur hyprctl (${raw}); attendu=liste des outputs"
		return 1
	fi
	if ! shape=$(
		jq -Sc '
			[.[] | {name, x, y, width, height, scale, transform,
				activeWorkspace: (.activeWorkspace.name // "")}]
			| sort_by(.name)
		' <<<"${raw}"
	); then
		fail "${label}: monitors observe=JSON invalide; attendu=liste projetable"
		return 1
	fi
	printf -v "${destination}" '%s' "${shape}"
}

# Liste des workspaces exigee par §10 G2.2 : un workspace renomme (le start
# renomme le workspace actif de son headless) ou detruit se voit ici, y compris
# quand il appartenait a un moniteur non focalise.
read_host_workspaces() {
	local destination=$1
	local label=$2
	local raw names

	if ! raw=$(hyprctl workspaces -j 2>&1); then
		fail "${label}: workspaces observe=erreur hyprctl (${raw}); attendu=liste des workspaces"
		return 1
	fi
	if ! names=$(jq -c '[.[].name] | sort' <<<"${raw}"); then
		fail "${label}: workspaces observe=JSON invalide; attendu=tableau de noms"
		return 1
	fi
	printf -v "${destination}" '%s' "${names}"
}

read_focused_monitor() {
	local x_destination=$1
	local y_destination=$2
	local height_destination=$3
	local label=$4
	local raw values x y height extra

	if ! raw=$(hyprctl monitors -j 2>&1); then
		fail "${label}: monitors observe=erreur hyprctl (${raw}); attendu=un moniteur focalise"
		return 1
	fi
	if ! values=$(
		jq -er '
			[.[] | select(.focused == true)]
			| select(length == 1)
			| .[0]
			| [.x, .y, .height]
			| @tsv
		' <<<"${raw}"
	); then
		fail "${label}: moniteur focalise observe=absent ou multiple; attendu=exactement un"
		return 1
	fi
	IFS=$'\t' read -r x y height extra <<<"${values}"
	if [[ -n ${extra:-} || ! ${x} =~ ^-?[0-9]+$ || ! ${y} =~ ^-?[0-9]+$ ||
		! ${height} =~ ^[1-9][0-9]*$ ]]; then
		fail "${label}: moniteur focalise observe=${values}; attendu=x, y et hauteur entiers"
		return 1
	fi
	printf -v "${x_destination}" '%s' "${x}"
	printf -v "${y_destination}" '%s' "${y}"
	printf -v "${height_destination}" '%s' "${height}"
}

# Point du bureau utilisateur volontairement excentre : `output remove`
# recentre le curseur sur le moniteur restant, un point central rendrait
# l'assertion de restauration vacante.
move_cursor_offcenter() {
	local x_destination=$1
	local y_destination=$2
	local label=$3
	local monitor_x monitor_y monitor_height target_x target_y command_output
	local observed_x observed_y delta_x delta_y attempt stable_reads=0

	read_focused_monitor monitor_x monitor_y monitor_height "${label}" || return 1
	target_x=$((monitor_x + 13))
	target_y=$((monitor_y + monitor_height / 2 + 7))
	if ! command_output=$(hyprctl dispatch movecursor "${target_x}" "${target_y}" 2>&1) ||
		[[ ${command_output} != ok ]]; then
		fail "${label}: movecursor observe=${command_output}; attendu=ok vers (${target_x}, ${target_y})"
		return 1
	fi
	for ((attempt = 0; attempt < 30; attempt++)); do
		read_cursor observed_x observed_y "${label}" || return 1
		delta_x=$((observed_x - target_x))
		delta_y=$((observed_y - target_y))
		((delta_x < 0)) && delta_x=$((-delta_x))
		((delta_y < 0)) && delta_y=$((-delta_y))
		if ((delta_x <= 1 && delta_y <= 1)); then
			((stable_reads += 1))
			if ((stable_reads == 2)); then
				printf -v "${x_destination}" '%s' "${observed_x}"
				printf -v "${y_destination}" '%s' "${observed_y}"
				return 0
			fi
		else
			stable_reads=0
		fi
		sleep 0.1
	done
	fail "${label}: curseur observe=(${observed_x}, ${observed_y}); attendu=(${target_x}, ${target_y}) +/-1 sur 2 lectures"
	return 1
}

read_stable_cursor() {
	local x_destination=$1
	local y_destination=$2
	local label=$3
	local observed_x observed_y previous_x="" previous_y="" attempt

	for ((attempt = 0; attempt < 30; attempt++)); do
		read_cursor observed_x observed_y "${label}" || return 1
		if [[ ${observed_x} == "${previous_x}" && ${observed_y} == "${previous_y}" ]]; then
			printf -v "${x_destination}" '%s' "${observed_x}"
			printf -v "${y_destination}" '%s' "${observed_y}"
			return 0
		fi
		previous_x=${observed_x}
		previous_y=${observed_y}
		sleep 0.1
	done
	fail "${label}: curseur observe=(${observed_x}, ${observed_y}); attendu=deux lectures identiques en 3s"
	return 1
}

assert_cursor_restored() {
	local expected_x=$1
	local expected_y=$2
	local label=$3
	local observed_x observed_y delta_x delta_y attempt

	for ((attempt = 0; attempt < 10; attempt++)); do
		read_cursor observed_x observed_y "${label}" || return 1
		delta_x=$((observed_x - expected_x))
		delta_y=$((observed_y - expected_y))
		((delta_x < 0)) && delta_x=$((-delta_x))
		((delta_y < 0)) && delta_y=$((-delta_y))
		if ((delta_x <= 1 && delta_y <= 1)); then
			return 0
		fi
		sleep 0.1
	done
	fail "${label}: curseur observe=(${observed_x}, ${observed_y}); attendu=(${expected_x}, ${expected_y}) +/-1 px par axe"
	return 1
}

read_host_snapshot() {
	local workspace_destination=$1
	local focus_destination=$2
	local cursor_x_destination=$3
	local cursor_y_destination=$4
	local addresses_destination=$5
	local monitors_destination=$6
	local workspaces_destination=$7
	local label=$8
	local workspace focus cursor_x cursor_y addresses monitors workspaces

	read_active_workspace workspace "${label}" || return 1
	read_active_address focus "${label}" || return 1
	read_cursor cursor_x cursor_y "${label}" || return 1
	read_host_addresses addresses "${label}" || return 1
	read_host_monitors_shape monitors "${label}" || return 1
	read_host_workspaces workspaces "${label}" || return 1
	printf -v "${workspace_destination}" '%s' "${workspace}"
	printf -v "${focus_destination}" '%s' "${focus}"
	printf -v "${cursor_x_destination}" '%s' "${cursor_x}"
	printf -v "${cursor_y_destination}" '%s' "${cursor_y}"
	printf -v "${addresses_destination}" '%s' "${addresses}"
	printf -v "${monitors_destination}" '%s' "${monitors}"
	printf -v "${workspaces_destination}" '%s' "${workspaces}"
}

# Compare le bureau utilisateur a un snapshot. Les fenêtres se comparent par
# ADRESSE, jamais par titre (un titre change tout seul). expected_monitors="-"
# (et de meme expected_workspaces) saute la comparaison des outputs et des
# workspaces, pour un instant ou la session tient encore son headless et son
# workspace agent. expected_gained = adresses hôte que le scenario autorise en
# plus (la console du nested, sinon []). Toutes les deviations sont rapportees.
assert_host_snapshot_equals() {
	local expected_workspace=$1
	local expected_focus=$2
	local expected_cursor_x=$3
	local expected_cursor_y=$4
	local expected_addresses=$5
	local expected_monitors=$6
	local expected_workspaces=$7
	local expected_gained=$8
	local label=$9
	local workspace focus addresses monitors workspaces lost gained status=0

	read_active_workspace workspace "${label}" || return 1
	if [[ ${workspace} != "${expected_workspace}" ]]; then
		fail "${label}: workspace actif observe=${workspace}; attendu=${expected_workspace}"
		status=1
	fi
	read_active_address focus "${label}" || return 1
	if [[ ${focus} != "${expected_focus}" ]]; then
		fail "${label}: fenetre active observe=${focus:-<aucune>}; attendu=${expected_focus:-<aucune>}"
		status=1
	fi
	assert_cursor_restored "${expected_cursor_x}" "${expected_cursor_y}" "${label}" || status=1
	read_host_addresses addresses "${label}" || return 1
	if ! lost=$(jq -c --argjson after "${addresses}" '. - $after' <<<"${expected_addresses}"); then
		fail "${label}: adresses observe=comparaison impossible; attendu=deux tableaux JSON"
		return 1
	fi
	if ! gained=$(jq -c --argjson before "${expected_addresses}" '. - $before' <<<"${addresses}"); then
		fail "${label}: adresses observe=comparaison impossible; attendu=deux tableaux JSON"
		return 1
	fi
	if [[ ${lost} != "[]" ]]; then
		fail "${label}: fenêtres hôte disparues=${lost}; attendu=aucune"
		status=1
	fi
	if ! jq -e --argjson expected "${expected_gained}" \
		'sort == ($expected | sort)' <<<"${gained}" >/dev/null; then
		fail "${label}: fenêtres hôte apparues=${gained}; attendu=${expected_gained}"
		status=1
	fi
	if [[ ${expected_monitors} != "-" ]]; then
		read_host_monitors_shape monitors "${label}" || return 1
		if [[ ${monitors} != "${expected_monitors}" ]]; then
			fail "${label}: monitors observe=${monitors}; attendu=${expected_monitors}"
			status=1
		fi
	fi
	if [[ ${expected_workspaces} != "-" ]]; then
		read_host_workspaces workspaces "${label}" || return 1
		if [[ ${workspaces} != "${expected_workspaces}" ]]; then
			fail "${label}: workspaces observe=${workspaces}; attendu=${expected_workspaces}"
			status=1
		fi
	fi
	return "${status}"
}

workspace_present() {
	local workspace=$1
	local raw

	raw=$(hyprctl workspaces -j 2>/dev/null) || return 2
	jq -e --arg name "${workspace}" 'any(.[]; .name == $name)' <<<"${raw}" >/dev/null
}

assert_workspace_absent() {
	local workspace=$1
	local label=$2
	local raw

	if ! raw=$(hyprctl workspaces -j 2>&1); then
		fail "${label}: workspaces observe=erreur hyprctl (${raw}); attendu=liste sans ${workspace}"
		return 1
	fi
	if jq -e --arg name "${workspace}" 'any(.[]; .name == $name)' <<<"${raw}" >/dev/null; then
		fail "${label}: workspace observe=${workspace} present; attendu=absent"
		return 1
	fi
}

# Hyprland peut mettre un instant a detruire un workspace nomme devenu vide
# apres `output remove` : l'attente est bornee, l'echec reste une fuite.
wait_workspace_absent() {
	local workspace=$1
	local label=$2
	local attempt

	for ((attempt = 0; attempt < 30; attempt++)); do
		workspace_present "${workspace}" || break
		sleep 0.1
	done
	assert_workspace_absent "${workspace}" "${label}"
}

# Selecteur accepte par le dispatcher workspace : un workspace numerique passe
# tel quel, un workspace nomme exige `name:` (meme regle que l'outil).
workspace_selector() {
	local workspace=$1

	if [[ ${workspace} =~ ^-?[0-9]+$ || ${workspace} == special:* ]]; then
		printf '%s' "${workspace}"
	else
		printf 'name:%s' "${workspace}"
	fi
}

# Simule l'utilisateur qui change de workspace. Le scenario ne bascule que vers
# un workspace qu'il a nomme lui-meme, jamais vers un workspace de
# l'utilisateur : le retour se fait par le nom lu dans le snapshot.
switch_host_workspace() {
	local workspace=$1
	local label=$2
	local command_output observed attempt stable_reads=0

	if ! command_output=$(
		hyprctl dispatch workspace "$(workspace_selector "${workspace}")" 2>&1
	) || [[ ${command_output} != ok ]]; then
		fail "${label}: workspace observe=${command_output}; attendu=ok vers ${workspace}"
		return 1
	fi
	for ((attempt = 0; attempt < 30; attempt++)); do
		read_active_workspace observed "${label}" || return 1
		if [[ ${observed} == "${workspace}" ]]; then
			((stable_reads += 1))
			if ((stable_reads == 2)); then
				return 0
			fi
		else
			stable_reads=0
		fi
		sleep 0.1
	done
	fail "${label}: workspace actif observe=${observed}; attendu=${workspace} stable en 3s"
	return 1
}

restore_workspace_best_effort() {
	local workspace=$1

	[[ -n ${workspace} ]] || return 0
	hyprctl dispatch workspace "$(workspace_selector "${workspace}")" >/dev/null 2>&1 || true
}

# §10 : `agent-<session>` doit rester le workspace ACTIF de son headless, meme
# console partie chez l'utilisateur. Sinon un `hide` recree le workspace sur le
# moniteur courant de la console, donc chez l'utilisateur, et le nested cesse de
# recevoir des frames (fait §2.3).
assert_agent_workspace_active() {
	local session=$1
	local label=$2
	local status=0 active_workspace
	# shellcheck disable=SC2034 # Sorties obligatoires de read_named_output (printf -v), seul le workspace est relu.
	local output_width output_height output_scale

	workspace_present "agent-${session}" || status=$?
	if ((status == 2)); then
		fail "${label}: workspaces observe=erreur hyprctl; attendu=liste contenant agent-${session}"
		return 1
	fi
	if ((status != 0)); then
		fail "${label}: workspace observe=agent-${session} absent; attendu=present"
		return 1
	fi
	read_named_output "hyprpilot-${session}" output_width output_height output_scale \
		active_workspace "${label}" || return 1
	if [[ ${active_workspace} != "agent-${session}" ]]; then
		fail "${label}: workspace actif de hyprpilot-${session} observe=${active_workspace}; attendu=agent-${session}"
		return 1
	fi
}

# Sockets Wayland du runtime : `wayland-<n>` et son verrou `wayland-<n>.lock`.
# Un nested tue brutalement ne les efface pas lui-meme, un start echoue non plus.
snapshot_wayland_sockets() {
	local destination=$1
	local entry joined=""

	for entry in "${XDG_RUNTIME_DIR}"/wayland-*; do
		[[ -e ${entry} ]] || continue
		joined+=${entry##*/}$'\n'
	done
	printf -v "${destination}" '%s' "${joined}"
}

new_wayland_sockets() {
	local destination=$1
	local before=$2
	local current="" socket joined=""

	snapshot_wayland_sockets current
	while IFS= read -r socket; do
		[[ -n ${socket} ]] || continue
		[[ $'\n'${before} != *$'\n'"${socket}"$'\n'* ]] || continue
		joined+=${socket}$'\n'
	done <<<"${current}"
	printf -v "${destination}" '%s' "${joined}"
}

assert_no_new_wayland_sockets() {
	local before=$1
	local label=$2
	local leftover="" attempt

	# libwayland efface le socket a la sortie du compositeur ; l'attente bornee
	# ne couvre que ce delai, une survie durable est la fuite recherchee.
	for ((attempt = 0; attempt < 30; attempt++)); do
		new_wayland_sockets leftover "${before}"
		[[ -n ${leftover} ]] || return 0
		sleep 0.1
	done
	fail "${label}: sockets Wayland residuels=${leftover//$'\n'/ }; attendu=aucun sous ${XDG_RUNTIME_DIR}"
	return 1
}

# `hyprctl instances -j` est la seule source qui lie une signature au socket
# Wayland qu'elle sert et a son PID.
read_instance_socket() {
	local signature=$1
	local socket_destination=$2
	local pid_destination=$3
	local label=$4
	local raw values observed_socket observed_pid extra

	if ! raw=$(hyprctl instances -j 2>&1); then
		fail "${label}: instances observe=erreur hyprctl (${raw}); attendu=liste des instances"
		return 1
	fi
	if ! values=$(
		jq -er --arg signature "${signature}" '
			[.[] | select(.instance == $signature)]
			| select(length == 1)
			| .[0]
			| [.wl_socket, .pid]
			| @tsv
		' <<<"${raw}"
	); then
		fail "${label}: instance observe=absente ou dupliquee (${signature}); attendu=une entree dans hyprctl instances"
		return 1
	fi
	IFS=$'\t' read -r observed_socket observed_pid extra <<<"${values}"
	if [[ -n ${extra:-} || ! ${observed_socket} =~ ^wayland-[0-9]+$ ||
		! ${observed_pid} =~ ^[0-9]+$ ]]; then
		fail "${label}: instance observe=${values}; attendu=wl_socket wayland-<n> et PID entier"
		return 1
	fi
	printf -v "${socket_destination}" '%s' "${observed_socket}"
	printf -v "${pid_destination}" '%s' "${observed_pid}"
}

# Le wayland_display et le PID enregistres doivent etre ceux de la signature
# enregistree par la MEME session : une decouverte par diff de repertoire peut,
# sous deux starts concurrents, enregistrer le socket ou le PID du voisin.
assert_state_instance_binding() {
	local session=$1
	local label=$2
	local signature="" display="" pid="" socket="" instance_pid=""

	read_state_field "${session}" '.instance.signature' signature "${label}" || return 1
	read_state_field "${session}" '.instance.wayland_display' display "${label}" || return 1
	read_state_field "${session}" '.instance.pid' pid "${label}" || return 1
	read_instance_socket "${signature}" socket instance_pid "${label}" || return 1
	if [[ ${socket} != "${display}" ]]; then
		fail "${label}: wayland_display enregistre=${display}; attendu=${socket}, celui de l'instance ${signature}"
		return 1
	fi
	if [[ ${instance_pid} != "${pid}" ]]; then
		fail "${label}: pid enregistre=${pid}; attendu=${instance_pid}, celui de l'instance ${signature}"
		return 1
	fi
}

# Tout process d'un bureau agent herite du marqueur pose par le spawn, y compris
# les descendants de l'app : la seule identification qui ne passe par aucun nom
# de binaire. Les environs illisibles (autre utilisateur, process disparu en
# cours de lecture) sont ignores.
scan_marked_pids() {
	local destination=$1
	local session=$2
	local entry pid environ joined=""

	for entry in /proc/[0-9]*/environ; do
		[[ -r ${entry} ]] || continue
		# `2>` avant `<` : sur un process non dumpable, access(2) autorise mais
		# l'ouverture echoue, et le message doit etre deja detourne.
		environ=$(tr '\0' '\n' 2>/dev/null <"${entry}") || continue
		if [[ $'\n'${environ}$'\n' == *$'\n'"HYPRPILOT_AGENT_SESSION=${session}"$'\n'* ]]; then
			pid=${entry#/proc/}
			joined+=${pid%/environ}$'\n'
		fi
	done
	printf -v "${destination}" '%s' "${joined}"
}

assert_no_marked_processes() {
	local session=$1
	local label=$2
	local pids="" attempt

	for ((attempt = 0; attempt < 10; attempt++)); do
		scan_marked_pids pids "${session}"
		[[ -n ${pids} ]] || return 0
		sleep 0.1
	done
	fail "${label}: process observe=${pids//$'\n'/ } portent encore HYPRPILOT_AGENT_SESSION=${session}; attendu=aucun"
	return 1
}

# Residus d'un cycle isole, quel que soit le chemin emprunte (start echoue,
# teardown, mort brutale) : output nomme, repertoire d'instance, socket Wayland,
# process marque. Toutes les deviations sont rapportees.
assert_isolated_no_residue() {
	local session=$1
	local signatures_before=$2
	local sockets_before=$3
	local label=$4
	local status=0 leftover=""

	wait_named_output_absent "hyprpilot-${session}" "${label}" || status=1
	new_hypr_signatures leftover "${signatures_before}"
	if [[ -n ${leftover} ]]; then
		fail "${label}: signatures residuelles=${leftover//$'\n'/ }; attendu=aucune"
		status=1
	fi
	assert_no_new_wayland_sockets "${sockets_before}" "${label}" || status=1
	assert_no_marked_processes "${session}" "${label}" || status=1
	return "${status}"
}

read_aquamarine_addresses() {
	local destination=$1
	local label=$2
	local raw addresses

	if ! raw=$(hyprctl clients -j 2>&1); then
		fail "${label}: clients observe=erreur hyprctl (${raw}); attendu=liste des fenêtres hôte"
		return 1
	fi
	if ! addresses=$(
		jq -c '[.[] | select(.class == "aquamarine") | .address] | sort' <<<"${raw}"
	); then
		fail "${label}: clients observe=JSON invalide; attendu=tableau d'adresses"
		return 1
	fi
	printf -v "${destination}" '%s' "${addresses}"
}

# Une fenêtre class aquamarine (fait §2.5) apparue depuis `before` et posee
# ailleurs que sur l'output headless de la session : la console de l'agent est
# chez l'utilisateur. Assertion de PRESENCE seulement, aucune action ne vise
# cette fenêtre. Silencieux : appele en boucle serree pendant qu'un start tourne.
# 0 = violation (decrite dans destination), 1 = propre, 2 = lecture impossible.
console_on_user_monitor() {
	local output=$1
	local before=$2
	local destination=$3
	local monitors clients found

	monitors=$(hyprctl monitors -j 2>/dev/null) || return 2
	clients=$(hyprctl clients -j 2>/dev/null) || return 2
	found=$(
		jq -cn --argjson monitors "${monitors}" --argjson clients "${clients}" \
			--argjson before "${before}" --arg output "${output}" '
			[$monitors[] | select(.name == $output) | .id] as $headless
			| [$clients[]
				| select(.class == "aquamarine")
				| select((.address | IN($before[])) | not)
				| select((.monitor | IN($headless[])) | not)
				| {address, monitor, workspace: (.workspace.name // "")}]
		' 2>/dev/null
	) || return 2
	[[ ${found} != "[]" ]] || return 1
	printf -v "${destination}" '%s' "${found}"
	return 0
}

# Trace ordonnee des evenements de l'hote (socket2, lecture seule, comme waybar).
# C'est la seule preuve DETERMINISTE de l'ordre entre le retrait de l'output
# headless et la disparition de la fenêtre-console: un client mort est reape en
# une iteration de la boucle du compositeur, trop vite pour un echantillonnage.
# Facultatif: sans socat le scenario retombe sur l'echantillonnage et le dit.
start_host_event_tap() {
	local pid_destination=$1
	local path=$2
	local socket=${XDG_RUNTIME_DIR}/hypr/${HYPRLAND_INSTANCE_SIGNATURE:-}/.socket2.sock

	command -v socat >/dev/null 2>&1 || return 1
	[[ -S ${socket} ]] || return 1
	: >"${path}" || return 1
	socat -u "UNIX-CONNECT:${socket}" - >"${path}" 2>/dev/null &
	printf -v "${pid_destination}" '%s' "$!"
}

stop_host_event_tap() {
	local pid=$1

	[[ -n ${pid} ]] || return 0
	kill "${pid}" 2>/dev/null || true
	wait "${pid}" 2>/dev/null || true
}

# Ordre impose par §6 et par le rollback du start: la fenêtre-console disparait
# AVANT le retrait de l'output qui la porte, sinon `output remove` la donne a un
# moniteur de l'utilisateur. Les adresses du socket2 n'ont pas le prefixe 0x,
# d'ou la comparaison par suffixe.
assert_console_reaped_before_output() {
	local path=$1
	local output=$2
	local console=$3
	local label=$4
	local suffix=${console#0x} removal="" close=""

	if [[ ! -f ${path} ]]; then
		fail "${label}: trace d'evenements observe=absente (${path}); attendu=journal du socket2"
		return 1
	fi
	removal=$(grep -n -m1 -e "^monitorremoved.*${output}" -- "${path}") || removal=""
	if [[ -z ${removal} ]]; then
		note "${label}: aucun evenement monitorremoved pour ${output}, preuve d'ordre indisponible"
		return 0
	fi
	close=$(grep -n -m1 -e "^closewindow>>.*${suffix}" -- "${path}") || close=""
	if [[ -z ${close} ]]; then
		fail "${label}: evenements observe=monitorremoved de ${output} sans closewindow de la console ${console}; attendu=la console reapee avant le retrait de son output"
		return 1
	fi
	if ((${close%%:*} > ${removal%%:*})); then
		fail "${label}: evenements observe=closewindow ligne ${close%%:*} apres monitorremoved ligne ${removal%%:*}; attendu=la console ${console} reapee AVANT le retrait de ${output}"
		return 1
	fi
}

# Horloge monotone suffisante pour distinguer un echec immediat d'un timeout de
# capture. EPOCHREALTIME = <secondes><radix><microsecondes> et le caractere de
# radix suit la locale : seuls les chiffres sont fiables, les 13 premiers font
# les millisecondes.
now_ms() {
	local destination=$1
	local label=$2
	local digits=${EPOCHREALTIME//[^0-9]/}

	if ((${#digits} < 13)); then
		fail "${label}: EPOCHREALTIME observe=${EPOCHREALTIME:-<vide>}; attendu=horloge bash 5"
		return 1
	fi
	printf -v "${destination}" '%s' "${digits:0:13}"
}

assert_png_dimensions() {
	local path=$1
	local expected_width=$2
	local expected_height=$3
	local label=$4
	local width height

	read_png_size "${path}" width height "${label}" || return 1
	if ((width != expected_width || height != expected_height)); then
		fail "${label}: PNG observe=${width}x${height} (${path}); attendu=${expected_width}x${expected_height}"
		return 1
	fi
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
	session_file=$(session_file_path default)
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
	session_file=$(session_file_path default)
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
	session_file=$(session_file_path default)
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
	# shellcheck disable=SC2034 # Sorties obligatoires de read_client_state (printf -v), lues seulement pour les axes geometriques.
	local user_x user_y user_width user_height user_workspace user_floating user_monitor
	# shellcheck disable=SC2030 # Portee locale a ce scenario : la variable homonyme des autres scenarios est independante.
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
	# shellcheck disable=SC2034 # Sortie obligatoire de read_stable_client_state (printf -v), non relue ici.
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
	# shellcheck disable=SC2031 # addresses_json est locale a ce scenario, remplie juste au-dessus dans le meme sous-shell.
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
			if [[ ${session_file} != "$(session_file_path default)" ]]; then
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
	session_file=$(session_file_path default)
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

scenario_shared_teardown_cursor() (
	local cleanup_failed=0
	local session="e2e-cursor-$$"
	local title="hyprpilot-e2e-shared-cursor-$$"
	local zenity_pid="" command_output="" cleanup_output="" session_file=""
	local entry_x="" entry_y="" before_x before_y
	# shellcheck disable=SC2034 # Sortie obligatoire de wait_client_addresses_by_title (printf -v), non relue ici.
	local addresses_json=""

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_shared_teardown_cursor() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if [[ -n ${session_file} && -e ${session_file} ]]; then
			if ! cleanup_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
				fail "nettoyage shared_teardown_cursor: teardown observe=echec (${cleanup_output}); attendu=succes"
				cleanup_failed=1
			fi
		fi
		if [[ -n ${zenity_pid} ]] && kill -0 "${zenity_pid}" 2>/dev/null; then
			kill "${zenity_pid}" 2>/dev/null || cleanup_failed=1
			wait "${zenity_pid}" 2>/dev/null || true
		fi
		if ! assert_output_absent "nettoyage shared_teardown_cursor"; then
			cleanup_failed=1
		fi
		if ! remove_session_dir "${session}" "nettoyage shared_teardown_cursor"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_shared_teardown_cursor EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	if [[ -z ${XDG_RUNTIME_DIR:-} ]]; then
		fail "shared_teardown_cursor: XDG_RUNTIME_DIR observe=vide; attendu=repertoire runtime"
		return 1
	fi
	session_file=$(session_file_path "${session}")
	read_cursor entry_x entry_y "precondition shared_teardown_cursor" || return 1
	if [[ -e ${session_file} ]]; then
		fail "precondition shared_teardown_cursor: session observe=presente (${session_file}); attendu=absente"
		return 1
	fi
	assert_output_absent "precondition shared_teardown_cursor" || return 1

	zenity --entry --title="${title}" >/dev/null 2>&1 &
	zenity_pid=$!
	wait_client_addresses_by_title addresses_json "${title}" 1 \
		"settle spawn shared_teardown_cursor" || return 1
	move_cursor_offcenter before_x before_y "precondition curseur shared_teardown_cursor" ||
		return 1

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --match-title "${title}" 2>&1
	); then
		fail "session start shared_teardown_cursor observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	read_stable_cursor before_x before_y "avant teardown shared_teardown_cursor" || return 1

	if ! command_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
		fail "teardown shared_teardown_cursor observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	assert_cursor_restored "${before_x}" "${before_y}" \
		"curseur apres teardown shared_teardown_cursor" || return 1
	assert_output_absent "teardown shared_teardown_cursor" || return 1
	if [[ -e ${session_file} ]]; then
		fail "teardown shared_teardown_cursor: session observe=presente (${session_file}); attendu=supprimee"
		return 1
	fi

	kill "${zenity_pid}" 2>/dev/null || true
	wait "${zenity_pid}" 2>/dev/null || true
	zenity_pid=""
)

# Un mode-set impossible fait echouer le start APRES la creation de l'output
# headless. Le rollback doit retirer cet output: sinon le bureau utilisateur
# garde un moniteur fantome que plus aucune commande de l'outil ne connait. Le
# message d'erreur ne prouve rien ici, seul l'etat du compositeur compte.
scenario_isolated_start_bad_size() (
	local cleanup_failed=0
	local session="e2e-badsize-$$"
	local title="hyprpilot-e2e-iso-badsize-$$"
	local signatures_before="" signatures_ready=0 sockets_before=""
	local entry_x="" entry_y="" session_dir="" command_output=""
	local host_workspace host_focus host_cursor_x host_cursor_y host_addresses host_monitors
	local host_workspaces
	# shellcheck disable=SC2034 # Sorties obligatoires de move_cursor_offcenter (printf -v); la reference reste le snapshot.
	local offcenter_x offcenter_y

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_start_bad_size() {
		local scenario_status=$?
		trap - EXIT INT TERM

		# Le nettoyage ne peut rien deleguer a l'outil: par construction son
		# start vient d'echouer.
		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_start_bad_size"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_start_bad_size EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_start_bad_size" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	snapshot_wayland_sockets sockets_before
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_start_bad_size: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi
	assert_named_output_absent "hyprpilot-${session}" \
		"precondition isolated_start_bad_size" || return 1
	assert_no_marked_processes "${session}" "precondition isolated_start_bad_size" || return 1
	move_cursor_offcenter offcenter_x offcenter_y \
		"precondition curseur isolated_start_bad_size" || return 1
	read_host_snapshot host_workspace host_focus host_cursor_x host_cursor_y host_addresses \
		host_monitors host_workspaces "snapshot avant isolated_start_bad_size" || return 1

	# 99999x99999 passe le parsing de --size (seul 0 est refuse) et echoue au
	# mode-set de l'output, une fois celui-ci deja cree.
	if command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "zenity --entry --title=${title}" \
			--match-title "${title}" --size 99999x99999 2>&1
	); then
		fail "session start isolated_start_bad_size observe=succes (${command_output}); attendu=exit non nul"
		return 1
	fi
	note "isolated_start_bad_size: start refuse (${command_output})"

	# L'assertion qui distingue le defaut.
	wait_named_output_absent "hyprpilot-${session}" \
		"start rate isolated_start_bad_size" || return 1
	wait_workspace_absent "agent-${session}" "start rate isolated_start_bad_size" || return 1
	assert_isolated_no_residue "${session}" "${signatures_before}" "${sockets_before}" \
		"start rate isolated_start_bad_size" || return 1
	assert_host_snapshot_equals "${host_workspace}" "${host_focus}" "${host_cursor_x}" \
		"${host_cursor_y}" "${host_addresses}" "${host_monitors}" "${host_workspaces}" "[]" \
		"hote apres start rate isolated_start_bad_size" || return 1
)

# Un start qui ne trouve jamais sa fenêtre echoue apres avoir tout acquis: c'est
# le chemin de rollback le plus complet. Assertion centrale: pendant cet echec,
# aucune fenêtre class aquamarine ne doit apparaitre sur un moniteur de
# l'utilisateur. Retirer l'output headless avant que l'hote ait detruit la
# fenêtre-console y pose la console de l'agent (fait §2.5 pour la class,
# §2.8 pour le retrait).
scenario_isolated_start_match_failure() (
	local cleanup_failed=0
	local session="e2e-nomatch-$$"
	local title="hyprpilot-e2e-iso-nomatch-app-$$"
	local never_title="hyprpilot-e2e-iso-nomatch-absent-$$"
	local signatures_before="" signatures_ready=0 sockets_before=""
	local entry_x="" entry_y="" session_dir="" scenario_tmp="" start_log=""
	local command_output="" aquamarine_before="" offenders="" violation=""
	local start_pid="" start_status=0 probe_status=0 probes=0 probe_errors=0
	local tap_pid="" tap_log="" tap_ready=0 console_address=""
	local host_workspace host_focus host_cursor_x host_cursor_y host_addresses host_monitors
	local host_workspaces
	# shellcheck disable=SC2034 # Sorties obligatoires de move_cursor_offcenter (printf -v); la reference reste le snapshot.
	local offcenter_x offcenter_y

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_start_match_failure() {
		local scenario_status=$?
		trap - EXIT INT TERM

		stop_host_event_tap "${tap_pid}"
		# Le start peut etre encore en vol si le scenario a echoue pendant
		# l'observation: c'est un process que le scenario a lui-meme lance.
		if [[ -n ${start_pid} ]] && kill -0 "${start_pid}" 2>/dev/null; then
			kill "${start_pid}" 2>/dev/null || cleanup_failed=1
			wait "${start_pid}" 2>/dev/null || true
		fi
		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_start_match_failure"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-iso-nomatch.* ]]; then
				fail "nettoyage isolated_start_match_failure: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage isolated_start_match_failure: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_start_match_failure EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_start_match_failure" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	snapshot_wayland_sockets sockets_before
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_start_match_failure: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi
	assert_named_output_absent "hyprpilot-${session}" \
		"precondition isolated_start_match_failure" || return 1
	assert_no_marked_processes "${session}" \
		"precondition isolated_start_match_failure" || return 1
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-iso-nomatch.XXXXXX"); then
		fail "isolated_start_match_failure: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi
	start_log=${scenario_tmp}/start.log
	tap_log=${scenario_tmp}/events.log
	if start_host_event_tap tap_pid "${tap_log}"; then
		tap_ready=1
	else
		note "isolated_start_match_failure: socket2 non observable (socat absent?), preuve d'ordre remplacee par l'echantillonnage seul"
	fi
	move_cursor_offcenter offcenter_x offcenter_y \
		"precondition curseur isolated_start_match_failure" || return 1
	# Les fenêtres aquamarine deja presentes (un nested de l'utilisateur) ne
	# comptent pas: la violation se mesure sur les adresses apparues.
	read_aquamarine_addresses aquamarine_before \
		"precondition isolated_start_match_failure" || return 1
	read_host_snapshot host_workspace host_focus host_cursor_x host_cursor_y host_addresses \
		host_monitors host_workspaces "snapshot avant isolated_start_match_failure" || return 1

	# L'app existe et s'affiche, seul le titre attendu n'arrivera jamais: le
	# start va jusqu'a l'attente du match avant d'echouer.
	"${HYPRPILOT}" --session "${session}" session start --isolated \
		--app "zenity --entry --title=${title}" \
		--match-title "${never_title}" --size 1280x720 >"${start_log}" 2>&1 &
	start_pid=$!
	# La console relocalisee ne vit que le temps que l'hote reape sa fenêtre:
	# l'echantillonnage est serre et sans assertion de duree.
	while kill -0 "${start_pid}" 2>/dev/null; do
		((probes += 1))
		if console_on_user_monitor "hyprpilot-${session}" "${aquamarine_before}" offenders; then
			if [[ -z ${violation} ]]; then
				violation=${offenders}
			fi
		else
			probe_status=$?
			if ((probe_status == 2)); then
				((probe_errors += 1))
			fi
		fi
		# L'etat nomme la console des que le spawn l'a identifiee; il disparait
		# avec le rollback, donc il se lit pendant que le start tourne.
		if [[ -z ${console_address} ]]; then
			console_address=$(state_field "${session}" '.instance.console_address') ||
				console_address=""
		fi
		sleep 0.01
	done
	if wait "${start_pid}"; then
		start_status=0
	else
		start_status=1
	fi
	start_pid=""
	# Le retrait de l'output peut tomber apres la derniere lecture de la boucle.
	if console_on_user_monitor "hyprpilot-${session}" "${aquamarine_before}" offenders; then
		if [[ -z ${violation} ]]; then
			violation=${offenders}
		fi
	fi
	command_output=$(<"${start_log}")

	if ((start_status == 0)); then
		fail "session start isolated_start_match_failure observe=succes (${command_output}); attendu=exit non nul"
		return 1
	fi
	if ((probes < 5 || probe_errors == probes)); then
		fail "observation isolated_start_match_failure: echantillons observe=${probes} dont ${probe_errors} illisibles; attendu=une fenêtre d'observation lisible pendant le start"
		return 1
	fi
	# Assertion centrale: la console de l'agent n'a jamais ete chez l'utilisateur.
	if [[ -n ${violation} ]]; then
		fail "isolated_start_match_failure: fenêtre aquamarine observe=${violation} hors de hyprpilot-${session}; attendu=aucune sur un moniteur de l'utilisateur"
		return 1
	fi
	# Prouve que l'echec est bien l'attente du match, pas une panne en amont qui
	# rendrait tout le scenario vacant.
	if [[ ${command_output} != *"${never_title}"* ]]; then
		fail "session start isolated_start_match_failure message observe=${command_output}; attendu=le critere ${never_title} nomme"
		return 1
	fi

	wait_named_output_absent "hyprpilot-${session}" \
		"start rate isolated_start_match_failure" || return 1
	# La trace est complete des que l'output a disparu: preuve d'ordre, la ou
	# l'echantillonnage n'est qu'une chance de voir la fenêtre.
	stop_host_event_tap "${tap_pid}"
	tap_pid=""
	if ((tap_ready == 1)); then
		if [[ -z ${console_address} ]]; then
			note "isolated_start_match_failure: console jamais nommee dans l'etat, preuve d'ordre indisponible"
		else
			assert_console_reaped_before_output "${tap_log}" "hyprpilot-${session}" \
				"${console_address}" "rollback isolated_start_match_failure" || return 1
		fi
	fi
	wait_workspace_absent "agent-${session}" "start rate isolated_start_match_failure" || return 1
	assert_isolated_no_residue "${session}" "${signatures_before}" "${sockets_before}" \
		"start rate isolated_start_match_failure" || return 1
	if [[ -e ${session_dir} ]]; then
		fail "start rate isolated_start_match_failure: session observe=presente (${session_dir}); attendu=supprimee"
		return 1
	fi
	assert_host_snapshot_equals "${host_workspace}" "${host_focus}" "${host_cursor_x}" \
		"${host_cursor_y}" "${host_addresses}" "${host_monitors}" "${host_workspaces}" "[]" \
		"hote apres start rate isolated_start_match_failure" || return 1
)

scenario_isolated_output() (
	local cleanup_failed=0
	local session="e2e-out-$$"
	local title="hyprpilot-e2e-iso-out-$$"
	local signatures_before="" signatures_ready=0 leftover=""
	local entry_x="" entry_y="" session_dir=""
	local command_output="" start_failed=0
	local output_width output_height output_scale output_workspace

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_output() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_output"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_output EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_output" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_output: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi
	assert_named_output_absent "hyprpilot-${session}" "precondition isolated_output" || return 1

	# Tolerance assumee: tant que S4 a S6 ne sont pas livrees, le start cree
	# l'output puis echoue en nommant sa slice. Le scenario porte sur l'output
	# observe, pas sur le code de sortie du start.
	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "zenity --entry --title=${title}" \
			--match-title "${title}" --size 1280x720 2>&1
	); then
		start_failed=1
	fi
	if ((start_failed != 0)); then
		if ! named_output_present "hyprpilot-${session}"; then
			fail "session start isolated_output observe=echec (${command_output}); attendu=output hyprpilot-${session} cree"
			return 1
		fi
		note "isolated_output: start incomplet (${command_output}); output en place, assertions poursuivies"
	fi

	read_named_output "hyprpilot-${session}" output_width output_height output_scale \
		output_workspace "isolated_output" || return 1
	if ((output_width != 1280 || output_height != 720)); then
		fail "isolated_output: taille observe=${output_width}x${output_height}; attendu=1280x720"
		return 1
	fi
	if [[ ! ${output_scale} =~ ^1(\.0+)?$ ]]; then
		fail "isolated_output: scale observe=${output_scale}; attendu=1"
		return 1
	fi
	if [[ ${output_workspace} != "agent-${session}" ]]; then
		fail "isolated_output: workspace actif de hyprpilot-${session} observe=${output_workspace}; attendu=agent-${session}"
		return 1
	fi
	assert_state_field "${session}" '.mode' isolated "etat isolated_output" || return 1
	assert_state_field "${session}" '.output' "hyprpilot-${session}" "etat isolated_output" || return 1
	assert_state_field "${session}" '.workspace' "agent-${session}" "etat isolated_output" || return 1

	# Retrait par hyprctl brut: le scenario ne depend pas du teardown de l'outil.
	isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
		"retrait isolated_output" || return 1
	wait_named_output_absent "hyprpilot-${session}" "retrait isolated_output" || return 1
	wait_workspace_absent "agent-${session}" "retrait isolated_output" || return 1
	if [[ -e ${session_dir} ]]; then
		fail "retrait isolated_output: session observe=presente (${session_dir}); attendu=supprimee"
		return 1
	fi
	new_hypr_signatures leftover "${signatures_before}"
	if [[ -n ${leftover} ]]; then
		fail "retrait isolated_output: signatures residuelles=${leftover//$'\n'/ }; attendu=aucune"
		return 1
	fi
)

scenario_isolated_spawn() (
	local cleanup_failed=0
	local session="e2e-spawn-$$"
	local title="hyprpilot-e2e-iso-spawn-$$"
	local signatures_before="" signatures_ready=0 fresh_signatures=""
	local entry_x="" entry_y="" session_dir="" command_output=""
	local signature="" console_address="" console_pid="" wayland_display=""
	local found_address="" found_pid=""
	local host_workspace host_focus host_cursor_x host_cursor_y host_addresses
	# shellcheck disable=SC2034 # Sorties obligatoires de read_host_snapshot (printf -v): la session tient encore son headless, ni les monitors ni les workspaces ne sont comparables ici.
	local host_monitors host_workspaces

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_spawn() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_spawn"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_spawn EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_spawn" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_spawn: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi
	assert_named_output_absent "hyprpilot-${session}" "precondition isolated_spawn" || return 1
	read_host_snapshot host_workspace host_focus host_cursor_x host_cursor_y host_addresses \
		host_monitors host_workspaces "snapshot isolated_spawn" || return 1

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "zenity --entry --title=${title}" \
			--match-title "${title}" --size 1280x720 2>&1
	); then
		fail "session start isolated_spawn observe=echec (${command_output}); attendu=instance vivante"
		return 1
	fi

	assert_state_field "${session}" '.mode' isolated "etat isolated_spawn" || return 1
	assert_state_field "${session}" '.instance.stage' live "etat isolated_spawn" || return 1
	read_state_field "${session}" '.instance.signature' signature "etat isolated_spawn" || return 1
	read_state_field "${session}" '.instance.pid' console_pid "etat isolated_spawn" || return 1
	read_state_field "${session}" '.instance.console_address' console_address \
		"etat isolated_spawn" || return 1
	read_state_field "${session}" '.instance.wayland_display' wayland_display \
		"etat isolated_spawn" || return 1
	if [[ ! ${wayland_display} =~ ^wayland-[0-9]+$ ]]; then
		fail "etat isolated_spawn: wayland_display observe=${wayland_display}; attendu=wayland-<n>"
		return 1
	fi

	new_hypr_signatures fresh_signatures "${signatures_before}"
	if [[ ${fresh_signatures} != "${signature}"$'\n' ]]; then
		fail "isolated_spawn: signatures apparues=${fresh_signatures//$'\n'/ }; attendu=la seule ${signature}"
		return 1
	fi
	if ! nested_instance_alive "${signature}"; then
		fail "isolated_spawn: instance observe=injoignable (${signature}); attendu=vivante"
		return 1
	fi
	if ! nested_process_is_hyprland "${console_pid}"; then
		fail "isolated_spawn: process observe=${console_pid} n'est pas un Hyprland; attendu=le compositeur imbrique"
		return 1
	fi
	if ! find_console_window "agent-${session}" found_address found_pid; then
		fail "isolated_spawn: console observe=absente de agent-${session}; attendu=une fenêtre class aquamarine"
		return 1
	fi
	if [[ ${found_address} != "${console_address}" || ${found_pid} != "${console_pid}" ]]; then
		fail "isolated_spawn: console observe=${found_address} pid ${found_pid}; attendu=${console_address} pid ${console_pid}"
		return 1
	fi

	assert_host_snapshot_equals "${host_workspace}" "${host_focus}" "${host_cursor_x}" \
		"${host_cursor_y}" "${host_addresses}" - - "[\"${console_address}\"]" \
		"hote apres spawn isolated_spawn" || return 1
)

# La config nested est generee par l'outil: une cle inconnue y affiche une
# banniere d'erreur DANS le bureau agent, qui atterrit dans toute capture --full
# et serait imputee a l'app.
scenario_isolated_config_clean() (
	local cleanup_failed=0
	local session="e2e-conf-$$"
	local title="hyprpilot-e2e-iso-conf-$$"
	local signatures_before="" signatures_ready=0 sockets_before=""
	local entry_x="" entry_y="" session_dir="" command_output=""
	local signature="" errors="" compact=""

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_config_clean() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_config_clean"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_config_clean EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_config_clean" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	snapshot_wayland_sockets sockets_before
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_config_clean: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "zenity --entry --title=${title}" \
			--match-title "${title}" --size 1280x720 2>&1
	); then
		fail "session start isolated_config_clean observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	read_state_field "${session}" '.instance.signature' signature \
		"etat isolated_config_clean" || return 1

	if ! errors=$(hyprctl -i "${signature}" configerrors 2>&1); then
		fail "configerrors isolated_config_clean observe=erreur hyprctl (${errors}); attendu=sortie vide"
		return 1
	fi
	# Une config saine ne rend que du blanc (verifie sur Hyprland 0.56).
	compact=${errors//[[:space:]]/}
	if [[ -n ${compact} ]]; then
		fail "configerrors isolated_config_clean observe=${errors}; attendu=aucune erreur de config dans le bureau agent"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
		fail "teardown isolated_config_clean observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	assert_isolated_no_residue "${session}" "${signatures_before}" "${sockets_before}" \
		"teardown isolated_config_clean" || return 1
	if [[ -e ${session_dir} ]]; then
		fail "teardown isolated_config_clean: session observe=presente (${session_dir}); attendu=supprimee"
		return 1
	fi
)

scenario_isolated_teardown() (
	local cleanup_failed=0
	local session="e2e-down-$$"
	local title="hyprpilot-e2e-iso-down-$$"
	local signatures_before="" signatures_ready=0 leftover=""
	local entry_x="" entry_y="" session_dir="" command_output=""
	local signature="" nested_pid="" console_address=""
	local before_x before_y attempt process_gone=0

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_teardown() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_teardown"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_teardown EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_teardown" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_teardown: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi
	assert_named_output_absent "hyprpilot-${session}" "precondition isolated_teardown" || return 1
	move_cursor_offcenter before_x before_y "precondition curseur isolated_teardown" || return 1

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "zenity --entry --title=${title}" \
			--match-title "${title}" --size 1280x720 2>&1
	); then
		fail "session start isolated_teardown observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	read_state_field "${session}" '.instance.signature' signature \
		"etat isolated_teardown" || return 1
	read_state_field "${session}" '.instance.pid' nested_pid "etat isolated_teardown" || return 1
	read_state_field "${session}" '.instance.console_address' console_address \
		"etat isolated_teardown" || return 1
	read_stable_cursor before_x before_y "avant teardown isolated_teardown" || return 1

	if ! command_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
		fail "teardown isolated_teardown observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	assert_cursor_restored "${before_x}" "${before_y}" \
		"curseur apres teardown isolated_teardown" || return 1
	wait_nested_instance_gone "${signature}" "teardown isolated_teardown" || return 1
	for ((attempt = 0; attempt < 30; attempt++)); do
		if ! kill -0 "${nested_pid}" 2>/dev/null; then
			process_gone=1
			break
		fi
		sleep 0.1
	done
	if ((process_gone == 0)); then
		fail "teardown isolated_teardown: process observe=${nested_pid} vivant apres 3s; attendu=disparu"
		return 1
	fi
	if [[ -e ${XDG_RUNTIME_DIR}/hypr/${signature} ]]; then
		fail "teardown isolated_teardown: socket observe=present (${XDG_RUNTIME_DIR}/hypr/${signature}); attendu=supprime"
		return 1
	fi
	wait_client_gone "${console_address}" "teardown isolated_teardown console" || return 1
	wait_named_output_absent "hyprpilot-${session}" "teardown isolated_teardown" || return 1
	wait_workspace_absent "agent-${session}" "teardown isolated_teardown" || return 1
	if [[ -e ${session_dir} ]]; then
		fail "teardown isolated_teardown: session observe=presente (${session_dir}); attendu=supprimee"
		return 1
	fi
	new_hypr_signatures leftover "${signatures_before}"
	if [[ -n ${leftover} ]]; then
		fail "teardown isolated_teardown: signatures residuelles=${leftover//$'\n'/ }; attendu=aucune"
		return 1
	fi

	# Idempotence (§6.5): un second teardown sur une session deja demontee.
	if ! command_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
		if [[ ${command_output} != *"no active session"* ]]; then
			fail "second teardown isolated_teardown observe=${command_output}; attendu=succes ou no active session"
			return 1
		fi
	fi
)

# Le teardown doit emporter TOUT le bureau agent, pas seulement le compositeur:
# l'app lance ses propres descendants, qui heritent du marqueur d'environnement.
# Un survivant continue de tenir le socket du nested et de consommer la machine.
scenario_isolated_no_marked_survivor() (
	local cleanup_failed=0
	local session="e2e-marked-$$"
	local title="hyprpilot-e2e-iso-marked-$$"
	local signatures_before="" signatures_ready=0 sockets_before=""
	local entry_x="" entry_y="" session_dir="" scenario_tmp=""
	local wrapper="" sentinel="" command_output="" signature="" addresses_json=""
	local marked="" marked_count=0

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_no_marked_survivor() {
		local scenario_status=$?
		trap - EXIT INT TERM

		# Le seul motif vise est le chemin de sentinelle que le scenario a lui-meme
		# ecrit sous son repertoire temporaire.
		if [[ -n ${sentinel} ]]; then
			pkill -f -- "${sentinel}" 2>/dev/null || true
		fi
		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_no_marked_survivor"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-iso-marked.* ]]; then
				fail "nettoyage isolated_no_marked_survivor: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage isolated_no_marked_survivor: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_no_marked_survivor EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_no_marked_survivor" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	snapshot_wayland_sockets sockets_before
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_no_marked_survivor: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi
	assert_no_marked_processes "${session}" \
		"precondition isolated_no_marked_survivor" || return 1
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-iso-marked.XXXXXX"); then
		fail "isolated_no_marked_survivor: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi
	wrapper=${scenario_tmp}/app.sh
	sentinel=${scenario_tmp}/sentinel.sh

	# L'app laisse derriere elle un descendant qui ne meurt pas avec sa fenêtre:
	# un teardown qui ne signale que le PID du compositeur le laisserait vivant.
	if ! printf '#!/bin/sh\nwhile :; do sleep 1; done\n' >"${sentinel}"; then
		fail "isolated_no_marked_survivor: sentinelle observe=ecriture impossible (${sentinel}); attendu=script de sentinelle"
		return 1
	fi
	if ! printf '#!/bin/sh\nsh %s &\nexec zenity --entry --title=%s\n' \
		"${sentinel}" "${title}" >"${wrapper}"; then
		fail "isolated_no_marked_survivor: wrapper observe=ecriture impossible (${wrapper}); attendu=script d'app"
		return 1
	fi

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "sh ${wrapper}" \
			--match-title "${title}" --size 1280x720 2>&1
	); then
		fail "session start isolated_no_marked_survivor observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	read_state_field "${session}" '.instance.signature' signature \
		"etat isolated_no_marked_survivor" || return 1
	wait_nested_addresses_by_title "${signature}" addresses_json "${title}" 1 \
		"fenêtre de l'app isolated_no_marked_survivor" || return 1

	# Sans descendant marque vivant, l'assertion d'apres serait vacante.
	scan_marked_pids marked "${session}"
	marked_count=$(grep -c . <<<"${marked}") || marked_count=0
	if ((marked_count < 2)); then
		fail "isolated_no_marked_survivor: process marques observe=${marked//$'\n'/ }; attendu=le compositeur et au moins un descendant de l'app"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
		fail "teardown isolated_no_marked_survivor observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	assert_isolated_no_residue "${session}" "${signatures_before}" "${sockets_before}" \
		"teardown isolated_no_marked_survivor" || return 1
	if [[ -e ${session_dir} ]]; then
		fail "teardown isolated_no_marked_survivor: session observe=presente (${session_dir}); attendu=supprimee"
		return 1
	fi
)

scenario_isolated_app() (
	local cleanup_failed=0
	local session="e2e-app-$$"
	local title="hyprpilot-e2e-iso-app-$$"
	local signatures_before="" signatures_ready=0 leftover=""
	local entry_x="" entry_y="" session_dir="" command_output=""
	local signature="" console_address="" addresses_json="" app_address=""
	local host_workspace host_focus host_cursor_x host_cursor_y host_addresses host_monitors
	local host_workspaces

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_app() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_app"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_app EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_app" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_app: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi
	assert_named_output_absent "hyprpilot-${session}" "precondition isolated_app" || return 1
	read_host_snapshot host_workspace host_focus host_cursor_x host_cursor_y host_addresses \
		host_monitors host_workspaces "snapshot isolated_app" || return 1

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "zenity --entry --title=${title}" \
			--match-title "${title}" --size 1280x720 2>&1
	); then
		fail "session start isolated_app observe=echec (${command_output}); attendu=ready"
		return 1
	fi
	if [[ ${command_output} != *"ready"* ]]; then
		fail "session start isolated_app observe=${command_output}; attendu=message ready"
		return 1
	fi
	read_state_field "${session}" '.instance.signature' signature "etat isolated_app" || return 1
	read_state_field "${session}" '.instance.console_address' console_address \
		"etat isolated_app" || return 1

	wait_nested_addresses_by_title "${signature}" addresses_json "${title}" 1 \
		"fenêtre de l'app isolated_app" || return 1
	app_address=$(jq -er '.[0]' <<<"${addresses_json}") || return 1
	assert_state_field "${session}" '.active_address' "${app_address}" \
		"etat isolated_app" || return 1
	assert_host_snapshot_equals "${host_workspace}" "${host_focus}" "${host_cursor_x}" \
		"${host_cursor_y}" "${host_addresses}" - - "[\"${console_address}\"]" \
		"hote avec app isolated_app" || return 1

	if ! command_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
		fail "teardown isolated_app observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	wait_named_output_absent "hyprpilot-${session}" "teardown isolated_app" || return 1
	new_hypr_signatures leftover "${signatures_before}"
	if [[ -n ${leftover} ]]; then
		fail "teardown isolated_app: signatures residuelles=${leftover//$'\n'/ }; attendu=aucune"
		return 1
	fi
	assert_host_snapshot_equals "${host_workspace}" "${host_focus}" "${host_cursor_x}" \
		"${host_cursor_y}" "${host_addresses}" "${host_monitors}" "${host_workspaces}" "[]" \
		"hote apres teardown isolated_app" || return 1
)

scenario_isolated_shot() (
	local cleanup_failed=0
	local session="e2e-shot-$$"
	local title="hyprpilot-e2e-iso-shot-$$"
	local signatures_before="" signatures_ready=0 leftover=""
	local entry_x="" entry_y="" session_dir="" scenario_tmp=""
	local command_output="" shot_output=""
	local signature="" addresses_json="" app_address=""
	local window_width window_height output_width output_height

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_shot() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_shot"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-iso-shot.* ]]; then
				fail "nettoyage isolated_shot: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage isolated_shot: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_shot EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_shot" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_shot: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-iso-shot.XXXXXX"); then
		fail "isolated_shot: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "zenity --entry --title=${title}" \
			--match-title "${title}" --size 1280x720 2>&1
	); then
		fail "session start isolated_shot observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	read_state_field "${session}" '.instance.signature' signature "etat isolated_shot" || return 1
	wait_nested_addresses_by_title "${signature}" addresses_json "${title}" 1 \
		"fenêtre de l'app isolated_shot" || return 1
	app_address=$(jq -er '.[0]' <<<"${addresses_json}") || return 1
	read_nested_client_size "${signature}" "${app_address}" window_width window_height \
		"fenêtre isolated_shot" || return 1
	read_nested_output_size "${signature}" output_width output_height \
		"output nested isolated_shot" || return 1

	if ! command_output=$("${HYPRPILOT}" --session "${session}" wait --stable --timeout 5s 2>&1); then
		fail "wait --stable isolated_shot observe=echec (${command_output}); attendu=frame stabilisee"
		return 1
	fi
	if ! shot_output=$(
		"${HYPRPILOT}" --session "${session}" shot iso-window --out "${scenario_tmp}" 2>&1
	); then
		fail "shot isolated_shot observe=echec (${shot_output}); attendu=PNG de la fenêtre"
		return 1
	fi
	assert_png_dimensions "${scenario_tmp}/iso-window.png" "${window_width}" "${window_height}" \
		"shot fenêtre isolated_shot" || return 1
	if ! shot_output=$(
		"${HYPRPILOT}" --session "${session}" shot iso-full --full --out "${scenario_tmp}" 2>&1
	); then
		fail "shot --full isolated_shot observe=echec (${shot_output}); attendu=PNG du bureau agent"
		return 1
	fi
	assert_png_dimensions "${scenario_tmp}/iso-full.png" "${output_width}" "${output_height}" \
		"shot --full isolated_shot" || return 1

	if ! command_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
		fail "teardown isolated_shot observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	wait_named_output_absent "hyprpilot-${session}" "teardown isolated_shot" || return 1
	new_hypr_signatures leftover "${signatures_before}"
	if [[ -n ${leftover} ]]; then
		fail "teardown isolated_shot: signatures residuelles=${leftover//$'\n'/ }; attendu=aucune"
		return 1
	fi
)

scenario_isolated_input() (
	local cleanup_failed=0
	local session="e2e-input-$$"
	local title="hyprpilot-e2e-iso-input-$$"
	local typed_text="iso-input-$$"
	local signatures_before="" signatures_ready=0 leftover=""
	local entry_x="" entry_y="" session_dir="" scenario_tmp="" stdout_file="" wrapper=""
	local command_output="" signature="" addresses_json=""
	local before_focus before_x before_y expected_output actual_output

	assert_host_still() {
		local label=$1
		local observed_focus observed_x observed_y delta_x delta_y attempt stable_reads=0

		for ((attempt = 0; attempt < 30; attempt++)); do
			read_active_address observed_focus "${label}" || return 1
			read_cursor observed_x observed_y "${label}" || return 1
			delta_x=$((observed_x - before_x))
			delta_y=$((observed_y - before_y))
			((delta_x < 0)) && delta_x=$((-delta_x))
			((delta_y < 0)) && delta_y=$((-delta_y))
			if [[ ${observed_focus} == "${before_focus}" ]] &&
				((delta_x <= 1 && delta_y <= 1)); then
				((stable_reads += 1))
				if ((stable_reads == 3)); then
					return 0
				fi
			else
				fail "${label}: observe=focus ${observed_focus:-<aucun>}, curseur (${observed_x}, ${observed_y}); attendu=focus ${before_focus:-<aucun>}, curseur (${before_x}, ${before_y}) +/-1"
				return 1
			fi
			sleep 0.1
		done
		fail "${label}: observe=focus ${observed_focus:-<aucun>}, curseur (${observed_x}, ${observed_y}); attendu=3 lectures identiques a focus ${before_focus:-<aucun>}, curseur (${before_x}, ${before_y})"
		return 1
	}

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_input() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_input"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-iso-input.* ]]; then
				fail "nettoyage isolated_input: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage isolated_input: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_input EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_input" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_input: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-iso-input.XXXXXX"); then
		fail "isolated_input: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi
	stdout_file=${scenario_tmp}/zenity.stdout
	wrapper=${scenario_tmp}/app.sh

	# La valeur saisie est relue dans la sortie de l'app. La redirection vit
	# dans un script, pas dans --app: la commande passee reste deux mots sans
	# guillemets ni metacaractere, quelle que soit la facon dont l'outil la
	# transmet a l'instance.
	if ! printf '#!/bin/sh\nexec zenity --entry --title=%s >%s\n' \
		"${title}" "${stdout_file}" >"${wrapper}"; then
		fail "isolated_input: wrapper observe=ecriture impossible (${wrapper}); attendu=script d'app"
		return 1
	fi
	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "sh ${wrapper}" \
			--match-title "${title}" --size 1280x720 2>&1
	); then
		fail "session start isolated_input observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	read_state_field "${session}" '.instance.signature' signature "etat isolated_input" || return 1
	wait_nested_addresses_by_title "${signature}" addresses_json "${title}" 1 \
		"fenêtre de l'app isolated_input" || return 1

	read_active_address before_focus "avant input isolated_input" || return 1
	read_stable_cursor before_x before_y "avant input isolated_input" || return 1

	# (20, 20) relatif a la fenêtre: le meme point que les scenarios partages,
	# donc hors des boutons du dialogue.
	if ! command_output=$("${HYPRPILOT}" --session "${session}" click 20 20 2>&1); then
		fail "click isolated_input observe=echec (${command_output}); attendu=clic dans le bureau agent"
		return 1
	fi
	assert_host_still "hote apres click isolated_input" || return 1
	if ! command_output=$("${HYPRPILOT}" --session "${session}" scroll 20 20 --dy 1 2>&1); then
		fail "scroll isolated_input observe=echec (${command_output}); attendu=scroll dans le bureau agent"
		return 1
	fi
	assert_host_still "hote apres scroll isolated_input" || return 1
	# --focus est un no-op documente en isolé (§5): il ne doit rien bouger cote hôte.
	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" type --focus "${typed_text}" 2>&1
	); then
		fail "type isolated_input observe=echec (${command_output}); attendu=${typed_text} saisi"
		return 1
	fi
	assert_host_still "hote apres type isolated_input" || return 1
	if ! command_output=$("${HYPRPILOT}" --session "${session}" key Return 2>&1); then
		fail "key isolated_input observe=echec (${command_output}); attendu=Return accepte"
		return 1
	fi
	assert_host_still "hote apres key isolated_input" || return 1

	wait_nested_addresses_by_title "${signature}" addresses_json "${title}" 0 \
		"apres Return isolated_input" || return 1
	if [[ ! -f ${stdout_file} ]]; then
		fail "stdout isolated_input observe=absent (${stdout_file}); attendu=sortie de l'app lancee dans l'instance"
		return 1
	fi
	expected_output="${typed_text}"$'\n'
	IFS= read -r -d '' actual_output <"${stdout_file}" || true
	if [[ ${actual_output} != "${expected_output}" ]]; then
		fail "stdout isolated_input observe=${actual_output@Q}; attendu=${expected_output@Q}"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
		fail "teardown isolated_input observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	wait_named_output_absent "hyprpilot-${session}" "teardown isolated_input" || return 1
	new_hypr_signatures leftover "${signatures_before}"
	if [[ -n ${leftover} ]]; then
		fail "teardown isolated_input: signatures residuelles=${leftover//$'\n'/ }; attendu=aucune"
		return 1
	fi
)

scenario_isolated_target() (
	local cleanup_failed=0
	local session="e2e-target-$$"
	local a_title="hyprpilot-e2e-iso-target-a-$$"
	local b_title="hyprpilot-e2e-iso-target-b-$$"
	local signatures_before="" signatures_ready=0 leftover=""
	local entry_x="" entry_y="" session_dir="" scenario_tmp=""
	local command_output="" shot_output="" signature="" addresses_json=""
	local a_address="" b_address="" active_address="" windows_json=""

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_target() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_target"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-iso-target.* ]]; then
				fail "nettoyage isolated_target: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage isolated_target: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_target EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_target" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_target: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-iso-target.XXXXXX"); then
		fail "isolated_target: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "zenity --entry --title=${a_title}" \
			--match-title "${a_title}" --size 1280x720 2>&1
	); then
		fail "session start isolated_target observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	read_state_field "${session}" '.instance.signature' signature "etat isolated_target" || return 1
	wait_nested_addresses_by_title "${signature}" addresses_json "${a_title}" 1 \
		"fenêtre A isolated_target" || return 1
	a_address=$(jq -er '.[0]' <<<"${addresses_json}") || return 1

	# Le second toplevel est lance DANS l'instance (§2.6): jamais sur l'hote.
	if ! command_output=$(
		hyprctl -i "${signature}" dispatch exec "zenity --entry --title=${b_title}" 2>&1
	) || [[ ${command_output} != ok ]]; then
		fail "setup B isolated_target: dispatch exec observe=${command_output}; attendu=ok dans l'instance"
		return 1
	fi
	wait_nested_addresses_by_title "${signature}" addresses_json "${b_title}" 1 \
		"fenêtre B isolated_target" || return 1
	b_address=$(jq -er '.[0]' <<<"${addresses_json}") || return 1

	if ! shot_output=$(
		"${HYPRPILOT}" --session "${session}" shot iso-target-a --out "${scenario_tmp}" 2>&1
	) || [[ ! -s ${scenario_tmp}/iso-target-a.png ]]; then
		fail "shot A isolated_target observe=${shot_output}; attendu=PNG non vide"
		return 1
	fi

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" target --match-title "${b_title}" --wait 5s 2>&1
	); then
		fail "target B isolated_target observe=echec (${command_output}); attendu=bascule vers B"
		return 1
	fi
	assert_state_field "${session}" '.active_address' "${b_address}" \
		"etat apres target B isolated_target" || return 1
	read_nested_active_address "${signature}" active_address "target B isolated_target" || return 1
	if [[ ${active_address} != "${b_address}" ]]; then
		fail "target B isolated_target: fenetre active de l'instance observe=${active_address}; attendu=${b_address}"
		return 1
	fi
	assert_nested_no_parking "target B isolated_target" || return 1
	if ! shot_output=$(
		"${HYPRPILOT}" --session "${session}" shot iso-target-b --out "${scenario_tmp}" 2>&1
	) || [[ ! -s ${scenario_tmp}/iso-target-b.png ]]; then
		fail "shot B isolated_target observe=${shot_output}; attendu=PNG non vide"
		return 1
	fi
	if cmp -s -- "${scenario_tmp}/iso-target-a.png" "${scenario_tmp}/iso-target-b.png"; then
		fail "captures isolated_target observe=identiques; attendu=deux toplevels distincts"
		return 1
	fi

	if ! windows_json=$("${HYPRPILOT}" --session "${session}" windows 2>&1); then
		fail "windows isolated_target observe=echec (${windows_json}); attendu=tableau JSON"
		return 1
	fi
	if ! jq -e --arg a "${a_address}" --arg b "${b_address}" \
		'type == "array" and ([.[].address] | sort) == ([$a, $b] | sort)' \
		<<<"${windows_json}" >/dev/null; then
		fail "windows isolated_target observe=${windows_json}; attendu=les deux fenêtres de l'instance (${a_address}, ${b_address})"
		return 1
	fi

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" target --match-title "${a_title}" 2>&1
	); then
		fail "target A isolated_target observe=echec (${command_output}); attendu=retour sur A"
		return 1
	fi
	read_nested_active_address "${signature}" active_address "target A isolated_target" || return 1
	if [[ ${active_address} != "${a_address}" ]]; then
		fail "target A isolated_target: fenetre active de l'instance observe=${active_address}; attendu=${a_address}"
		return 1
	fi
	assert_nested_no_parking "target A isolated_target" || return 1

	if ! command_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
		fail "teardown isolated_target observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	wait_named_output_absent "hyprpilot-${session}" "teardown isolated_target" || return 1
	new_hypr_signatures leftover "${signatures_before}"
	if [[ -n ${leftover} ]]; then
		fail "teardown isolated_target: signatures residuelles=${leftover//$'\n'/ }; attendu=aucune"
		return 1
	fi
)

scenario_isolated_show_hide() (
	local cleanup_failed=0
	local session="e2e-show-$$"
	local title="hyprpilot-e2e-iso-show-$$"
	local signatures_before="" signatures_ready=0 leftover=""
	local entry_x="" entry_y="" session_dir="" scenario_tmp=""
	local command_output="" shot_output="" signature="" console_address=""
	local addresses_json="" app_address="" window_width window_height
	local host_workspace host_focus host_cursor_x host_cursor_y host_addresses host_monitors
	local host_workspaces
	# shellcheck disable=SC2034 # Sorties obligatoires de read_client_state (printf -v): seuls workspace et floating sont relus.
	local console_x console_y console_width console_height console_monitor console_workspace
	local console_floating

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_show_hide() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_show_hide"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-iso-show.* ]]; then
				fail "nettoyage isolated_show_hide: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage isolated_show_hide: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_show_hide EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_show_hide" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_show_hide: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-iso-show.XXXXXX"); then
		fail "isolated_show_hide: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "zenity --entry --title=${title}" \
			--match-title "${title}" --size 1280x720 2>&1
	); then
		fail "session start isolated_show_hide observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	read_state_field "${session}" '.instance.signature' signature \
		"etat isolated_show_hide" || return 1
	read_state_field "${session}" '.instance.console_address' console_address \
		"etat isolated_show_hide" || return 1
	assert_state_field "${session}" '.shown' false "etat isolated_show_hide" || return 1
	wait_nested_addresses_by_title "${signature}" addresses_json "${title}" 1 \
		"fenêtre de l'app isolated_show_hide" || return 1
	app_address=$(jq -er '.[0]' <<<"${addresses_json}") || return 1
	read_nested_client_size "${signature}" "${app_address}" window_width window_height \
		"fenêtre isolated_show_hide" || return 1
	read_host_snapshot host_workspace host_focus host_cursor_x host_cursor_y host_addresses \
		host_monitors host_workspaces "snapshot isolated_show_hide" || return 1

	if ! command_output=$("${HYPRPILOT}" --session "${session}" session show 2>&1); then
		fail "session show isolated_show_hide observe=echec (${command_output}); attendu=console sur ${host_workspace}"
		return 1
	fi
	wait_console_workspace "${host_workspace}" "${console_address}" \
		"session show isolated_show_hide" || return 1
	read_client_state "${console_address}" console_x console_y console_width console_height \
		console_workspace console_floating console_monitor \
		"console apres show isolated_show_hide" || return 1
	if [[ ${console_floating} != true ]]; then
		fail "session show isolated_show_hide: console observe=floating ${console_floating}; attendu=true"
		return 1
	fi
	# Console partie, le workspace agent doit rester celui du headless: sinon le
	# `hide` qui suit le recree sur le moniteur courant de la console.
	assert_agent_workspace_active "${session}" "apres show isolated_show_hide" || return 1
	assert_state_field "${session}" '.shown' true "etat apres show isolated_show_hide" || return 1
	if ! shot_output=$(
		"${HYPRPILOT}" --session "${session}" shot iso-shown --out "${scenario_tmp}" 2>&1
	); then
		fail "shot pendant show isolated_show_hide observe=echec (${shot_output}); attendu=capture valide"
		return 1
	fi
	assert_png_dimensions "${scenario_tmp}/iso-shown.png" "${window_width}" "${window_height}" \
		"shot pendant show isolated_show_hide" || return 1

	if ! command_output=$("${HYPRPILOT}" --session "${session}" session hide 2>&1); then
		fail "session hide isolated_show_hide observe=echec (${command_output}); attendu=console sur agent-${session}"
		return 1
	fi
	wait_console_workspace "agent-${session}" "${console_address}" \
		"session hide isolated_show_hide" || return 1
	# Le workspace de retour doit etre celui du headless, pas un homonyme recree
	# sur le moniteur ou la console etait posee (fait §2.3).
	assert_agent_workspace_active "${session}" "apres hide isolated_show_hide" || return 1
	assert_state_field "${session}" '.shown' false "etat apres hide isolated_show_hide" || return 1
	if ! shot_output=$(
		"${HYPRPILOT}" --session "${session}" shot iso-hidden --out "${scenario_tmp}" 2>&1
	); then
		fail "shot apres hide isolated_show_hide observe=echec (${shot_output}); attendu=capture toujours valide"
		return 1
	fi
	assert_png_dimensions "${scenario_tmp}/iso-hidden.png" "${window_width}" "${window_height}" \
		"shot apres hide isolated_show_hide" || return 1
	assert_host_snapshot_equals "${host_workspace}" "${host_focus}" "${host_cursor_x}" \
		"${host_cursor_y}" "${host_addresses}" - - "[]" \
		"hote apres hide isolated_show_hide" || return 1

	if ! command_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
		fail "teardown isolated_show_hide observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	wait_named_output_absent "hyprpilot-${session}" "teardown isolated_show_hide" || return 1
	new_hypr_signatures leftover "${signatures_before}"
	if [[ -n ${leftover} ]]; then
		fail "teardown isolated_show_hide: signatures residuelles=${leftover//$'\n'/ }; attendu=aucune"
		return 1
	fi
)

# Console montree, puis l'utilisateur change de workspace: la console n'est plus
# composee, le nested cesse de recevoir des frames (fait §2.2) et toute capture
# bloquerait. La capture doit refuser IMMEDIATEMENT en nommant la console
# occluse. Un timeout de 5 s, ou un diagnostic qui parle du workspace du
# headless, signale un code qui a cru le drapeau `shown` au lieu d'observer la
# composition.
scenario_isolated_show_occluded() (
	local cleanup_failed=0
	local session="e2e-occl-$$"
	local title="hyprpilot-e2e-iso-occl-$$"
	local decoy="hyprpilot-e2e-occl-$$"
	local signatures_before="" signatures_ready=0 sockets_before=""
	local entry_x="" entry_y="" session_dir="" scenario_tmp=""
	local command_output="" shot_output="" lowered="" signature="" console_address=""
	local restore_workspace="" started=0 finished=0 elapsed=0
	local host_workspace host_focus host_cursor_x host_cursor_y host_addresses host_monitors
	local host_workspaces
	# shellcheck disable=SC2034 # Sorties obligatoires de move_cursor_offcenter (printf -v); la reference reste le snapshot.
	local offcenter_x offcenter_y

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_show_occluded() {
		local scenario_status=$?
		trap - EXIT INT TERM

		# Le workspace de l'utilisateur revient d'abord: c'est la seule mutation
		# hôte que le scenario a demandee.
		restore_workspace_best_effort "${restore_workspace}"
		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_show_occluded"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-iso-occl.* ]]; then
				fail "nettoyage isolated_show_occluded: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage isolated_show_occluded: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_show_occluded EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_show_occluded" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	snapshot_wayland_sockets sockets_before
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_show_occluded: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi
	assert_workspace_absent "${decoy}" "precondition isolated_show_occluded" || return 1
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-iso-occl.XXXXXX"); then
		fail "isolated_show_occluded: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi
	move_cursor_offcenter offcenter_x offcenter_y \
		"precondition curseur isolated_show_occluded" || return 1
	read_host_snapshot host_workspace host_focus host_cursor_x host_cursor_y host_addresses \
		host_monitors host_workspaces "snapshot avant isolated_show_occluded" || return 1

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "zenity --entry --title=${title}" \
			--match-title "${title}" --size 1280x720 2>&1
	); then
		fail "session start isolated_show_occluded observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	read_state_field "${session}" '.instance.signature' signature \
		"etat isolated_show_occluded" || return 1
	read_state_field "${session}" '.instance.console_address' console_address \
		"etat isolated_show_occluded" || return 1

	if ! command_output=$("${HYPRPILOT}" --session "${session}" session show 2>&1); then
		fail "session show isolated_show_occluded observe=echec (${command_output}); attendu=console sur ${host_workspace}"
		return 1
	fi
	wait_console_workspace "${host_workspace}" "${console_address}" \
		"session show isolated_show_occluded" || return 1
	# Reference: console visible, la capture passe.
	if ! shot_output=$(
		"${HYPRPILOT}" --session "${session}" shot iso-visible --out "${scenario_tmp}" 2>&1
	) || [[ ! -s ${scenario_tmp}/iso-visible.png ]]; then
		fail "shot console visible isolated_show_occluded observe=${shot_output}; attendu=PNG non vide"
		return 1
	fi

	# L'utilisateur change de workspace. La bascule vise un workspace nomme que le
	# scenario cree, jamais un des workspaces de l'utilisateur.
	restore_workspace=${host_workspace}
	switch_host_workspace "${decoy}" "occlusion isolated_show_occluded" || return 1
	wait_console_workspace "${host_workspace}" "${console_address}" \
		"console occluse isolated_show_occluded" || return 1

	now_ms started "shot occlus isolated_show_occluded" || return 1
	if shot_output=$(
		"${HYPRPILOT}" --session "${session}" shot iso-occluded --out "${scenario_tmp}" 2>&1
	); then
		fail "shot pendant occlusion isolated_show_occluded observe=succes (${shot_output}); attendu=exit non nul"
		return 1
	fi
	now_ms finished "shot occlus isolated_show_occluded" || return 1
	elapsed=$((finished - started))
	if ((elapsed >= 3000)); then
		fail "shot pendant occlusion isolated_show_occluded: refus observe=apres ${elapsed} ms (${shot_output}); attendu=refus immediat, moins de 3000 ms, pas un timeout de capture"
		return 1
	fi
	lowered=${shot_output,,}
	if [[ ${shot_output} != *"${console_address}"* && ${lowered} != *console* ]]; then
		fail "shot pendant occlusion isolated_show_occluded: message observe=${shot_output}; attendu=la console occluse nommee (${console_address})"
		return 1
	fi
	if [[ ${shot_output} == *"active one on host output"* ]]; then
		fail "shot pendant occlusion isolated_show_occluded: message observe=${shot_output}; attendu=le diagnostic de la console occluse, pas celui du workspace du headless"
		return 1
	fi

	# L'utilisateur revient: le bureau agent doit redevenir capturable, l'echec
	# precedent etait un refus, pas une casse.
	switch_host_workspace "${host_workspace}" "retour isolated_show_occluded" || return 1
	restore_workspace=""
	wait_workspace_absent "${decoy}" "retour isolated_show_occluded" || return 1
	if ! command_output=$("${HYPRPILOT}" --session "${session}" session hide 2>&1); then
		fail "session hide isolated_show_occluded observe=echec (${command_output}); attendu=console sur agent-${session}"
		return 1
	fi
	wait_console_workspace "agent-${session}" "${console_address}" \
		"session hide isolated_show_occluded" || return 1
	assert_agent_workspace_active "${session}" "apres hide isolated_show_occluded" || return 1
	if ! shot_output=$(
		"${HYPRPILOT}" --session "${session}" shot iso-again --out "${scenario_tmp}" 2>&1
	) || [[ ! -s ${scenario_tmp}/iso-again.png ]]; then
		fail "shot apres retour isolated_show_occluded observe=${shot_output}; attendu=PNG non vide"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
		fail "teardown isolated_show_occluded observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	assert_isolated_no_residue "${session}" "${signatures_before}" "${sockets_before}" \
		"teardown isolated_show_occluded" || return 1
	wait_workspace_absent "agent-${session}" "teardown isolated_show_occluded" || return 1
	# Le focus revient au workspace d'origine par la memoire de Hyprland, le
	# scenario ne refocalise aucune fenêtre de l'utilisateur.
	assert_host_snapshot_equals "${host_workspace}" "${host_focus}" "${host_cursor_x}" \
		"${host_cursor_y}" "${host_addresses}" "${host_monitors}" "${host_workspaces}" "[]" \
		"hote apres isolated_show_occluded" || return 1
)

scenario_isolated_status() (
	local cleanup_failed=0
	local session="e2e-status-$$"
	local title="hyprpilot-e2e-iso-status-$$"
	local signatures_before="" signatures_ready=0 leftover=""
	local entry_x="" entry_y="" session_dir="" command_output="" status_json=""
	local signature="" nested_pid="" wayland_display="" lowered=""
	local attempt process_gone=0

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_status() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_status"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_status EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_status" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_status: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "zenity --entry --title=${title}" \
			--match-title "${title}" --size 1280x720 2>&1
	); then
		fail "session start isolated_status observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	read_state_field "${session}" '.instance.signature' signature "etat isolated_status" || return 1
	read_state_field "${session}" '.instance.pid' nested_pid "etat isolated_status" || return 1
	read_state_field "${session}" '.instance.wayland_display' wayland_display \
		"etat isolated_status" || return 1

	if ! status_json=$("${HYPRPILOT}" --session "${session}" status 2>&1); then
		fail "status isolated_status observe=echec (${status_json}); attendu=JSON de session isolée"
		return 1
	fi
	if ! jq -e --arg session "${session}" \
		'.mode == "isolated" and .session == $session and .shown == false' \
		<<<"${status_json}" >/dev/null; then
		fail "status isolated_status observe=${status_json}; attendu=mode isolated, session ${session}, shown false"
		return 1
	fi
	if ! jq -e --arg signature "${signature}" --arg display "${wayland_display}" \
		'tostring | contains($signature) and contains($display)' \
		<<<"${status_json}" >/dev/null; then
		fail "status isolated_status observe=${status_json}; attendu=signature ${signature} et display ${wayland_display}"
		return 1
	fi

	# Kill brutal du compositeur imbrique, par un PID confirme via /proc.
	if ! nested_process_is_hyprland "${nested_pid}"; then
		fail "kill isolated_status: process observe=${nested_pid} n'est pas un Hyprland; attendu=le compositeur imbrique"
		return 1
	fi
	if ! nested_pid_is_console "${nested_pid}"; then
		fail "kill isolated_status: process observe=${nested_pid} sans fenêtre console cote hote; attendu=le compositeur imbrique de la session"
		return 1
	fi
	if ! kill -KILL "${nested_pid}" 2>/dev/null; then
		fail "kill isolated_status: signal observe=echec sur ${nested_pid}; attendu=SIGKILL delivre"
		return 1
	fi
	for ((attempt = 0; attempt < 30; attempt++)); do
		if ! kill -0 "${nested_pid}" 2>/dev/null; then
			process_gone=1
			break
		fi
		sleep 0.1
	done
	if ((process_gone == 0)); then
		fail "kill isolated_status: process observe=${nested_pid} vivant apres 3s; attendu=mort"
		return 1
	fi
	wait_nested_instance_gone "${signature}" "kill isolated_status" || return 1

	if command_output=$("${HYPRPILOT}" --session "${session}" status 2>&1); then
		fail "status apres kill isolated_status observe=succes (${command_output}); attendu=exit non nul"
		return 1
	fi
	lowered=${command_output,,}
	if [[ ${lowered} != *instance* || ${lowered} != *teardown* ]]; then
		fail "status apres kill isolated_status message observe=${command_output}; attendu=instance morte et sortie par teardown"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
		fail "teardown apres kill isolated_status observe=echec (${command_output}); attendu=succes idempotent"
		return 1
	fi
	wait_named_output_absent "hyprpilot-${session}" "teardown isolated_status" || return 1
	if [[ -e ${session_dir} ]]; then
		fail "teardown isolated_status: session observe=presente (${session_dir}); attendu=supprimee"
		return 1
	fi
	new_hypr_signatures leftover "${signatures_before}"
	if [[ -n ${leftover} ]]; then
		fail "teardown isolated_status: signatures residuelles=${leftover//$'\n'/ }; attendu=aucune"
		return 1
	fi
)

# Compositeur imbrique tue brutalement: aucune commande ne doit plus rien
# envoyer nulle part (surtout pas au bureau de l'utilisateur), et le teardown
# doit quand meme rendre l'output, le repertoire d'instance, le socket Wayland
# (que le SIGKILL laisse derriere, fait §2.9) et l'etat.
scenario_isolated_nested_killed() (
	local cleanup_failed=0
	local session="e2e-killed-$$"
	local title="hyprpilot-e2e-iso-killed-$$"
	local signatures_before="" signatures_ready=0 sockets_before=""
	local entry_x="" entry_y="" session_dir="" scenario_tmp="" command_output=""
	local signature="" nested_pid="" wayland_display=""
	local before_x="" before_y="" attempt process_gone=0

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_nested_killed() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_nested_killed"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-iso-killed.* ]]; then
				fail "nettoyage isolated_nested_killed: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage isolated_nested_killed: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	# Toute commande de la session doit refuser en nommant l'instance morte.
	assert_dead_command() {
		local label=$1
		shift
		local output="" lowered=""

		if output=$("${HYPRPILOT}" --session "${session}" "$@" 2>&1); then
			fail "${label} isolated_nested_killed observe=succes (${output}); attendu=exit non nul"
			return 1
		fi
		lowered=${output,,}
		if [[ ${output} != *"${signature}"* || ${lowered} != *teardown* ]] ||
			[[ ${lowered} != *dead* && ${lowered} != *gone* ]]; then
			fail "${label} isolated_nested_killed message observe=${output}; attendu=instance ${signature} morte et sortie par teardown"
			return 1
		fi
	}

	trap cleanup_isolated_nested_killed EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition isolated_nested_killed" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	snapshot_wayland_sockets sockets_before
	if [[ -e ${session_dir} ]]; then
		fail "precondition isolated_nested_killed: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-iso-killed.XXXXXX"); then
		fail "isolated_nested_killed: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi
	move_cursor_offcenter before_x before_y \
		"precondition curseur isolated_nested_killed" || return 1

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "zenity --entry --title=${title}" \
			--match-title "${title}" --size 1280x720 2>&1
	); then
		fail "session start isolated_nested_killed observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	read_state_field "${session}" '.instance.signature' signature \
		"etat isolated_nested_killed" || return 1
	read_state_field "${session}" '.instance.pid' nested_pid \
		"etat isolated_nested_killed" || return 1
	read_state_field "${session}" '.instance.wayland_display' wayland_display \
		"etat isolated_nested_killed" || return 1
	if [[ ! ${wayland_display} =~ ^wayland-[0-9]+$ ]]; then
		fail "etat isolated_nested_killed: wayland_display observe=${wayland_display}; attendu=wayland-<n>"
		return 1
	fi

	# Le SIGKILL ne vise que le PID lu dans l'etat de la session que le scenario
	# vient de creer, et seulement apres confirmation par /proc et par la fenêtre
	# console cote hote.
	if ! nested_process_is_hyprland "${nested_pid}"; then
		fail "kill isolated_nested_killed: process observe=${nested_pid} n'est pas un Hyprland; attendu=le compositeur imbrique"
		return 1
	fi
	if ! nested_pid_is_console "${nested_pid}"; then
		fail "kill isolated_nested_killed: process observe=${nested_pid} sans fenêtre console cote hote; attendu=le compositeur imbrique de la session"
		return 1
	fi
	if ! kill -KILL "${nested_pid}" 2>/dev/null; then
		fail "kill isolated_nested_killed: signal observe=echec sur ${nested_pid}; attendu=SIGKILL delivre"
		return 1
	fi
	for ((attempt = 0; attempt < 30; attempt++)); do
		if ! kill -0 "${nested_pid}" 2>/dev/null; then
			process_gone=1
			break
		fi
		sleep 0.1
	done
	if ((process_gone == 0)); then
		fail "kill isolated_nested_killed: process observe=${nested_pid} vivant apres 3s; attendu=mort"
		return 1
	fi
	wait_nested_instance_gone "${signature}" "kill isolated_nested_killed" || return 1

	assert_dead_command "status" status || return 1
	assert_dead_command "windows" windows || return 1
	assert_dead_command "shot" shot iso-dead --out "${scenario_tmp}" || return 1
	assert_dead_command "wait" wait --stable --timeout 5s || return 1
	assert_dead_command "click" click 20 20 || return 1
	assert_dead_command "scroll" scroll 20 20 --dy 1 || return 1
	assert_dead_command "type" type "dead-$$" || return 1
	assert_dead_command "key" key Return || return 1
	assert_dead_command "target" target --match-title "${title}" || return 1
	assert_dead_command "session show" session show || return 1
	assert_dead_command "session hide" session hide || return 1

	read_stable_cursor before_x before_y "avant teardown isolated_nested_killed" || return 1
	if ! command_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
		fail "teardown isolated_nested_killed observe=echec (${command_output}); attendu=succes malgre l'instance morte"
		return 1
	fi
	assert_cursor_restored "${before_x}" "${before_y}" \
		"curseur apres teardown isolated_nested_killed" || return 1
	wait_named_output_absent "hyprpilot-${session}" "teardown isolated_nested_killed" || return 1
	wait_workspace_absent "agent-${session}" "teardown isolated_nested_killed" || return 1
	if [[ -e ${XDG_RUNTIME_DIR}/hypr/${signature} ]]; then
		fail "teardown isolated_nested_killed: repertoire d'instance observe=present (${XDG_RUNTIME_DIR}/hypr/${signature}); attendu=supprime"
		return 1
	fi
	if [[ -e ${XDG_RUNTIME_DIR}/${wayland_display} ||
		-e ${XDG_RUNTIME_DIR}/${wayland_display}.lock ]]; then
		fail "teardown isolated_nested_killed: socket observe=present (${XDG_RUNTIME_DIR}/${wayland_display}); attendu=supprime avec son verrou"
		return 1
	fi
	if [[ -e ${session_dir} ]]; then
		fail "teardown isolated_nested_killed: session observe=presente (${session_dir}); attendu=supprimee"
		return 1
	fi
	assert_isolated_no_residue "${session}" "${signatures_before}" "${sockets_before}" \
		"teardown isolated_nested_killed" || return 1
)

scenario_host_intact() (
	local cleanup_failed=0
	local session="e2e-host-$$"
	local title="hyprpilot-e2e-iso-host-$$"
	local typed_text="iso-host-$$"
	local signatures_before="" signatures_ready=0 leftover=""
	local entry_x="" entry_y="" session_dir="" scenario_tmp=""
	local command_output="" shot_output="" signature="" addresses_json=""
	local host_workspace host_focus host_cursor_x host_cursor_y host_addresses host_monitors
	local host_workspaces
	# shellcheck disable=SC2034 # Sorties obligatoires de move_cursor_offcenter (printf -v); la reference reste le snapshot.
	local offcenter_x offcenter_y

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_host_intact() {
		local scenario_status=$?
		trap - EXIT INT TERM

		if ! isolated_raw_cleanup "${session}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage host_intact"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-host-intact.* ]]; then
				fail "nettoyage host_intact: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage host_intact: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_host_intact EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	session_dir=$(session_dir_path "${session}")
	read_cursor entry_x entry_y "precondition host_intact" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	if [[ -e ${session_dir} ]]; then
		fail "precondition host_intact: session observe=presente (${session_dir}); attendu=absente"
		return 1
	fi
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-host-intact.XXXXXX"); then
		fail "host_intact: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi

	# Curseur excentre AVANT le snapshot de reference: pose au centre du moniteur
	# restant, il y serait deja remis par le recentrage de `output remove` (fait
	# §2.8) et l'assertion de restauration serait vacante.
	move_cursor_offcenter offcenter_x offcenter_y "precondition curseur host_intact" || return 1
	# Snapshot de reference: tout le scenario doit laisser le bureau utilisateur
	# exactement ou il est, curseur compris.
	read_host_snapshot host_workspace host_focus host_cursor_x host_cursor_y host_addresses \
		host_monitors host_workspaces "snapshot avant host_intact" || return 1

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session}" session start --isolated \
			--app "zenity --entry --title=${title}" \
			--match-title "${title}" --size 1280x720 2>&1
	); then
		fail "session start host_intact observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	read_state_field "${session}" '.instance.signature' signature "etat host_intact" || return 1
	wait_nested_addresses_by_title "${signature}" addresses_json "${title}" 1 \
		"fenêtre de l'app host_intact" || return 1

	if ! command_output=$("${HYPRPILOT}" --session "${session}" click 20 20 2>&1); then
		fail "click host_intact observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	if ! command_output=$("${HYPRPILOT}" --session "${session}" scroll 20 20 --dy 1 2>&1); then
		fail "scroll host_intact observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	if ! command_output=$("${HYPRPILOT}" --session "${session}" type "${typed_text}" 2>&1); then
		fail "type host_intact observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	if ! command_output=$("${HYPRPILOT}" --session "${session}" key Tab 2>&1); then
		fail "key host_intact observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	if ! command_output=$("${HYPRPILOT}" --session "${session}" wait --stable --timeout 5s 2>&1); then
		fail "wait host_intact observe=echec (${command_output}); attendu=frame stabilisee"
		return 1
	fi
	if ! shot_output=$(
		"${HYPRPILOT}" --session "${session}" shot iso-host --out "${scenario_tmp}" 2>&1
	) || [[ ! -s ${scenario_tmp}/iso-host.png ]]; then
		fail "shot host_intact observe=${shot_output}; attendu=PNG non vide"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" --session "${session}" teardown 2>&1); then
		fail "teardown host_intact observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	wait_named_output_absent "hyprpilot-${session}" "teardown host_intact" || return 1
	wait_workspace_absent "agent-${session}" "teardown host_intact" || return 1
	new_hypr_signatures leftover "${signatures_before}"
	if [[ -n ${leftover} ]]; then
		fail "teardown host_intact: signatures residuelles=${leftover//$'\n'/ }; attendu=aucune"
		return 1
	fi
	assert_host_snapshot_equals "${host_workspace}" "${host_focus}" "${host_cursor_x}" \
		"${host_cursor_y}" "${host_addresses}" "${host_monitors}" "${host_workspaces}" "[]" \
		"snapshot apres host_intact" || return 1
)

scenario_isolated_parallel() (
	local cleanup_failed=0
	local session_a="e2e-par-a-$$"
	local session_b="e2e-par-b-$$"
	local a_title="hyprpilot-e2e-iso-par-a-$$"
	local b_title="hyprpilot-e2e-iso-par-b-$$"
	local signatures_before="" signatures_ready=0 leftover=""
	local entry_x="" entry_y="" dir_a="" dir_b="" scenario_tmp=""
	local command_output="" shot_output="" a_signature="" b_signature=""
	local a_console="" b_console="" addresses_json=""
	local host_workspace host_focus host_cursor_x host_cursor_y host_addresses host_monitors
	local host_workspaces

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_parallel() {
		local scenario_status=$?
		trap - EXIT INT TERM

		# Le premier appel termine toutes les instances apparues, le second ne
		# s'occupe que des ressources nommees de la seconde session.
		if ! isolated_raw_cleanup "${session_a}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_parallel A"; then
			cleanup_failed=1
		fi
		if ! isolated_raw_cleanup "${session_b}" 0 "" "nettoyage isolated_parallel B"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-iso-par.* ]]; then
				fail "nettoyage isolated_parallel: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage isolated_parallel: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_parallel EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	dir_a=$(session_dir_path "${session_a}")
	dir_b=$(session_dir_path "${session_b}")
	read_cursor entry_x entry_y "precondition isolated_parallel" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	if [[ -e ${dir_a} || -e ${dir_b} ]]; then
		fail "precondition isolated_parallel: sessions observe=presentes (${dir_a}, ${dir_b}); attendu=absentes"
		return 1
	fi
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-iso-par.XXXXXX"); then
		fail "isolated_parallel: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi
	read_host_snapshot host_workspace host_focus host_cursor_x host_cursor_y host_addresses \
		host_monitors host_workspaces "snapshot avant isolated_parallel" || return 1

	if ! command_output=$(
		"${HYPRPILOT}" --session "${session_a}" session start --isolated \
			--app "zenity --entry --title=${a_title}" \
			--match-title "${a_title}" --size 800x600 2>&1
	); then
		fail "session start A isolated_parallel observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	if ! command_output=$(
		"${HYPRPILOT}" --session "${session_b}" session start --isolated \
			--app "zenity --entry --title=${b_title}" \
			--match-title "${b_title}" --size 800x600 2>&1
	); then
		fail "session start B isolated_parallel observe=echec (${command_output}); attendu=succes en parallèle de A"
		return 1
	fi
	read_state_field "${session_a}" '.instance.signature' a_signature \
		"etat A isolated_parallel" || return 1
	read_state_field "${session_b}" '.instance.signature' b_signature \
		"etat B isolated_parallel" || return 1
	read_state_field "${session_a}" '.instance.console_address' a_console \
		"etat A isolated_parallel" || return 1
	read_state_field "${session_b}" '.instance.console_address' b_console \
		"etat B isolated_parallel" || return 1
	if [[ ${a_signature} == "${b_signature}" || ${a_console} == "${b_console}" ]]; then
		fail "isolated_parallel: instances observe=signature ${a_signature}/${b_signature}, console ${a_console}/${b_console}; attendu=deux instances distinctes"
		return 1
	fi

	# Chaque instance ne voit que sa propre app: l'isolation porte sur les
	# clients, pas seulement sur les dossiers d'etat.
	wait_nested_addresses_by_title "${a_signature}" addresses_json "${a_title}" 1 \
		"fenêtre A isolated_parallel" || return 1
	wait_nested_addresses_by_title "${b_signature}" addresses_json "${b_title}" 1 \
		"fenêtre B isolated_parallel" || return 1
	wait_nested_addresses_by_title "${a_signature}" addresses_json "${b_title}" 0 \
		"etancheite A isolated_parallel" || return 1
	wait_nested_addresses_by_title "${b_signature}" addresses_json "${a_title}" 0 \
		"etancheite B isolated_parallel" || return 1

	# Actions croisees.
	if ! command_output=$("${HYPRPILOT}" --session "${session_a}" type "a-$$" 2>&1); then
		fail "type A isolated_parallel observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	if ! command_output=$("${HYPRPILOT}" --session "${session_b}" click 20 20 2>&1); then
		fail "click B isolated_parallel observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	if ! command_output=$("${HYPRPILOT}" --session "${session_a}" scroll 20 20 --dy 1 2>&1); then
		fail "scroll A isolated_parallel observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	if ! command_output=$("${HYPRPILOT}" --session "${session_b}" type "b-$$" 2>&1); then
		fail "type B isolated_parallel observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	if ! shot_output=$(
		"${HYPRPILOT}" --session "${session_a}" shot par-a --out "${scenario_tmp}" 2>&1
	) || [[ ! -s ${scenario_tmp}/par-a.png ]]; then
		fail "shot A isolated_parallel observe=${shot_output}; attendu=PNG non vide"
		return 1
	fi
	if ! shot_output=$(
		"${HYPRPILOT}" --session "${session_b}" shot par-b --out "${scenario_tmp}" 2>&1
	) || [[ ! -s ${scenario_tmp}/par-b.png ]]; then
		fail "shot B isolated_parallel observe=${shot_output}; attendu=PNG non vide"
		return 1
	fi
	if cmp -s -- "${scenario_tmp}/par-a.png" "${scenario_tmp}/par-b.png"; then
		fail "captures isolated_parallel observe=identiques; attendu=deux bureaux agents distincts"
		return 1
	fi

	# Teardown A: B doit rester intacte et pilotable.
	if ! command_output=$("${HYPRPILOT}" --session "${session_a}" teardown 2>&1); then
		fail "teardown A isolated_parallel observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	wait_nested_instance_gone "${a_signature}" "teardown A isolated_parallel" || return 1
	wait_named_output_absent "hyprpilot-${session_a}" "teardown A isolated_parallel" || return 1
	wait_workspace_absent "agent-${session_a}" "teardown A isolated_parallel" || return 1
	if [[ -e ${dir_a} ]]; then
		fail "teardown A isolated_parallel: session observe=presente (${dir_a}); attendu=supprimee"
		return 1
	fi
	if [[ -e ${XDG_RUNTIME_DIR}/hypr/${a_signature} ]]; then
		fail "teardown A isolated_parallel: socket observe=present (${XDG_RUNTIME_DIR}/hypr/${a_signature}); attendu=supprime"
		return 1
	fi
	if ! nested_instance_alive "${b_signature}"; then
		fail "teardown A isolated_parallel: instance B observe=injoignable (${b_signature}); attendu=intacte"
		return 1
	fi
	if [[ ! -e ${dir_b} ]]; then
		fail "teardown A isolated_parallel: session B observe=absente (${dir_b}); attendu=intacte"
		return 1
	fi
	if ! named_output_present "hyprpilot-${session_b}"; then
		fail "teardown A isolated_parallel: output B observe=absent; attendu=hyprpilot-${session_b} intact"
		return 1
	fi
	wait_nested_addresses_by_title "${b_signature}" addresses_json "${b_title}" 1 \
		"fenêtre B apres teardown A isolated_parallel" || return 1
	if ! shot_output=$(
		"${HYPRPILOT}" --session "${session_b}" shot par-b-apres --out "${scenario_tmp}" 2>&1
	) || [[ ! -s ${scenario_tmp}/par-b-apres.png ]]; then
		fail "shot B apres teardown A isolated_parallel observe=${shot_output}; attendu=PNG non vide"
		return 1
	fi

	if ! command_output=$("${HYPRPILOT}" --session "${session_b}" teardown 2>&1); then
		fail "teardown B isolated_parallel observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	wait_nested_instance_gone "${b_signature}" "teardown B isolated_parallel" || return 1
	wait_named_output_absent "hyprpilot-${session_b}" "teardown B isolated_parallel" || return 1
	wait_workspace_absent "agent-${session_b}" "teardown B isolated_parallel" || return 1
	if [[ -e ${dir_b} ]]; then
		fail "teardown B isolated_parallel: session observe=presente (${dir_b}); attendu=supprimee"
		return 1
	fi
	if [[ -e ${XDG_RUNTIME_DIR}/hypr/${b_signature} ]]; then
		fail "teardown B isolated_parallel: socket observe=present (${XDG_RUNTIME_DIR}/hypr/${b_signature}); attendu=supprime"
		return 1
	fi
	new_hypr_signatures leftover "${signatures_before}"
	if [[ -n ${leftover} ]]; then
		fail "teardown isolated_parallel: signatures residuelles=${leftover//$'\n'/ }; attendu=aucune"
		return 1
	fi
	assert_host_snapshot_equals "${host_workspace}" "${host_focus}" "${host_cursor_x}" \
		"${host_cursor_y}" "${host_addresses}" "${host_monitors}" "${host_workspaces}" "[]" \
		"snapshot apres isolated_parallel" || return 1
)

# Deux starts REELLEMENT concurrents: c'est le chevauchement des fenêtres de
# decouverte (nouvelle signature sous hypr/, nouveau socket Wayland sous
# $XDG_RUNTIME_DIR) qui est en cause, un enchainement sequentiel ne le reproduit
# pas. Chaque session doit enregistrer le socket de SON instance: un display
# echange envoie les captures et le pointeur virtuel dans le bureau du voisin.
scenario_isolated_start_concurrent() (
	local cleanup_failed=0
	local session_a="e2e-conc-a-$$"
	local session_b="e2e-conc-b-$$"
	local a_title="hyprpilot-e2e-iso-conc-a-$$"
	local b_title="hyprpilot-e2e-iso-conc-b-$$"
	local signatures_before="" signatures_ready=0 sockets_before=""
	local entry_x="" entry_y="" dir_a="" dir_b="" scenario_tmp=""
	local a_log="" b_log="" a_output="" b_output=""
	local a_pid="" b_pid="" a_status=0 b_status=0
	local command_output="" shot_output="" addresses_json=""
	local a_signature="" b_signature="" a_display="" b_display=""
	local a_width a_height b_width b_height
	local host_workspace host_focus host_cursor_x host_cursor_y host_addresses host_monitors
	local host_workspaces
	# shellcheck disable=SC2034 # Sorties obligatoires de move_cursor_offcenter (printf -v); la reference reste le snapshot.
	local offcenter_x offcenter_y

	# shellcheck disable=SC2329 # Invoked indirectly by the EXIT trap.
	cleanup_isolated_start_concurrent() {
		local scenario_status=$?
		local pid
		trap - EXIT INT TERM

		for pid in "${a_pid}" "${b_pid}"; do
			if [[ -n ${pid} ]] && kill -0 "${pid}" 2>/dev/null; then
				kill "${pid}" 2>/dev/null || cleanup_failed=1
				wait "${pid}" 2>/dev/null || true
			fi
		done
		# Le premier appel termine toutes les instances apparues, le second ne
		# s'occupe que des ressources nommees de la seconde session.
		if ! isolated_raw_cleanup "${session_a}" "${signatures_ready}" "${signatures_before}" \
			"nettoyage isolated_start_concurrent A"; then
			cleanup_failed=1
		fi
		if ! isolated_raw_cleanup "${session_b}" 0 "" \
			"nettoyage isolated_start_concurrent B"; then
			cleanup_failed=1
		fi
		restore_cursor_best_effort "${entry_x}" "${entry_y}"
		if [[ -n ${scenario_tmp} ]]; then
			if [[ ${scenario_tmp} != "${XDG_RUNTIME_DIR}"/hyprpilot-e2e-iso-conc.* ]]; then
				fail "nettoyage isolated_start_concurrent: repertoire observe=${scenario_tmp}; attendu=sous ${XDG_RUNTIME_DIR}"
				cleanup_failed=1
			elif ! rm -rf -- "${scenario_tmp}"; then
				fail "nettoyage isolated_start_concurrent: repertoire observe=present (${scenario_tmp}); attendu=supprime"
				cleanup_failed=1
			fi
		fi

		if ((scenario_status != 0 || cleanup_failed != 0)); then
			exit 1
		fi
		exit 0
	}

	trap cleanup_isolated_start_concurrent EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	require_isolated_support
	dir_a=$(session_dir_path "${session_a}")
	dir_b=$(session_dir_path "${session_b}")
	read_cursor entry_x entry_y "precondition isolated_start_concurrent" || return 1
	snapshot_hypr_signatures signatures_before
	signatures_ready=1
	snapshot_wayland_sockets sockets_before
	if [[ -e ${dir_a} || -e ${dir_b} ]]; then
		fail "precondition isolated_start_concurrent: sessions observe=presentes (${dir_a}, ${dir_b}); attendu=absentes"
		return 1
	fi
	assert_named_output_absent "hyprpilot-${session_a}" \
		"precondition isolated_start_concurrent" || return 1
	assert_named_output_absent "hyprpilot-${session_b}" \
		"precondition isolated_start_concurrent" || return 1
	if ! scenario_tmp=$(mktemp -d -- "${XDG_RUNTIME_DIR}/hyprpilot-e2e-iso-conc.XXXXXX"); then
		fail "isolated_start_concurrent: repertoire temporaire observe=creation impossible sous ${XDG_RUNTIME_DIR}; attendu=mktemp -d reussi"
		return 1
	fi
	a_log=${scenario_tmp}/start-a.log
	b_log=${scenario_tmp}/start-b.log
	move_cursor_offcenter offcenter_x offcenter_y \
		"precondition curseur isolated_start_concurrent" || return 1
	read_host_snapshot host_workspace host_focus host_cursor_x host_cursor_y host_addresses \
		host_monitors host_workspaces "snapshot avant isolated_start_concurrent" || return 1

	# Tailles distinctes: c'est ce qui rend la capture discriminante plus loin.
	"${HYPRPILOT}" --session "${session_a}" session start --isolated \
		--app "zenity --entry --title=${a_title}" \
		--match-title "${a_title}" --size 800x600 >"${a_log}" 2>&1 &
	a_pid=$!
	"${HYPRPILOT}" --session "${session_b}" session start --isolated \
		--app "zenity --entry --title=${b_title}" \
		--match-title "${b_title}" --size 1024x768 >"${b_log}" 2>&1 &
	b_pid=$!
	if wait "${a_pid}"; then
		a_status=0
	else
		a_status=1
	fi
	if wait "${b_pid}"; then
		b_status=0
	else
		b_status=1
	fi
	a_pid=""
	b_pid=""
	a_output=$(<"${a_log}")
	b_output=$(<"${b_log}")
	if ((a_status != 0 || b_status != 0)); then
		fail "starts concurrents isolated_start_concurrent observe=A exit ${a_status} (${a_output}), B exit ${b_status} (${b_output}); attendu=deux succes"
		return 1
	fi

	read_state_field "${session_a}" '.instance.signature' a_signature \
		"etat A isolated_start_concurrent" || return 1
	read_state_field "${session_b}" '.instance.signature' b_signature \
		"etat B isolated_start_concurrent" || return 1
	read_state_field "${session_a}" '.instance.wayland_display' a_display \
		"etat A isolated_start_concurrent" || return 1
	read_state_field "${session_b}" '.instance.wayland_display' b_display \
		"etat B isolated_start_concurrent" || return 1
	if [[ ${a_signature} == "${b_signature}" || ${a_display} == "${b_display}" ]]; then
		fail "isolated_start_concurrent: etats observe=signature ${a_signature}/${b_signature}, display ${a_display}/${b_display}; attendu=deux instances distinctes"
		return 1
	fi
	# Le socket enregistre appartient a l'instance enregistree par la meme session.
	assert_state_instance_binding "${session_a}" "liaison A isolated_start_concurrent" || return 1
	assert_state_instance_binding "${session_b}" "liaison B isolated_start_concurrent" || return 1

	# Chaque instance ne voit que son app: la decouverte n'a pas croise les deux.
	wait_nested_addresses_by_title "${a_signature}" addresses_json "${a_title}" 1 \
		"fenêtre A isolated_start_concurrent" || return 1
	wait_nested_addresses_by_title "${b_signature}" addresses_json "${b_title}" 1 \
		"fenêtre B isolated_start_concurrent" || return 1
	wait_nested_addresses_by_title "${a_signature}" addresses_json "${b_title}" 0 \
		"etancheite A isolated_start_concurrent" || return 1
	wait_nested_addresses_by_title "${b_signature}" addresses_json "${a_title}" 0 \
		"etancheite B isolated_start_concurrent" || return 1

	# Verification fonctionnelle du display enregistre: `shot --full` passe par lui
	# et rend la taille du bureau agent, distincte d'une session a l'autre.
	read_nested_output_size "${a_signature}" a_width a_height \
		"output nested A isolated_start_concurrent" || return 1
	read_nested_output_size "${b_signature}" b_width b_height \
		"output nested B isolated_start_concurrent" || return 1
	if ((a_width == b_width && a_height == b_height)); then
		fail "isolated_start_concurrent: bureaux agents observe=${a_width}x${a_height} pour A et B; attendu=deux tailles distinctes (800x600 et 1024x768) pour discriminer les captures"
		return 1
	fi
	if ! shot_output=$(
		"${HYPRPILOT}" --session "${session_a}" shot conc-a --full --out "${scenario_tmp}" 2>&1
	); then
		fail "shot A isolated_start_concurrent observe=echec (${shot_output}); attendu=capture du bureau agent A"
		return 1
	fi
	assert_png_dimensions "${scenario_tmp}/conc-a.png" "${a_width}" "${a_height}" \
		"shot A isolated_start_concurrent" || return 1
	if ! shot_output=$(
		"${HYPRPILOT}" --session "${session_b}" shot conc-b --full --out "${scenario_tmp}" 2>&1
	); then
		fail "shot B isolated_start_concurrent observe=echec (${shot_output}); attendu=capture du bureau agent B"
		return 1
	fi
	assert_png_dimensions "${scenario_tmp}/conc-b.png" "${b_width}" "${b_height}" \
		"shot B isolated_start_concurrent" || return 1

	# Teardowns independants.
	if ! command_output=$("${HYPRPILOT}" --session "${session_a}" teardown 2>&1); then
		fail "teardown A isolated_start_concurrent observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	wait_nested_instance_gone "${a_signature}" "teardown A isolated_start_concurrent" || return 1
	wait_named_output_absent "hyprpilot-${session_a}" \
		"teardown A isolated_start_concurrent" || return 1
	wait_workspace_absent "agent-${session_a}" "teardown A isolated_start_concurrent" || return 1
	assert_no_marked_processes "${session_a}" "teardown A isolated_start_concurrent" || return 1
	if [[ -e ${dir_a} || -e ${XDG_RUNTIME_DIR}/hypr/${a_signature} ||
		-e ${XDG_RUNTIME_DIR}/${a_display} ]]; then
		fail "teardown A isolated_start_concurrent: residus observe=${dir_a}, ${XDG_RUNTIME_DIR}/hypr/${a_signature} ou ${XDG_RUNTIME_DIR}/${a_display}; attendu=tout supprime"
		return 1
	fi
	if ! nested_instance_alive "${b_signature}"; then
		fail "teardown A isolated_start_concurrent: instance B observe=injoignable (${b_signature}); attendu=intacte"
		return 1
	fi
	if ! named_output_present "hyprpilot-${session_b}"; then
		fail "teardown A isolated_start_concurrent: output B observe=absent; attendu=hyprpilot-${session_b} intact"
		return 1
	fi
	assert_state_instance_binding "${session_b}" \
		"liaison B apres teardown A isolated_start_concurrent" || return 1

	if ! command_output=$("${HYPRPILOT}" --session "${session_b}" teardown 2>&1); then
		fail "teardown B isolated_start_concurrent observe=echec (${command_output}); attendu=succes"
		return 1
	fi
	wait_nested_instance_gone "${b_signature}" "teardown B isolated_start_concurrent" || return 1
	wait_workspace_absent "agent-${session_b}" "teardown B isolated_start_concurrent" || return 1
	if [[ -e ${dir_b} || -e ${XDG_RUNTIME_DIR}/hypr/${b_signature} ||
		-e ${XDG_RUNTIME_DIR}/${b_display} ]]; then
		fail "teardown B isolated_start_concurrent: residus observe=${dir_b}, ${XDG_RUNTIME_DIR}/hypr/${b_signature} ou ${XDG_RUNTIME_DIR}/${b_display}; attendu=tout supprime"
		return 1
	fi
	assert_isolated_no_residue "${session_b}" "${signatures_before}" "${sockets_before}" \
		"teardown B isolated_start_concurrent" || return 1
	assert_host_snapshot_equals "${host_workspace}" "${host_focus}" "${host_cursor_x}" \
		"${host_cursor_y}" "${host_addresses}" "${host_monitors}" "${host_workspaces}" "[]" \
		"hote apres isolated_start_concurrent" || return 1
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
	for binary in hyprctl grim jq zenity cmp; do
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
