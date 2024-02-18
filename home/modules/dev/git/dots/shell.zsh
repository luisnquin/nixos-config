gbh() {
    typeset -gA branch_color
    branch_color=(
        ['refactor/*']='\033[38;2;70;117;90m'
        ['security/*']='\033[38;2;196;35;57m'
        ['chore/*']='\033[38;2;237;40;158m'
        ['style/*']='\033[38;2;56;93;245m'
        ['perf/*']='\033[38;2;40;237;122m'
        ['docs/*']='\033[38;2;89;87;222m'
        ['dev/*']='\033[38;2;121;48;230m'
        ['ci/*']='\033[38;2;240;222;22m'
        ['feat/*']='\033[0;92m'
        ['master']='\033[0;94m'
        ['fix/*']='\033[0;93m'
        ['main']='\033[0;94m'
    )

    branch_keys=('feat/*' 'fix/*' 'refactor/*' 'ci/*' 'docs/*' 'dev/*' 'security/*' 'perf/*' 'chore/*' 'master' 'main')

    current_branch=$(git branch --show-current)

    git for-each-ref --color=always --sort=-committerdate \
        --format='%(HEAD) %(refname:short) %(color:reset)' refs/heads/ | while read -r branch; do
        if [[ $branch == *"*"* ]]; then
            branch="${branch#* }"
        fi

        for pattern in "${branch_keys[@]}"; do
            if [[ $branch =~ $pattern ]]; then
                frags=($(echo "$branch" | tr '/' ' '))

                if [ ${#frags[@]} = 3 ]; then
                    if [[ "$branch" =~ $current_branch ]]; then
                        branch="${branch_color[$pattern]}${frags[1]}/\033[0m${frags[2]} \033[38;2;93;201;196m← current\033[0m"
                    else
                        branch="${branch_color[$pattern]}${frags[1]}/\033[0m${frags[2]}"
                    fi
                else
                    if [[ "$branch" =~ $current_branch ]]; then
                        branch="${branch_color[$pattern]}${frags[1]}\033[0m \033[38;2;93;201;196m← current\033[0m"
                    else
                        branch="${branch_color[$pattern]}${frags[1]}\033[0m"
                    fi
                fi

                break
            fi
        done

        echo "$branch"
    done
}
