#!/usr/bin/env bash
# list <file>...              JSON of every fetchFromGitHub pin found
# bump <file> <owner> <repo>  rewrite that pin to its upstream default branch HEAD
set -euo pipefail

extract_pins() {
  awk '
    function attr(line, name,   m) {
      if (!match(line, name " *= *\"[^\"]+\"")) return ""
      m = substr(line, RSTART, RLENGTH)
      sub(/^[^"]*"/, "", m)
      sub(/"$/, "", m)
      return m
    }
    function count(line, ch,   tmp) { tmp = line; return gsub(ch, "", tmp) }

    /fetchFromGitHub[[:space:]]*{/ { inside = 1; depth = 0; owner = repo = rev = hash = "" }

    inside {
      if ((v = attr($0, "owner")) != "") owner = v
      if ((v = attr($0, "repo")) != "") repo = v
      if ((v = attr($0, "rev")) != "") rev = v
      if ((v = attr($0, "sha256")) != "") hash = v
      if ((v = attr($0, "hash")) != "") hash = v

      depth += count($0, "{") - count($0, "}")
      if (depth <= 0) {
        inside = 0
        if (owner && repo && rev && hash) print FILENAME "\t" owner "\t" repo "\t" rev "\t" hash
      }
    }
  ' "$@"
}

emit() { [[ -n "${GITHUB_OUTPUT:-}" ]] && echo "$1" >>"$GITHUB_OUTPUT"; }

case "${1:?usage: list|bump}" in
list)
  shift
  extract_pins "$@" | jq -R -s -c '
    split("\n") | map(select(length > 0) | split("\t") | {
      file: .[0], owner: .[1], repo: .[2], rev: .[3]
    })
  '
  ;;

bump)
  file=${2:?file} owner=${3:?owner} repo=${4:?repo}

  pin=$(extract_pins "$file" | awk -F'\t' -v o="$owner" -v r="$repo" '$2 == o && $3 == r')
  [[ -n "$pin" ]] || {
    echo "no $owner/$repo pin in $file" >&2
    exit 1
  }

  IFS=$'\t' read -r _ _ _ old_rev old_hash <<<"$pin"
  rev=$(gh api "repos/$owner/$repo/commits/HEAD" --jq '.sha')

  if [[ "$rev" == "$old_rev" ]]; then
    echo "$owner/$repo: up to date (${old_rev:0:7})"
    emit "changed=false"
    exit 0
  fi

  hash=$(nix-prefetch-github --rev "$rev" "$owner" "$repo" | jq -r '.hash')
  sed -i "s|$old_rev|$rev|g; s|$old_hash|$hash|g" "$file"

  echo "$owner/$repo: ${old_rev:0:7} -> ${rev:0:7}"
  emit "changed=true"
  emit "rev=$rev"
  emit "short=${rev:0:7}"
  emit "compare=https://github.com/$owner/$repo/compare/$old_rev...$rev"
  ;;

*)
  echo "usage: $0 list|bump" >&2
  exit 1
  ;;
esac
