{pkgs, ...}: {
  programs.zsh = {
    enable = true;
    enableCompletion = true;
    completionInit = builtins.readFile ../../dots/completionInit.zsh + builtins.readFile ../../../tools/nyx/completions.zsh;

    history = {
      expireDuplicatesFirst = true;
      save = 10000 * 2;
    };

    plugins = with pkgs; [
      {
        name = "nix-shell";
        file = "nix-shell.plugin.zsh";
        src = fetchFromGitHub {
          owner = "chisui";
          repo = "zsh-nix-shell";
          rev = "v0.5.0";
          sha256 = "0za4aiwwrlawnia4f29msk822rj9bgcygw6a8a6iikiwzjjz0g91";
        };
      }
      {
        name = "you-should-use";
        file = "you-should-use.plugin.zsh";
        src = fetchFromGitHub {
          owner = "MichaelAquilina";
          repo = "zsh-you-should-use";
          rev = "5b316f4af3ac90e044f386003aacdaa0ad606488";
          sha256 = "192jb680f1sc5xpgzgccncsb98xa414aysprl52a0bsmd1slnyxs";
        };
      }
      {
        name = "extract";
        file = "extract.sh";
        src = fetchFromGitHub {
          owner = "xvoland";
          repo = "Extract";
          rev = "439e92c5b355b40c36d8a445636d0e761ec08217";
          sha256 = "1yaphcdnpxcdrlwidw47waix8kmv2lb5a9ccmf8dynlwvhyvh1wi";
        };
      }
      {
        name = "zinsults";
        file = "zinsults.plugin.zsh";
        src = fetchFromGitHub {
          owner = "ahmubashshir";
          repo = "zinsults";
          rev = "1ea07a3d1535d7343344176b73fec7f5c760225c";
          sha256 = "1zi7ni5sri8a8favkk4vxwn6n5sawvvi5lhn9c48hkv0rf5y84nr";
        };
      }
      {
        name = "zsh-better-npm-completion";
        file = "zsh-better-npm-completion.plugin.zsh";
        src = fetchFromGitHub {
          owner = "lukechilds";
          repo = "zsh-better-npm-completion";
          rev = "47e5987ca422de43784f9d76311d764f82af2717";
          sha256 = "0n9pd29rr7y6k5v4rzhpd94nsixsscdmhgvwisvbfz843pfikr5f";
        };
      }
    ];
  };
}
