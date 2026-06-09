#!/usr/bin/env zsh

unzip_copy_loop() {
  emulate -L zsh

  local zip="$1"

  if [[ -z "$zip" ]]; then
    echo "Usage: unzip_copy_loop <file.zip>" >&2
    return 1
  fi

  if [[ ! -f "$zip" ]]; then
    echo "File not found: $zip" >&2
    return 1
  fi

  if [[ "${zip:e:l}" != "zip" ]]; then
    echo "Not a zip file: $zip" >&2
    return 1
  fi

  for dep in unzip fzf wl-copy file mktemp chafa; do
    if ! command -v "$dep" >/dev/null 2>&1; then
      echo "Missing dependency: $dep" >&2
      return 1
    fi
  done

  local base="${zip:t:r}"
  local tmp_dir=""
  local out_dir=""
  local preview_script=""
  local selected=""
  local mime=""

  {
    tmp_dir="$(mktemp -d -t "unzip_copy_loop_${base}_XXXXXXXX")" || return 1
    out_dir="${tmp_dir}/${base}_zip_files"

    mkdir -p "$out_dir" || return 1

    if ! unzip -q "$zip" -d "$out_dir"; then
      echo "Failed to extract: $zip" >&2
      return 1
    fi

    preview_script="${tmp_dir}/preview_file"

    cat > "$preview_script" <<'EOF'
#!/usr/bin/env sh

rel_path="$1"
base_dir="$2"
path="$base_dir/$rel_path"

if [ ! -f "$path" ]; then
  echo "File not found:"
  echo "$path"
  exit 0
fi

mime="$(file --mime-type --brief -- "$path" 2>/dev/null)"

case "$mime" in
  image/*)
    cols="${FZF_PREVIEW_COLUMNS:-80}"
    rows="${FZF_PREVIEW_LINES:-30}"

    chafa \
      --format=symbols \
      --symbols=block+border+space \
      --colors=full \
      --dither=none \
      --size="${cols}x${rows}" \
      --animate=false \
      -- "$path" 2>/dev/null

    echo
    echo "$rel_path"
    echo "$mime"
    ;;
  text/*|application/json|application/xml|application/javascript|application/x-shellscript)
    echo "$rel_path"
    echo "$mime"
    echo
    sed -n '1,120p' "$path" 2>/dev/null
    ;;
  *)
    echo "$rel_path"
    echo "$mime"
    echo
    file --brief -- "$path" 2>/dev/null
    echo
    ls -lh -- "$path" 2>/dev/null
    ;;
esac
EOF

    chmod +x "$preview_script"

    echo "Extracted temporarily to: $out_dir"
    echo "It will be removed when you exit."

    while true; do
      selected="$(
        cd "$out_dir" || exit 1

        find . -type f -print |
          sed 's#^\./##' |
          sort |
          fzf \
            --ansi \
            --prompt="copy > " \
            --height=80% \
            --border \
            --layout=reverse \
            --preview-window='right:60%:wrap' \
            --preview="$preview_script {} $out_dir" \
            --bind='q:abort,esc:abort,ctrl-c:abort,ctrl-d:abort'
      )"

      if [[ $? -ne 0 || -z "$selected" ]]; then
        break
      fi

      mime="$(file --mime-type --brief -- "$out_dir/$selected" 2>/dev/null)"

      if [[ -n "$mime" ]]; then
        wl-copy --type "$mime" < "$out_dir/$selected"
      else
        wl-copy < "$out_dir/$selected"
      fi

      echo "Copied: $selected"
    done
  } always {
    if [[ -n "$tmp_dir" && -d "$tmp_dir" ]]; then
      rm -rf -- "$tmp_dir"
      echo "Cleaned: $tmp_dir"
    fi
  }
}