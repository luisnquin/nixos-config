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

gbh() {
    typeset -gA branch_color
    branch_color=(
        ["refactor/*"]="\033[38;2;70;117;90m"
        ["security/*"]="\033[38;2;196;35;57m"
        ["chore/*"]="\033[38;2;237;40;158m"
        ["perf/*"]="\033[38;2;40;237;122m"
        ["dev/*"]="\033[38;2;121;48;230m"
        ["feat/*"]="\033[0;92m"
        ["fix/*"]="\033[0;93m"
        ["master"]="\033[0;94m"
        ["main"]="\033[0;94m"
    )

    branch_keys=("feat/*" "fix/*" "refactor/*" "dev/*" "security/*" "perf/*" "chore/*" "master" "main")

    current_branch=$(git branch --show-current)

    git for-each-ref --color=always --sort=-committerdate \
        --format='%(HEAD) %(refname:short) %(color:reset)' refs/heads/ | while read -r branch; do
        if echo $branch | grep -q '*'; then
            branch="${branch#* }"
        fi

        for pattern in $branch_keys; do
            if [[ $branch =~ $pattern ]]; then
                frags=($(echo "$branch" | tr '/' ' '))

                if [ ${#frags[@]} = 3 ]; then
                    if [[ "$branch" =~ "$current_branch" ]]; then
                        branch="${branch_color[$pattern]}${frags[1]}/\033[0m${frags[2]} \033[38;2;93;201;196m← current\033[0m"
                    else
                        branch="${branch_color[$pattern]}${frags[1]}/\033[0m${frags[2]}"
                    fi
                else
                    if [[ "$branch" =~ "$current_branch" ]]; then
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

billboard() {
    local city=$(echo "$1" | xargs | tr '[:upper:]' '[:lower:]')

    get_available_cities() { # TODO: mix with cinestar places and discard any in case of not found
        curl --silent http://www.cinerama.com.pe/cines |
            htmlq --pretty .row .container .card .row .col-md-8 .card-body .btn --attribute href |
            awk '{sub("cartelera_cine/",""); print}' | tr '\n' ' '
    }

    # Returns options in case of no passed argument
    if [[ "$city" == "" ]]; then
        set -A cities $(get_available_cities)

        local reset=$(tput sgr0)

        echo "Options:"

        for city in "${cities[@]}"; do
            color=$((RANDOM % 256))
            color_code=$(tput setaf $color)

            city=$(echo $city | sed 's/.*/\L&/; s/[a-zñ]*/\u&/g')
            echo " - ${color_code}${city}${reset}"
        done

        return
    fi

    title_city=$(echo "$city" | sed 's/[^ ]\+/\L\u&/g')

    # Location
    headquarter_id=$(curl -s -X GET 'https://api.cinestar.pe/api/v1/headquarters' | jq -r ".data | .[] | select(.city.name == \"$title_city\") | .id")

    if [[ $headquarter_id == "" ]]; then
        available_cities=$(get_available_cities)

        echo "\033[0;31minvalid argument, options: $available_cities\033[0m"

        return 1
    fi

    cinerama_response=$(curl -s -X GET http://www.cinerama.com.pe/cartelera_cine/$city)
    cinestar_response=$(curl -s -X GET "https://api.cinestar.pe/api/v1/movies?headquarter_id=$headquarter_id&is_next_releases=false")

    cinerama_raw_movies=$(echo "$cinerama_response" | htmlq --text .row .container .card .card-header | sed 's/.*/\L&/; s/[a-zñ]*/\u&/g')
    cinestar_raw_movies=$(echo "$cinestar_response" | jq -r '.data | map(.name) | .[]' | sed 's/.*/\L&/; s/[a-zñ]*/\u&/g')

    declare -a cinerama_movies_schedules
    index=1

    while read -r schedule; do
        # Sometimes at dawn there's nothing added for the current day
        if [[ "$schedule" == "" ]]; then
            break
        fi

        schedule_timestamp=$(date --date "$schedule" +%s)
        latest_schedules=($(echo "${cinerama_movies_schedules[-1]}" | tr ',' ' '))
        last_unix_schedule=$(date --date "${latest_schedules[-1]}" +%s)

        if [ -z ${latest_schedules[-1]} ]; then # if last_unix_schedule is null
            cinerama_movies_schedules+=("$(date -d @$schedule_timestamp +'%H:%M')")

            continue
        fi

        if [[ $schedule_timestamp > $last_unix_schedule ]]; then
            latest_schedules+=($(date -d @$schedule_timestamp +'%H:%M'))
            cinerama_movies_schedules[$index]=$(printf '%s\n' "$(IFS=, printf '%s' "${latest_schedules[*]}")")
        else
            cinerama_movies_schedules+=($(date -d @$schedule_timestamp +'%H:%M'))
            let index=index+1
        fi

    done <<<"$(echo "$cinerama_response" | htmlq --text .row .container .card .row .col-md-9 .card-body .row .ml-1)"

    echo "Cinerama:"
    index=1

    while read -r movie; do
        schedules="${cinerama_movies_schedules[$index]}"

        if [[ "$schedules" == "" ]]; then
            schedules="(not available)"
        fi

        printf " - %-50s %s\n" "$movie" "$schedules"
        let index=index+1
    done <<<"$cinerama_raw_movies"

    printf "\nCinestar:\n"

    # No schedules by now, but https://api.cinestar.pe/api/v1/movies-times?date=2023-01-08&movie_id=2254
    while read -r movie; do echo " - $movie"; done <<<"$cinestar_raw_movies"
}

remove_node_modules() {
    find . -name "node_modules" -type d -prune -exec rm -rf {} \;
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
