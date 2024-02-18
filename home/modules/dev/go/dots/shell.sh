gmt() {
    if test -e go.mod; then
        printf "\033[38;2;62;207;168mReviewing modules...\033[0m\n"
        go mod tidy
    else
        printf "\033[38;2;212;42;51mError: go.mod file not found\033[0m\n" >&2
        return 1
    fi
}
