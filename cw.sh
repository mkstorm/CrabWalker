# Add this to .bashrc
# source "$HOME/.config/crabwalker/cw.sh"

cw() {
    case "$1" in
        list|add|remove|tree|edit|help|--help|-h)
            command -p cw "$PWD" "$@"
            ;;
        back)
            if [[ "$2" == "-l" ]]; then
                command -p cw "$PWD" "$@"
                return
            fi

            target="$(command -p cw "$PWD" "$@")" || return
            [ -z "$target" ] && return

            if [ -d "$target" ]; then
                cd "$target"
            elif [ -f "$target" ]; then
                cd "$(dirname "$target")"
            fi
            ;;
        *)
            target="$(command cw "$PWD" "$@")" || return
            [ -z "$target" ] && return

            if [ -d "$target" ]; then
                cd "$target"
            elif [ -f "$target" ]; then
                cd "$(dirname "$target")"
            fi
            ;;
    esac
}

#Autocomplete
_cw_complete() {
    local cur
    cur="${COMP_WORDS[COMP_CWORD]}"

    COMPREPLY=()

    # 1️⃣ Filesystem: files + directories
    while IFS= read -r path; do
        if [ -d "$path" ]; then
            COMPREPLY+=("$path/")  # add trailing slash for directories
        else
            COMPREPLY+=("$path")   # keep files as-is
        fi
    done < <(compgen -f -- "$cur")

    # 2️⃣ Favourites from Rust binary
    if command -v cw >/dev/null 2>&1; then
        while IFS= read -r fav; do
            if [ -d "$fav" ]; then
                COMPREPLY+=("$fav/")
            else
                COMPREPLY+=("$fav")
            fi
        done < <(command -p cw --complete "$cur" 2>/dev/null)
    fi
}

# Bind it to cw function
complete -o nospace -F _cw_complete cw
