# gump

(noun) : directed momentum.

## A smarter cd

Directory jumper using frecency. Type directory fragments, land where you meant.

```
projects           # jumps to ~/code/projects
gmp                # jumps to ~/code/gump (fuzzy)
doc my             # jumps to ~/documents/myfiles (multi-term)
```

No command prefix required.

## Install

**Homebrew**
```bash
brew install tenseleyFlow/tap/gump
```

**AUR**
```bash
yay -S gump
```

**From source**
```bash
cargo build --release
cp target/release/gump ~/.local/bin/
```

## Shell Setup

Add to shell rc file:

**Bash** (`~/.bashrc`)
```bash
eval "$(gump init bash)"
```

**Zsh** (`~/.zshrc`)
```zsh
eval "$(gump init zsh)"
```

**Fish** (`~/.config/fish/config.fish`)
```fish
gump init fish | source
```

Restart shell or source the file.

## Usage

Just type where you want to go:

```bash
projects           # jumps to ~/code/projects
doc                # jumps to ./Documents (fuzzy CWD match)
gmp                # jumps to ~/code/gump (fuzzy database match)
conf fish          # jumps to ~/.config/fish (multi-term)
```

Or use the `g` command:

```bash
g foo              # jump to best match for "foo"
g                  # go home
g -                # go back
gi foo             # interactive selection with fzf
```

Resolution order:
1. Existing commands/aliases/builtins
2. Exact directory in CWD
3. Fuzzy match against CWD contents
4. Fuzzy match against database

Directories are learned automatically as you `cd` around.

## Commands

```
gump add <path>       # manually add directory
gump remove <path>    # remove from database
gump list             # show all entries
gump list --score     # show entries with scores
gump query <terms>    # print best match (for scripts)
gump clean            # remove non-existent directories
gump import           # import from zoxide/autojump/z/fasd
gump edit             # edit database as JSON
```

## Options

```bash
gump init bash --cmd j      # use 'j' instead of 'g'
gump init bash --hook pwd   # only track on directory change (not every prompt)
gump init bash --no-cmd     # skip g/gi aliases, keep no-prefix jumping
```

## Environment

| Variable | Default | Description |
|----------|---------|-------------|
| `GUMP_DATA_DIR` | `~/.local/share/gump` | Database location |
| `GUMP_MAXAGE` | `10000` | Max total score before aging |
| `GUMP_EXCLUDE` | - | Colon-separated paths to ignore |

## How it works

1. Shell hook records directories on `cd`
2. Frecency score = access count × recency multiplier
3. Query matches terms against paths using fuzzy matching
4. Unknown commands are intercepted and checked against the database
