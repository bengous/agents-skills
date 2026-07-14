#!/bin/sh

usage() {
	printf 'Usage: %s TARGET -- PRODUCER [ARG...]\n' "${0##*/}" >&2
}

die_usage() {
	printf 'error: %s\n' "$1" >&2
	usage
	exit 64
}

[ "$#" -ge 3 ] || die_usage 'target and producer command are required'

target=$1
shift
[ "$1" = -- ] || die_usage 'expected -- before producer command'
shift
[ "$#" -gt 0 ] || die_usage 'producer command is required'
[ -n "$target" ] || die_usage 'target must not be empty'

case $target in
*/)
	die_usage 'target must name a file'
	;;
/* | ./* | ../*) ;;
*) target=./$target ;;
esac

parent=${target%/*}
[ -n "$parent" ] || parent=/
base=${target##*/}

[ -d "$parent" ] || {
	printf 'error: target directory does not exist: %s\n' "$parent" >&2
	exit 72
}
[ ! -d "$target" ] || {
	printf 'error: target is a directory: %s\n' "$target" >&2
	exit 73
}
command -v mktemp >/dev/null 2>&1 || {
	printf 'error: required command not found: mktemp\n' >&2
	exit 69
}

umask 077
tmp=

cleanup() {
	status=$?
	trap - 0 HUP INT TERM
	if [ -n "$tmp" ] && ! rm -f "$tmp"; then
		printf 'error: failed to remove temporary file: %s\n' "$tmp" >&2
		[ "$status" -ne 0 ] || status=1
	fi
	exit "$status"
}

trap cleanup 0
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

template=$parent/.$base.tmp.XXXXXX
if tmp=$(mktemp "$template"); then
	:
else
	printf 'error: failed to create temporary file in %s\n' "$parent" >&2
	exit 73
fi

if "$@" >"$tmp"; then
	:
else
	status=$?
	printf 'error: producer failed with status %d\n' "$status" >&2
	exit "$status"
fi

if mv -f "$tmp" "$target"; then
	tmp=
else
	status=$?
	printf 'error: failed to replace target: %s\n' "$target" >&2
	exit "$status"
fi
