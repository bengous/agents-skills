#!/usr/bin/env bash
set -euo pipefail

usage() {
	cat <<'EOF'
Usage: update-skills.sh --scope global|project|all [options]

Options:
  --scope VALUE        Update scope: global, project, or all.
  --project-dir DIR   Project directory for project-scope updates. Defaults to $PWD.
  --dotfiles-dir DIR  Dotfiles repo for status reporting. Defaults to ~/dotfiles.
  --force-project     Run project update even when no project skill marker is found.
  --managed-bootstrap Run the dotfiles-managed global skills bootstrap before global update.
  --help              Show this help.

Environment:
  SKILLS_MAINTENANCE_DRY_RUN=1  Print commands without running updates.
EOF
}

scope="global"
project_dir="${PWD}"
dotfiles_dir="${HOME}/dotfiles"
force_project=0
managed_bootstrap=0
dry_run="${SKILLS_MAINTENANCE_DRY_RUN:-0}"
live_lock="${HOME}/.agents/.skill-lock.json"

while [[ $# -gt 0 ]]; do
	case "$1" in
		--scope)
			[[ $# -ge 2 ]] || {
				printf 'missing value for --scope\n' >&2
				exit 2
			}
			scope="$2"
			shift 2
			;;
		--project-dir)
			[[ $# -ge 2 ]] || {
				printf 'missing value for --project-dir\n' >&2
				exit 2
			}
			project_dir="$2"
			shift 2
			;;
		--dotfiles-dir)
			[[ $# -ge 2 ]] || {
				printf 'missing value for --dotfiles-dir\n' >&2
				exit 2
			}
			dotfiles_dir="$2"
			shift 2
			;;
		--force-project)
			force_project=1
			shift
			;;
		--managed-bootstrap)
			managed_bootstrap=1
			shift
			;;
		--help|-h)
			usage
			exit 0
			;;
		*)
			printf 'unknown argument: %s\n' "$1" >&2
			usage >&2
			exit 2
			;;
	esac
done

case "${scope}" in
	global|project|all) ;;
	*)
		printf 'invalid --scope: %s\n' "${scope}" >&2
		exit 2
		;;
esac

if ! command -v bunx >/dev/null 2>&1; then
	printf 'bunx is required for skills.sh updates\n' >&2
	exit 127
fi

if [[ ! -d "${project_dir}" ]]; then
	printf 'project directory does not exist: %s\n' "${project_dir}" >&2
	exit 2
fi

export DISABLE_TELEMETRY="${DISABLE_TELEMETRY:-1}"

section() {
	printf '\n== %s ==\n' "$*"
}

quote_cmd() {
	printf '%q ' "$@"
	printf '\n'
}

run_cmd() {
	printf '$ '
	quote_cmd "$@"
	if [[ "${dry_run}" == "1" ]]; then
		return 0
	fi
	"$@"
}

git_status() {
	local dir="$1" label="$2"
	if git -C "${dir}" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
		section "${label} git status"
		git -C "${dir}" status --short --branch --untracked-files=all
	fi
}

project_has_skill_markers() {
	[[ -f "${project_dir}/skills-lock.json" ]] ||
		[[ -f "${project_dir}/skills.lock" ]] ||
		[[ -d "${project_dir}/.agents/skills" ]] ||
		[[ -d "${project_dir}/.skills" ]]
}

print_lock_candidates() {
	local dir="$1"
	find "${dir}" -maxdepth 4 \( -name 'skills-lock.json' -o -name 'skills.lock' \) -print 2>/dev/null | sort
}

print_live_lock() {
	section "live global skill lock"
	if [[ ! -e "${live_lock}" ]]; then
		printf 'missing: %s\n' "${live_lock}"
		return 0
	fi

	printf 'path: %s\n' "${live_lock}"
	if command -v sha256sum >/dev/null 2>&1; then
		sha256sum "${live_lock}"
	fi
	if command -v stat >/dev/null 2>&1; then
		stat -c 'mtime: %y' "${live_lock}" 2>/dev/null || true
	fi
}

run_managed_bootstrap() {
	local bootstrap="${dotfiles_dir}/.chezmoiscripts/run_after_install-global-skills.sh"
	if [[ "${managed_bootstrap}" != "1" ]]; then
		return 0
	fi
	if [[ ! -x "${bootstrap}" ]]; then
		section "managed skills bootstrap"
		printf 'skipped: missing executable bootstrap at %s\n' "${bootstrap}"
		return 0
	fi

	section "managed skills bootstrap"
	run_cmd "${bootstrap}"
}

update_global() {
	run_managed_bootstrap
	section "global skills update"
	run_cmd bunx skills update -g -y
}

update_project() {
	if [[ "${force_project}" != "1" ]] && ! project_has_skill_markers; then
		section "project skills update"
		printf 'skipped: no project skill markers found under %s\n' "${project_dir}"
		return 0
	fi

	section "project skills update"
	(
		cd "${project_dir}"
		run_cmd bunx skills update -p -y
	)
}

section "skills maintenance"
printf 'scope: %s\n' "${scope}"
printf 'project_dir: %s\n' "${project_dir}"
printf 'dotfiles_dir: %s\n' "${dotfiles_dir}"
printf 'dry_run: %s\n' "${dry_run}"
printf 'managed_bootstrap: %s\n' "${managed_bootstrap}"

git_status "${project_dir}" "project before"
if [[ -d "${dotfiles_dir}" ]]; then
	git_status "${dotfiles_dir}" "dotfiles before"
fi
print_live_lock

case "${scope}" in
	global)
		update_global
		;;
	project)
		update_project
		;;
	all)
		update_global
		update_project
		;;
esac

section "skill lock candidates"
printf '%s\n' "${live_lock}"
print_lock_candidates "${project_dir}"
if [[ -d "${dotfiles_dir}" ]]; then
	print_lock_candidates "${dotfiles_dir}"
fi

print_live_lock
git_status "${project_dir}" "project after"
if [[ -d "${dotfiles_dir}" ]]; then
	git_status "${dotfiles_dir}" "dotfiles after"
fi
