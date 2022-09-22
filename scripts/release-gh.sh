#!/usr/bin/env bash
set -euo pipefail

main() {
  cd "$(dirname "${BASH_SOURCE[0]}")" && cd .. # aka $WORKSPACE_ROOT

  # allow passing in date so we can test this
  local -r DATE=${1:-$(date +%Y-%m-%d)}
  # extract a dated release between equals dividers (assuming consistent whitespace)
  grep "${DATE}" -A 250 CHANGELOG.md \
    | tail -n +3 \
    | sed '1,/===================/!d' \
    | head -n -3 > release.txt

  # Add links to critical bugs
  local -r CRITISSUES="$(curl -sSL -H "Accept: application/vnd.github.v3+json" https://api.github.com/repos/kube-rs/kube/issues?labels=critical)"
  if (( $(echo "${CRITISSUES}" | jq length) > 0 )); then
    echo -e "\n### Known Issues" >> release.txt
    echo "${CRITISSUES}" | jq '.[] | "- \(.title) - #[\(.number)](\(.url))"' -r >> release.txt
  fi
}

# shellcheck disable=SC2068
main $@
