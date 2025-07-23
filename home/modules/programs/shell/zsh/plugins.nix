{pkgs, ...}: {
  programs.zsh.plugins = with pkgs; [
    {
      name = "nix-shell";
      file = "nix-shell.plugin.zsh";
      src = fetchFromGitHub {
        owner = "chisui";
        repo = "zsh-nix-shell";
        rev = "227d284ab2dc2f5153826974e0094a1990b1b5b9";
        sha256 = "11mkq58ssafvkq8sq27v0dl19mi2myi392nhxgi1q2q9q0gazcaa";
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
      # Such a name, "Auto close and delete matching delimiters in zsh"
      name = "hlissner/zsh-autopair";
      file = "autopair.zsh";
      src = fetchFromGitHub {
        owner = "hlissner";
        repo = "zsh-autopair";
        rev = "376b586c9739b0a044192747b337f31339d548fd";
        hash = "sha256-mtDrt4Q5kbddydq/pT554ph0hAd5DGk9jci9auHx0z0=";
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
    # {
    #   name = "fast-syntax-highlighting"; # The reason why the shell startup is slow
    #   src = fetchFromGitHub {
    #     owner = "zdharma-continuum";
    #     repo = "fast-syntax-highlighting";
    #     rev = "cf318e06a9b7c9f2219d78f41b46fa6e06011fd9";
    #     sha256 = "1bmrb724vphw7y2gwn63rfssz3i8lp75ndjvlk5ns1g35ijzsma5";
    #   };
    # }
  ];
}
