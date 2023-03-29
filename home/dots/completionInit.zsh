#!bin/zsh

autoload -U compinit && compinit
source <(nao completion zsh)
compdef _nao nao
complete -C "$(which aws_completer)" aws

# Show only Makefile rules unless they aren't defined
zstyle ':completion::complete:make::' tag-order targets variables
