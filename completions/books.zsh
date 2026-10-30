# Capture the expanded alias before clap's completion function shifts words.
_reading() {
    local _reading_executable=$words[1] _reading_store='' _reading_command=''
    local -i i
    for (( i=2; i <= $#words; i++ )); do
        case $words[i] in
            (-s|--store)
                (( i++ ))
                _reading_store=$words[i]
                ;;
            (--store=*) _reading_store=${words[i]#--store=} ;;
            (-s?*) _reading_store=${words[i]#-s} ;;
            (*) _reading_command=$words[i]; break ;;
        esac
    done
    _reading_generated "$@"
}

_reading_books() {
    [[ -n $_reading_store ]] || return 1
    local row
    local -a ids descriptions
    while IFS= read -r row; do
        [[ $row == *$'\t'* ]] || continue
        ids+=("${row%%$'\t'*}")
        descriptions+=("${row#*$'\t'}")
    done < <("$_reading_executable" -s "$_reading_store" __complete-books --command "$_reading_command" -- "${(Q)PREFIX}" 2>/dev/null)
    (( $#ids )) || return 1
    # The helper has already matched the title, author, or ID. Insert only IDs.
    compadd -U -d descriptions -a ids
}
