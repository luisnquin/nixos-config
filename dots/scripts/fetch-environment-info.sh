fetch_environment_info() {
    if [ -z "$1" ]; then
        printf "\033[0;38;2;255;110;114mMissing api-key\033[0m\n"

        return 1
    elif [ -z "$2" ]; then
        printf "\033[0;38;2;255;110;114mMissing destination file\033[0m\n"

        return 1
    elif [ -z "$3" ]; then
        printf "\033[0;38;2;255;110;114mMissing file owner\033[0m\n"

        return 1
    fi

    api_key="$1"
    dst_file="$2"
    file_owner="$3"

    geolocation_service_response=$(curl -fsX GET "https://ipgeolocation.abstractapi.com/v1/?api_key=$api_key")
    if [ "$geolocation_service_response" = "" ]; then
        printf "\033[0;38;2;255;110;114mBad response from geolocation service, are you sure that the api-key is correct?\033[0m\n"

        return 1
    fi

    echo "sending request to geolocation service..."

    connection=$(echo "$geolocation_service_response" | jq -r '{ location: .city, ip_address: .ip_address, provider: .connection.organization_name, under_a_vpn: .security.is_vpn }')
    echo "geolocation service data: $geolocation_service_response"

    location=$(echo "$connection" | jq -r '.location')
    echo "sending request to weather service..."

    weather=$(curl -fsX GET "https://wttr.in/$location?format=j1" | jq -r '.current_condition | .[] | { feels_like: .FeelsLikeC, humidity: .humidity, cloudy: .cloudcover, text: (.weatherDesc | .[] | .value) }')
    echo "weather service data: $weather"

    echo "removing old file..."
    rm -f "$dst_file"

    echo "creating new file from collected data..."
    jq -s '{weather: .[0], connection: .[1]}' <<<"$weather $connection" >"$dst_file"

    echo "file will be 0400"
    chmod 0400 "$dst_file"

    echo "file owner will be $file_owner"
    chown "$file_owner":users "$dst_file"
}
