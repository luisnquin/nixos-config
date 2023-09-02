#!bin/zsh

autoload -U compinit && compinit

complete -C "$(which aws_completer)" aws

# source <(nao completion zsh)
# compdef _nao nao

source <(argocd completion zsh)
compdef _argocd argocd

source <(senv completion zsh)

# Show only Makefile rules unless they aren't defined
zstyle ':completion::complete:make::' tag-order targets variables
