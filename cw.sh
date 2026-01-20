# Add this to .bashrc
# source "$HOME/.config/crabwalker/cw.sh"

cw() {

    if [ $# -eq 0 ]; then
        command -p cw "$PWD"
        return
    fi

    case "$1" in
        list|add|remove|tree|edit|help|--help|-h|-a)
            command -p cw "$PWD" "$@"
            return
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

_cw_complete() {
    local cur
    cur="${COMP_WORDS[COMP_CWORD]}"

    COMPREPLY=()

    #Filesystem: files + directories
    while IFS= read -r path; do
        if [ -d "$path" ]; then
            COMPREPLY+=("$path/")  # add trailing slash for directories
        else
            COMPREPLY+=("$path")   # keep files as-is
        fi
    done < <(compgen -f -- "$cur")

    #Favourites from Rust binary
    while IFS= read -r fav; do
        case "$fav" in
            fav:*)
                fav="${fav#fav:}"
                COMPREPLY+=("$fav")   # NEVER add slash
                ;;
        esac
    done < <(command -p cw --complete "$cur" 2>/dev/null)
}


# Bind it to your cw function
complete -o nospace -F _cw_complete cw
