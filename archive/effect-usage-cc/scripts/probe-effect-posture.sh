#!/usr/bin/env bash
# Probes the current project for Effect adoption signals.
# Called via !`command` preprocessing when the skill loads.
# Outputs XML to <project-effect-posture>.
set -euo pipefail

require() {
	command -v "$1" > /dev/null 2>&1 || {
		printf '<project-effect-posture error="missing dependency: %s" />\n' "$1"
		exit 0
	}
}

require jq
require rg

# Escape XML-special chars to prevent package.json values from breaking
# the XML structure (defense-in-depth against prompt injection).
xml_escape() {
	local s="${1//&/\&amp;}"
	s="${s//</\&lt;}"
	s="${s//>/\&gt;}"
	printf '%s' "${s}"
}

if [[ -f package.json ]]; then
	raw_v=$(jq -r '.dependencies.effect // .devDependencies.effect // "none"' package.json) || true
	raw_p=$(jq -r '(.dependencies // {}) + (.devDependencies // {}) | keys[] | select(startswith("@effect/"))' package.json | paste -sd,) || true
	v=$(xml_escape "${raw_v}")
	p=$(xml_escape "${raw_p}")
else
	v="no package.json"
	p=""
fi
p=${p:-none}

rg_opts=(--type ts -l --no-messages --max-depth 20)

# Explicit "." is required: without a path arg, rg guesses whether to search
# cwd or read stdin. When stdin is a pipe (as in !`command` preprocessing),
# rg reads stdin and blocks forever. See BurntSushi/ripgrep#2582.
n=$(rg "${rg_opts[@]}" "from ['\"]effect" . 2> /dev/null | wc -l | tr -d ' ') || true
s=$(rg "${rg_opts[@]}" 'Context\.Tag|Effect\.Service' . 2> /dev/null | wc -l | tr -d ' ') || true

cat << XML
<project-effect-posture>
  <version>${v}</version>
  <files-importing-effect>${n}</files-importing-effect>
  <packages>${p}</packages>
  <service-definitions>${s}</service-definitions>
</project-effect-posture>
XML
