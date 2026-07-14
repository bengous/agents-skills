#!/bin/sh

usage() {
	printf 'Usage: %s --needle VALUE -- FILE...\n' "${0##*/}" >&2
}

die_usage() {
	printf 'error: %s\n' "$1" >&2
	usage
	exit 64
}

needle=
needle_set=false

while [ "$#" -gt 0 ]; do
	case $1 in
	--needle)
		[ "$#" -ge 2 ] || die_usage '--needle requires a value'
		needle=$2
		needle_set=true
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

[ "$needle_set" = true ] || die_usage '--needle is required'
[ "$#" -gt 0 ] || die_usage 'at least one file is required'

for file; do
	if [ ! -f "$file" ] || [ ! -r "$file" ]; then
		printf 'error: not a readable regular file: %s\n' "$file" >&2
		exit 66
	fi
done

matched=false

for file; do
	if grep -F -e "$needle" <"$file"; then
		matched=true
	else
		status=$?
		if [ "$status" -gt 1 ]; then
			printf 'error: grep failed for %s with status %d\n' "$file" "$status" >&2
			exit "$status"
		fi
	fi
done

[ "$matched" = true ]
