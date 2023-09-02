#!bin/zsh

autoload -U compinit && compinit

complete -C "$(which aws_completer)" aws

source <(senv completion zsh)

# Show only Makefile rules unless they aren't defined
zstyle ':completion::complete:make::' tag-order targets variables
