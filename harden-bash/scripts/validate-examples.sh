#!/bin/sh

fail() {
	printf 'FAIL: %s\n' "$1" >&2
	exit 1
}

expect_status() {
	expected_status=$1
	shift
	if "$@" >/dev/null 2>&1; then
		actual_status=0
	else
		actual_status=$?
	fi
	[ "$actual_status" -eq "$expected_status" ] || fail "expected status $expected_status, got $actual_status: $*"
}

expect_output() {
	expected_output=$1
	shift
	if actual_output=$("$@"); then
		:
	else
		actual_status=$?
		fail "expected success, got status $actual_status: $*"
	fi
	[ "$actual_output" = "$expected_output" ] || fail "unexpected output from: $*"
}

wait_for_path() {
	wait_path=$1
	wait_attempt=0
	while [ ! -e "$wait_path" ]; do
		wait_attempt=$((wait_attempt + 1))
		[ "$wait_attempt" -lt 100 ] || fail "timed out waiting for $wait_path"
		sleep 0.05
	done
}

test_safe_cli() (
	test_shell=$1
	test_script=$2
	case_dir=$test_root/safe-cli
	mkdir "$case_dir" || exit 1
	cd "$case_dir" || exit 1

	printf '%s\n' 'space value' '--literal' "\$(touch injected)" >'-input file'
	expect_output 'space value' "$test_shell" "$test_script" --needle 'space value' -- '-input file'
	expect_output '--literal' "$test_shell" "$test_script" --needle '--literal' -- '-input file'
	expect_output "\$(touch injected)" "$test_shell" "$test_script" --needle "\$(touch injected)" -- '-input file'
	[ ! -e injected ] || fail 'safe CLI executed input as code'
	expect_status 64 "$test_shell" "$test_script" --needle
	expect_status 64 "$test_shell" "$test_script" --needle value --
	expect_status 66 "$test_shell" "$test_script" --needle value -- missing
)

test_atomic_replace() (
	test_shell=$1
	test_script=$2
	case_dir=$test_root/atomic-replace
	mkdir "$case_dir" || exit 1
	cd "$case_dir" || exit 1

	printf 'old value\n' >'-target file'
	expect_status 23 "$test_shell" "$test_script" '-target file' -- sh -c 'printf "partial\n"; exit 23'
	expect_output 'old value' sh -c "cat < \"\$1\"" sh '-target file'

	set -- ./.-target\ file.tmp.*
	[ "$#" -eq 1 ] && [ "$1" = './.-target file.tmp.*' ] || fail 'producer failure left a temporary file'

	expect_status 0 "$test_shell" "$test_script" '-target file' -- sh -c 'printf "new value\n"'
	expect_output 'new value' sh -c "cat < \"\$1\"" sh '-target file'
	set -- ./.-target\ file.tmp.*
	[ "$#" -eq 1 ] && [ "$1" = './.-target file.tmp.*' ] || fail 'successful replacement left a temporary file'
)

test_supervisor() (
	test_script=$1
	case_dir=$test_root/supervisor
	mkdir "$case_dir" || exit 1

	expect_status 7 bash "$test_script" --jobs 3 -- bash -c "if [[ \$JOB_INDEX == 2 ]]; then exit 7; fi"

	MARKER_PREFIX=$case_dir/done
	export MARKER_PREFIX
	expect_status 0 bash "$test_script" --jobs 2 -- bash -c ": > \"\$MARKER_PREFIX.\$JOB_INDEX\""
	[ -f "$MARKER_PREFIX.1" ] && [ -f "$MARKER_PREFIX.2" ] || fail 'normal run did not reap all workers'

	READY_PREFIX=$case_dir/ready
	TERM_PREFIX=$case_dir/term
	PID_PREFIX=$case_dir/pid
	export READY_PREFIX TERM_PREFIX PID_PREFIX
	# The single-quoted argument is a program for the child Bash, not this shell.
	# shellcheck disable=SC2016
	bash "$test_script" --jobs 2 -- bash -c 'on_term() { : > "$TERM_PREFIX.$JOB_INDEX"; exit 0; }; printf "%s\n" "$BASHPID" > "$PID_PREFIX.$JOB_INDEX"; : > "$READY_PREFIX.$JOB_INDEX"; trap on_term TERM; while :; do sleep 1; done' &
	supervisor_pid=$!
	wait_for_path "$READY_PREFIX.1"
	wait_for_path "$READY_PREFIX.2"
	kill -TERM "$supervisor_pid"
	if wait "$supervisor_pid"; then
		supervisor_status=0
	else
		supervisor_status=$?
	fi
	[ "$supervisor_status" -eq 143 ] || fail "supervisor returned $supervisor_status after SIGTERM"
	[ -f "$TERM_PREFIX.1" ] && [ -f "$TERM_PREFIX.2" ] || fail 'SIGTERM was not forwarded to every worker'

	for index in 1 2; do
		worker_pid=$(cat "$PID_PREFIX.$index") || exit 1
		if kill -0 "$worker_pid" 2>/dev/null; then
			fail "worker still running after supervisor exit: $worker_pid"
		fi
	done
)

run_runtime_tests() {
	runtime=$1
	test_root=$(mktemp -d /tmp/harden-bash.XXXXXX) || fail 'cannot create test directory'

	# Invoked by the exit trap below.
	# shellcheck disable=SC2329
	cleanup_tests() {
		cleanup_status=$?
		trap - 0 HUP INT TERM
		case $test_root in
		/tmp/harden-bash.*) rm -rf "$test_root" || cleanup_status=1 ;;
		*)
			printf 'FAIL: refusing to remove unexpected test path: %s\n' "$test_root" >&2
			cleanup_status=1
			;;
		esac
		exit "$cleanup_status"
	}

	trap cleanup_tests 0
	trap 'exit 129' HUP
	trap 'exit 130' INT
	trap 'exit 143' TERM

	case $runtime in
	bash)
		test_safe_cli bash /skill/examples/bash/safe-cli.sh || exit 1
		test_supervisor /skill/examples/bash/supervise-jobs.sh || exit 1
		;;
	posix)
		test_safe_cli sh /skill/examples/posix/safe-cli.sh || exit 1
		test_atomic_replace sh /skill/examples/posix/atomic-replace.sh || exit 1
		;;
	*) fail "unknown runtime: $runtime" ;;
	esac

	printf 'runtime %s: ok\n' "$runtime"
}

case ${1-} in
--runtime)
	[ "$#" -eq 2 ] || fail 'usage: validate-examples.sh --runtime bash|posix'
	run_runtime_tests "$2"
	exit 0
	;;
'') ;;
*) fail 'usage: validate-examples.sh [--runtime bash|posix]' ;;
esac

case $0 in
*/*) script_dir=${0%/*} ;;
*) script_dir=. ;;
esac
skill_dir=$(CDPATH='' cd "$script_dir/.." && pwd -P) || fail 'cannot locate skill directory'

for required_command in bash shellcheck shfmt docker; do
	command -v "$required_command" >/dev/null 2>&1 || fail "required command not found: $required_command"
done

for bash_file in "$skill_dir"/examples/bash/*.sh; do
	bash -n "$bash_file"
	shellcheck -s bash "$bash_file"
	shfmt -d -ln bash "$bash_file"
done

for posix_file in "$skill_dir"/examples/posix/*.sh "$skill_dir"/scripts/validate-examples.sh; do
	shellcheck -s sh "$posix_file"
	shfmt -d -ln posix "$posix_file"
done

container_user=$(id -u):$(id -g)

docker run --rm --network none --read-only \
	--user "$container_user" \
	--tmpfs /tmp:rw,nosuid,nodev,noexec,mode=1777,size=16m \
	--volume "$skill_dir:/skill:ro" \
	--workdir /tmp \
	bash:5.3 sh /skill/scripts/validate-examples.sh --runtime bash

docker run --rm --network none --read-only \
	--user "$container_user" \
	--tmpfs /tmp:rw,nosuid,nodev,noexec,mode=1777,size=16m \
	--volume "$skill_dir:/skill:ro" \
	--workdir /tmp \
	debian:bookworm-slim sh /skill/scripts/validate-examples.sh --runtime posix

docker run --rm --network none --read-only \
	--user "$container_user" \
	--tmpfs /tmp:rw,nosuid,nodev,noexec,mode=1777,size=16m \
	--volume "$skill_dir:/skill:ro" \
	--workdir /tmp \
	busybox:1.37 sh /skill/scripts/validate-examples.sh --runtime posix

printf 'all examples: ok\n'
