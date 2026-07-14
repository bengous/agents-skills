#!/usr/bin/env bash

usage() {
	printf 'Usage: %s --needle VALUE -- FILE...\n' "${0##*/}" >&2
}

die_usage() {
	printf 'error: %s\n' "$1" >&2
	usage
	exit 64
}

needle=
needle_set=0

while (($# > 0)); do
	case $1 in
	--needle)
		(($# >= 2)) || die_usage '--needle requires a value'
		needle=$2
		needle_set=1
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
	-*)
		die_usage "unknown option: $1"
		;;
	*)
		break
		;;
	esac
done

((needle_set == 1)) || die_usage '--needle is required'
(($# > 0)) || die_usage 'at least one file is required'

for file in "$@"; do
	if [[ ! -f $file || ! -r $file ]]; then
		printf 'error: not a readable regular file: %s\n' "$file" >&2
		exit 66
	fi
done

grep_args=(-F -e "$needle")
matched=0

for file in "$@"; do
	if grep "${grep_args[@]}" <"$file"; then
		matched=1
	else
		status=$?
		if ((status > 1)); then
			printf 'error: grep failed for %s with status %d\n' "$file" "$status" >&2
			exit "$status"
		fi
	fi
done

((matched == 1))
