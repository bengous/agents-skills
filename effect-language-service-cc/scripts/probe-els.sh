#!/usr/bin/env bash
# Probes the current project for @effect/language-service state.
# Called via !`command` preprocessing when the skill loads.
# Outputs XML to <els-context>.
set -euo pipefail

require() {
	command -v "$1" > /dev/null 2>&1 || {
		printf '<els-context error="missing dependency: %s" />\n' "$1"
		exit 0
	}
}

require jq

xml_escape() {
	local s="${1//&/\&amp;}"
	s="${s//</\&lt;}"
	s="${s//>/\&gt;}"
	printf '%s' "${s}"
}

if [[ ! -f package.json ]]; then
	printf '<els-context installed="false" reason="no package.json" />\n'
	exit 0
fi

raw_v=$(jq -r '.devDependencies["@effect/language-service"] // .dependencies["@effect/language-service"] // "none"' package.json) || true
v=$(xml_escape "${raw_v}")

if [[ "$v" == "none" ]]; then
	printf '<els-context installed="false" />\n'
	exit 0
fi

# Check TS patch status
patch_status="unknown"
els_bin="${ELS_BIN:-./node_modules/.bin/effect-language-service}"
if [[ -x "$els_bin" ]]; then
	check_out=$($els_bin check 2>&1) || true
	if echo "$check_out" | grep -q "patched with version"; then
		patch_status="patched"
	elif echo "$check_out" | grep -q "not patched"; then
		patch_status="not-patched"
	fi
fi

# Check for npm scripts
has_diagnose="false"
has_quickfixes="false"
has_codegen="false"
scripts=$(jq -r '.scripts // {} | to_entries[] | "\(.key)=\(.value)"' package.json 2>/dev/null) || true
echo "$scripts" | grep -q "effect-language-service diagnostics" && has_diagnose="true"
echo "$scripts" | grep -q "effect-language-service quickfixes" && has_quickfixes="true"
echo "$scripts" | grep -q "effect-language-service codegen" && has_codegen="true"

# Count configured diagnostic severities in tsconfig
# tsconfig is JSONC (comments). Try jq directly (works with jq 1.7+ on clean JSON),
# fall back to 0 on parse error. Avoids sed stripping which corrupts URLs.
rule_count=0
if [[ -f tsconfig.json ]]; then
	rule_count=$(jq '[.compilerOptions.plugins[]? | select(.name == "@effect/language-service") | .diagnosticSeverity // {} | keys[]] | length' tsconfig.json 2>/dev/null) || rule_count=0
fi

cat << XML
<els-context>
  <installed>true</installed>
  <version>${v}</version>
  <patch-status>${patch_status}</patch-status>
  <scripts diagnose="${has_diagnose}" quickfixes="${has_quickfixes}" codegen="${has_codegen}" />
  <configured-rules>${rule_count}</configured-rules>
</els-context>
XML
