use crate::cli::{Hook, Shell};

use super::Result;

/// Generate shell integration code.
pub fn run(shell: Shell, cmd: String, hook: Hook, no_cmd: bool) -> Result<()> {
    let output = match shell {
        Shell::Bash => generate_bash(&cmd, &hook, no_cmd),
        Shell::Zsh => generate_zsh(&cmd, &hook, no_cmd),
        Shell::Fish => generate_fish(&cmd, &hook, no_cmd),
        Shell::Fortsh => generate_fortsh(&cmd, &hook, no_cmd),
    };

    println!("{}", output);
    Ok(())
}

fn generate_bash(cmd: &str, hook: &Hook, no_cmd: bool) -> String {
    let mut output = String::new();

    // Hook function
    output.push_str(r#"
# gump hook - called after directory changes
__gump_hook() {
    command gump add -- "$PWD"
}
"#);

    // Hook trigger based on type
    match hook {
        Hook::Prompt => {
            output.push_str(r#"
# Update on every prompt
if [[ "${PROMPT_COMMAND:-}" != *'__gump_hook'* ]]; then
    PROMPT_COMMAND="__gump_hook;${PROMPT_COMMAND#;}"
fi
"#);
        }
        Hook::Pwd => {
            output.push_str(r#"
# Update when directory changes
__gump_oldpwd="$PWD"
__gump_pwd_hook() {
    if [[ "$PWD" != "$__gump_oldpwd" ]]; then
        __gump_oldpwd="$PWD"
        __gump_hook
    fi
}
if [[ "${PROMPT_COMMAND:-}" != *'__gump_pwd_hook'* ]]; then
    PROMPT_COMMAND="__gump_pwd_hook;${PROMPT_COMMAND#;}"
fi
"#);
        }
    }

    // Command aliases (unless --no-cmd)
    if !no_cmd {
        output.push_str(&format!(
            r#"
# Jump function
{cmd}() {{
    if [[ $# -eq 0 ]]; then
        builtin cd ~ && __gump_hook
    elif [[ $# -eq 1 && "$1" == "-" ]]; then
        builtin cd - && __gump_hook
    elif [[ $# -eq 1 && -d "$1" ]]; then
        # Explicit path - cd directly without fuzzy matching
        builtin cd -- "$1" && __gump_hook
    else
        local result
        # Try CWD first, then database
        result=$(command gump query --cwd -- "$@" 2>/dev/null)
        if [[ -z "$result" ]]; then
            result=$(command gump query -- "$@" 2>/dev/null)
        fi
        if [[ -n "$result" ]]; then
            builtin cd -- "$result" && __gump_hook
        else
            echo "gump: no match found" >&2
            return 1
        fi
    fi
}}

# Interactive mode with fzf
{cmd}i() {{
    local result
    result=$(command gump query --all -- "$@" | fzf --height=40% --reverse)
    if [[ -n "$result" ]]; then
        builtin cd -- "$result" && __gump_hook
    fi
}}
"#,
            cmd = cmd
        ));
    }

    // The magic: command_not_found_handle for no-prefix jumping
    output.push_str(r#"
# No-prefix directory jumping
command_not_found_handle() {
    # Check if it's a local directory first (exact match)
    if [[ -d "$1" ]]; then
        builtin cd -- "$1" && __gump_hook
        return 0
    fi

    # Fuzzy match against current directory contents
    local result
    result=$(command gump query --cwd -- "$@" 2>/dev/null)
    if [[ -n "$result" ]]; then
        builtin cd -- "$result" && __gump_hook
        return 0
    fi

    # Query gump database
    result=$(command gump query -- "$@" 2>/dev/null)
    if [[ -n "$result" ]]; then
        builtin cd -- "$result" && __gump_hook
        return 0
    fi

    # Fallback to default "command not found" behavior
    echo "bash: $1: command not found" >&2
    return 127
}
"#);

    output
}

fn generate_zsh(cmd: &str, hook: &Hook, no_cmd: bool) -> String {
    let mut output = String::new();

    // Hook function
    output.push_str(r#"
# gump hook - called after directory changes
__gump_hook() {
    command gump add -- "$PWD"
}
"#);

    // Hook trigger
    match hook {
        Hook::Prompt => {
            output.push_str(r#"
# Update on every prompt
[[ -n "${precmd_functions[(r)__gump_hook]}" ]] || precmd_functions+=(__gump_hook)
"#);
        }
        Hook::Pwd => {
            output.push_str(r#"
# Update when directory changes (chpwd hook)
[[ -n "${chpwd_functions[(r)__gump_hook]}" ]] || chpwd_functions+=(__gump_hook)
"#);
        }
    }

    // Command aliases
    if !no_cmd {
        output.push_str(&format!(
            r#"
# Jump function
{cmd}() {{
    if [[ $# -eq 0 ]]; then
        builtin cd ~
    elif [[ $# -eq 1 && "$1" == "-" ]]; then
        builtin cd -
    elif [[ $# -eq 1 && -d "$1" ]]; then
        # Explicit path - cd directly without fuzzy matching
        builtin cd -- "$1"
    else
        local result
        # Try CWD first, then database
        result=$(command gump query --cwd -- "$@" 2>/dev/null)
        if [[ -z "$result" ]]; then
            result=$(command gump query -- "$@" 2>/dev/null)
        fi
        if [[ -n "$result" ]]; then
            builtin cd -- "$result"
        else
            echo "gump: no match found" >&2
            return 1
        fi
    fi
}}

# Interactive mode with fzf
{cmd}i() {{
    local result
    result=$(command gump query --all -- "$@" | fzf --height=40% --reverse)
    if [[ -n "$result" ]]; then
        builtin cd -- "$result"
    fi
}}
"#,
            cmd = cmd
        ));
    }

    // ZLE widget for no-prefix jumping
    output.push_str(r#"
# ZLE widget to intercept commands before execution
__gump_accept_line() {
    local first_word="${BUFFER%% *}"

    # Skip if buffer is empty
    if [[ -z "$BUFFER" ]]; then
        zle .accept-line
        return
    fi

    # Skip paths (starting with . / or ~)
    if [[ "$first_word" == "."* ]] || \
       [[ "$first_word" == "/"* ]] || \
       [[ "$first_word" == "~"* ]]; then
        zle .accept-line
        return
    fi

    # Skip if command exists (whence checks builtins, functions, aliases, and PATH)
    if whence "$first_word" >/dev/null 2>&1; then
        zle .accept-line
        return
    fi

    # Check if it's a local directory (exact match)
    if [[ -d "$first_word" ]]; then
        BUFFER="cd ${(q)first_word}"
        zle .accept-line
        return
    fi

    # Fuzzy match against current directory contents
    local result
    result=$(command gump query --cwd -- $=BUFFER 2>/dev/null)
    if [[ -n "$result" ]]; then
        BUFFER="cd ${(q)result}"
        zle .accept-line
        return
    fi

    # Query gump database
    result=$(command gump query -- $=BUFFER 2>/dev/null)
    if [[ -n "$result" ]]; then
        BUFFER="cd ${(q)result}"
    fi

    zle .accept-line
}

zle -N accept-line __gump_accept_line
"#);

    output
}

fn generate_fish(cmd: &str, hook: &Hook, no_cmd: bool) -> String {
    let mut output = String::new();

    // Hook function based on type
    match hook {
        Hook::Prompt => {
            output.push_str(r#"
# Update on every prompt
function __gump_hook --on-event fish_prompt
    command gump add -- $PWD
end
"#);
        }
        Hook::Pwd => {
            output.push_str(r#"
# Update when directory changes
function __gump_hook --on-variable PWD
    command gump add -- $PWD
end
"#);
        }
    }

    // Command aliases
    if !no_cmd {
        output.push_str(&format!(
            r#"
# Jump function
function {cmd} --description "Jump to a directory"
    if test (count $argv) -eq 0
        cd ~
    else if test (count $argv) -eq 1 -a "$argv[1]" = "-"
        cd -
    else if test (count $argv) -eq 1 -a -d "$argv[1]"
        # Explicit path - cd directly without fuzzy matching
        cd $argv[1]
    else
        # Try CWD first, then database
        set -l result (command gump query --cwd -- $argv 2>/dev/null)
        if test -z "$result"
            set result (command gump query -- $argv 2>/dev/null)
        end
        if test -n "$result"
            cd $result
        else
            echo "gump: no match found" >&2
            return 1
        end
    end
end

# Interactive mode with fzf
function {cmd}i --description "Jump to a directory (interactive)"
    set -l result (command gump query --all -- $argv | fzf --height=40% --reverse)
    if test -n "$result"
        cd $result
    end
end
"#,
            cmd = cmd
        ));
    }

    // Intercept Enter key to check for gump jumps before execution
    output.push_str(r#"
# No-prefix directory jumping via Enter key interception
function __gump_execute
    set -l cmd (commandline -b)
    set -l first_word (string split ' ' -- $cmd)[1]

    # Skip if empty
    if test -z "$first_word"
        commandline -f execute
        return
    end

    # Skip if command exists
    if type -q "$first_word"
        commandline -f execute
        return
    end

    # Skip if starts with path characters
    if string match -qr '^[./~]' -- "$first_word"
        commandline -f execute
        return
    end

    # Check if it's a local directory (exact match)
    if test -d "$first_word"
        commandline -r "cd $first_word"
        commandline -f execute
        return
    end

    # Fuzzy match against current directory contents
    set -l result (command gump query --cwd -- $cmd 2>/dev/null)
    if test -n "$result"
        commandline -r "cd \"$result\""
        commandline -f execute
        return
    end

    # Query gump database
    set -l result (command gump query -- $cmd 2>/dev/null)
    if test -n "$result"
        commandline -r "cd \"$result\""
        commandline -f execute
        return
    end

    # Not a gump match, execute normally
    commandline -f execute
end

bind \r __gump_execute
bind \n __gump_execute
"#);

    output
}

fn generate_fortsh(cmd: &str, hook: &Hook, no_cmd: bool) -> String {
    let mut output = String::new();

    // Hook function
    output.push_str(r#"
# gump hook - called after directory changes
__gump_hook() {
    command gump add -- "$PWD"
}
"#);

    // Hook helper for pwd mode (defined before trap, used later)
    if matches!(hook, Hook::Pwd) {
        output.push_str(r#"
# Track directory changes
__gump_oldpwd="$PWD"
__gump_pwd_hook() {
    if [[ "$PWD" != "$__gump_oldpwd" ]]; then
        __gump_oldpwd="$PWD"
        __gump_hook
    fi
}
"#);
    }

    // Command aliases (unless --no-cmd)
    if !no_cmd {
        output.push_str(&format!(
            r#"
# Jump function
{cmd}() {{
    if [[ $# -eq 0 ]]; then
        command cd ~ && __gump_hook
    elif [[ $# -eq 1 && "$1" == "-" ]]; then
        command cd - && __gump_hook
    elif [[ $# -eq 1 && -d "$1" ]]; then
        command cd -- "$1" && __gump_hook
    else
        local result
        result=$(command gump query --cwd -- "$@" 2>/dev/null)
        if [[ -z "$result" ]]; then
            result=$(command gump query -- "$@" 2>/dev/null)
        fi
        if [[ -n "$result" ]]; then
            command cd -- "$result" && __gump_hook
        else
            echo "gump: no match found" >&2
            return 1
        fi
    fi
}}

# Interactive mode with fzf
{cmd}i() {{
    local result
    result=$(command gump query --all -- "$@" | fzf --height=40% --reverse)
    if [[ -n "$result" ]]; then
        command cd -- "$result" && __gump_hook
    fi
}}
"#,
            cmd = cmd
        ));
    }

    // command_not_found_handle for no-prefix jumping
    output.push_str(r#"
# No-prefix directory jumping
command_not_found_handle() {
    # Check if it's a local directory first (exact match)
    if [[ -d "$1" ]]; then
        command cd -- "$1" && __gump_hook
        return 0
    fi

    # Fuzzy match against current directory contents
    local result
    result=$(command gump query --cwd -- "$@" 2>/dev/null)
    if [[ -n "$result" ]]; then
        command cd -- "$result" && __gump_hook
        return 0
    fi

    # Query gump database
    result=$(command gump query -- "$@" 2>/dev/null)
    if [[ -n "$result" ]]; then
        command cd -- "$result" && __gump_hook
        return 0
    fi

    # Fallback to default "command not found" behavior
    echo "fortsh: $1: command not found" >&2
    return 127
}
"#);

    // Set trap LAST to avoid firing during function definitions above
    match hook {
        Hook::Prompt => {
            output.push_str(r#"
# Update on every prompt (via DEBUG trap)
trap '__gump_hook' DEBUG
"#);
        }
        Hook::Pwd => {
            output.push_str(r#"
# Update when directory changes (via DEBUG trap)
trap '__gump_pwd_hook' DEBUG
"#);
        }
    }

    output
}
