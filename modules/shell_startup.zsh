google() {
    search=""
    for term in "$@"; do search="$search%20$term"; done
    xdg-open "http://www.google.com/search?q=$search"
}

emoji() {
    emojis=(🍍 🍝 🥞 🥗 🧋 🍁 🍂 🍃 🌱)

    month_nb=$(date +%m)

    case $month_nb in
    1 | 2) # Summer
        emojis+=(🐚 🌴 🍹 🌻 🏊 ☀️ 👙)

        # Valentine's month
        if [[ $month_nb -eq 2 ]]; then
            emojis+=(💞 🍫 🧸 💐 🌹 💌)
        fi

        ;;
    3 | 4 | 5) # Spring
        emojis+=(🐣 🌳 🍀 🍃 🌈 🌷 🐝 🐇)

        ;;
    9 | 11) # Autumn
        emojis+=(🍂 🥮 ☕ 🌰 🍊)

        if [[ $month_nb -eq 11 ]]; then
            emojis+=(🎂 🍰 🎁 🎉 🎈)
        fi

        ;;
    10) # Halloween
        emojis+=(🐈‍⬛ 🦇 🕷️ 🥀 🍬 🍫 🎃 🍭 ⚰️ 🪦 🫀)

        ;;
    12) # Christmas
        emojis+=(🍷 🎁 🎄 ☃️ ❄️ 🥛 🦌) ;;
    esac

    printf "%s\n" "${emojis[$((RANDOM * ${#emojis[@]} / 32767))]}"
}

upload_img() {
    local image_path=$1

    # Check that the image path is valid
    if ! test -f "$image_path"; then
        printf "Error: '%s' is not a valid image file\n" "$image_path" >&2
        return 1
    fi

    # Make the POST request to the imgbb API
    response=$(curl -f -H "Content-Type: multipart/form-data" \
        -F "image=@$image_path" \
        -X POST "https://api.imgbb.com/1/upload?key=72fd020dc10851413f82a48f9318faa6")

    # Check if the request was successful
    if ! jq -e '.success' <<<"$response" >/dev/null; then
        printf "Error: failed to upload image\n\nResponse: %s\n" "$response" >&2

        return 1
    fi

    # Extract the image URL from the response
    image_url=$(jq -r '.data.url' <<<"$response")

    printf "%s\n" "$image_url"
}

# I'm not wrong leaving this here
gbh() {
    color_end="\033[0m"
    purple="\033[0;95m"
    yellow="\033[0;93m"
    black="\033[0;90m"
    green="\033[0;92m"
    blue="\033[0;94m"

    is_current_branch=0

    em=$(emoji)

    for branch in $(git branch | head -n 15); do
        if [[ "$branch" == "*" ]]; then
            is_current_branch=1

            continue
        fi

        frags=($(echo "$branch" | tr "/" "\n"))

        result=""

        if [[ "$branch" == feat/* ]]; then
            result="$green$frags[1]/$color_end$frags[2]"
        elif [[ "$branch" == fix/* ]]; then
            result="$yellow$frags[1]/$color_end$frags[2]"
        elif [[ "$branch" == refactor/* ]]; then
            result="$green$frags[1]/$color_end$frags[2]"
        elif [[ "$branch" == dev/* ]]; then
            result="$purple$frags[1]/$color_end$frags[2]"
        elif [[ "$branch" == security/* ]]; then
            result="$blue$frags[1]/$color_end$frags[2]"
        elif [[ "$branch" == chore/* ]]; then
            result="$black$frags[1]/$color_end$frags[2]"
        elif [[ "$branch" == "master" || "$branch" == "main" ]]; then
            result="$blue$branch$color_end"
        else
            result="$branch"
        fi

        if [[ $is_current_branch -eq 1 ]]; then
            if [[ $em == "" ]]; then
                em="*"
            fi

            result="$result $em"
            is_current_branch=0
        fi

        # Regex for Jira ticketsssss
        if [[ $(echo "$result" | grep -E --colour=auto "[A-Z0-9]{2,}-\d+*") == "" ]]; then
            echo "$result"
        else
            echo "$result" | grep -E --colour=auto "[A-Z0-9]{2,}-\d+*"
        fi
    done
}

billboard() {
    local city=$(echo "$1" | xargs | tr '[:upper:]' '[:lower:]')

    if [[ "$city" == "" ]]; then
        local output=$(curl --silent http://www.cinerama.com.pe/cines |
            htmlq --pretty .row .container .card .row .col-md-8 .card-body .btn --attribute href |
            awk '{sub("cartelera_cine/",""); print}')
        set -A cities $(echo "$output" | tr '\n' ' ')

        local reset=$(tput sgr0)

        echo "Options:"

        for city in "${cities[@]}"; do
            color=$((RANDOM % 256))
            color_code=$(tput setaf $color)

            city=$(echo $city | sed 's/.*/\L&/; s/[a-z]*/\u&/g')
            echo " - ${color_code}${city}${reset}"
        done

        return
    fi

    # TODO: improve input verification in case of empty response
    curl --silent http://www.cinerama.com.pe/cartelera_cine/$city | htmlq --text .row .container .card .card-header | sed 's/.*/\L&/; s/[a-z]*/\u&/g'
}

if [[ $(ps -p$$ -ocmd=) == *"zsh"* ]]; then hsi() grep "$*" ~/.zsh_history; fi

if [ "$TMUX" = "" ] && [ "$TERM_PROGRAM" != "vscode" ]; then
    exec tmux
fi

#    # Greeting of the date
#    current_date=$(date +%Y-%m-%d)
#    last_startup_date=$(cat /home/$USER/.cache/last_startup_date.txt 2>/dev/null || true)
#
#    if [ "$current_date" != "$last_startup_date" ]; then
#        # This is the first terminal emulator startup of the day
#        tmux send-keys <<EOF
#echo "$current_date" >/home/$USER/.cache/last_startup_date.txt
#echo "HIiiiiiiiiiiiiiiiiiiiiiiiiiiiiiii"
#EOF
#    fi
#
#    unset current_date last_startup_date
