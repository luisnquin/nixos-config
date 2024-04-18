{lib, ...}: {
  programs = {
    fzf = {
      enable = true;
      enableBashIntegration = true;
      enableZshIntegration = true;
    };

    emoji-fzf.enable = true;
  };

  home.sessionVariables = {
    # https://vitormv.github.io/fzf-themes/#eyJib3JkZXJTdHlsZSI6InJvdW5kZWQiLCJib3JkZXJMYWJlbCI6IiIsImJvcmRlckxhYmVsUG9zaXRpb24iOjAsInByZXZpZXdCb3JkZXJTdHlsZSI6InJvdW5kZWQiLCJwYWRkaW5nIjoiMCIsIm1hcmdpbiI6IjAiLCJwcm9tcHQiOiI+ICIsIm1hcmtlciI6Ij4iLCJwb2ludGVyIjoi4peGIiwic2VwYXJhdG9yIjoi4pSAIiwic2Nyb2xsYmFyIjoi4pSCIiwibGF5b3V0IjoiZGVmYXVsdCIsImluZm8iOiJkZWZhdWx0IiwiY29sb3JzIjoiZmc6I2U4ZDRmMyxmZys6I2QwZDBkMCxiZys6IzIyMTMyNixobDojNGViMGQwLGhsKzojNGViOGRjLGluZm86I2QyZDI4NixtYXJrZXI6Izc4MmNjZSxwcm9tcHQ6IzNkODQ4YixzcGlubmVyOiNhZjVmZmYscG9pbnRlcjojYWY1ZmZmLGhlYWRlcjojODdhZmFmLGJvcmRlcjojMjYyNjI2LGxhYmVsOiNhZWFlYWUscXVlcnk6I2Q5ZDlkOSJ9
    FZF_DEFAULT_OPTS = lib.concatStrings [
      "\"--color='fg:#e8d4f3,fg+:#d0d0d0,bg:-1,bg+:#221326' "
      "--color='hl:#4eb0d0,hl+:#4eb8dc,info:#d2d286,marker:#782cce' "
      "--color='prompt:#3d848b,spinner:#af5fff,pointer:#af5fff,header:#87afaf' "
      "--color='border:#262626,label:#aeaeae,query:#d9d9d9' "
      "--border='rounded' --preview-window='border-rounded' --prompt='> ' "
      "--marker='>' --pointer='◆' --separator='─' --scrollbar='│'\""
    ];
  };
}
