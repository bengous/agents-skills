#!/usr/bin/env bash

usage() {
	printf 'Usage: %s [--jobs COUNT] -- COMMAND [ARG...]\n' "${0##*/}" >&2
}

die_usage() {
	printf 'error: %s\n' "$1" >&2
	usage
	exit 64
}

job_count=1

while (($# > 0)); do
	case $1 in
	--jobs)
		(($# >= 2)) || die_usage '--jobs requires a value'
		job_count=$2
		shift 2
		;;
	--help)
		usage
		exit 0
		;;
	--)
		shift
		break
		;;
	*)
		die_usage "unknown option: $1"
		;;
	esac
done

[[ $job_count =~ ^[1-9][0-9]*$ ]] || die_usage '--jobs must be a positive integer'
(($# > 0)) || die_usage 'a worker command is required'

pids=()
signal_name=
signal_status=0

# Invoked by the signal traps below.
# shellcheck disable=SC2329
forward_signal() {
	local requested_signal=$1
	local requested_status=$2
	local pid

	if ((signal_status == 0)); then
		signal_name=$requested_signal
		signal_status=$requested_status
	fi

	for pid in "${pids[@]}"; do
		kill -s "$signal_name" "$pid" 2>/dev/null || :
	done
}

trap 'forward_signal HUP 129' HUP
trap 'forward_signal INT 130' INT
trap 'forward_signal TERM 143' TERM

for ((index = 1; index <= job_count; index++)); do
	((signal_status == 0)) || break
	JOB_INDEX=$index "$@" &
	pid=$!
	pids+=("$pid")
	if ((signal_status != 0)); then
		kill -s "$signal_name" "$pid" 2>/dev/null || :
	fi
done

first_failure=0

for pid in "${pids[@]}"; do
	while :; do
		if wait "$pid"; then
			status=0
		else
			status=$?
		fi

		if ((signal_status != 0)) && kill -0 "$pid" 2>/dev/null; then
			continue
		fi
		break
	done

	if ((first_failure == 0 && status != 0)); then
		first_failure=$status
	fi
done

trap - HUP INT TERM

if ((signal_status != 0)); then
	exit "$signal_status"
fi

exit "$first_failure"
