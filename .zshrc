# PATH (siempre primero)
typeset -U path
path=(
  $HOME/.local/bin
  $HOME/.opencode/bin
  $HOME/.cargo/bin
  $path
)

# Historial
HISTFILE=~/.zsh_history
HISTSIZE=10000
SAVEHIST=10000
setopt SHARE_HISTORY HIST_IGNORE_DUPS HIST_IGNORE_SPACE HIST_REDUCE_BLANKS

# Comportamiento tipo fish
setopt AUTO_CD
setopt INTERACTIVE_COMMENTS
setopt NO_BEEP

# Completado (un solo compinit, con caché)
[[ -d ~/.cache/zsh/compcache ]] || mkdir -p ~/.cache/zsh/compcache
fpath=(~/.dots/zsh/completions $fpath)
zstyle ':completion:*' menu select
zstyle ':completion:*' matcher-list 'm:{a-zA-Z}={A-Za-z}' 'r:|[._-]=* r:|=*'
zstyle ':completion:*' list-colors ${(s.:.)LS_COLORS}
zstyle ':completion:*' use-cache yes
zstyle ':completion:*' cache-path ~/.cache/zsh/compcache
autoload -Uz compinit
compinit -C

# Teclas
bindkey -e
autoload -Uz up-line-or-beginning-search down-line-or-beginning-search
zle -N up-line-or-beginning-search
zle -N down-line-or-beginning-search
bindkey '^[[A' up-line-or-beginning-search
bindkey '^[[B' down-line-or-beginning-search
bindkey '^[[H' beginning-of-line
bindkey '^[[F' end-of-line
bindkey '^[[3~' delete-char
bindkey '^[[1;5C' forward-word
bindkey '^[[1;5D' backward-word
bindkey '^H' backward-kill-word

# Prompt (init cacheado)
omp_cfg=~/.cache/oh-my-posh/themes/dracula.omp.json
omp_cache=~/.cache/oh-my-posh/init.zsh
if [[ ! -f $omp_cache || $omp_cfg -nt $omp_cache ]]; then
  oh-my-posh init zsh --config $omp_cfg > $omp_cache
fi
source $omp_cache

# Sugerencias
source /usr/share/zsh/plugins/zsh-autosuggestions/zsh-autosuggestions.zsh

# Resaltado (SIEMPRE al final)
source /usr/share/zsh/plugins/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh



export PATH="$HOME/.npm-global/bin:$PATH"

