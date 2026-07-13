#!/usr/bin/env bash
# ssh to the 3900x measurement host (wink@192.168.1.101), from a
# normal shell or from inside the Claude Code sandbox.
#
# - Host key is pinned in scripts/known_hosts-3900x (seeded from a
#   trusted ~/.ssh/known_hosts entry), so a stale or missing
#   known_hosts line never blocks either environment.
# - Sandbox: raw TCP egress is blocked; when $HTTPS_PROXY is set
#   (http://user:pass@localhost:PORT), tunnel through that HTTP
#   CONNECT proxy via socat. The host must be in the sandbox
#   network allowlist (.claude/settings.local.json).
# - Sandbox: ~/.ssh/config is unreadable (credential deny); fall
#   back to -F none there, keep the user's config otherwise.
# - Multiplexes over a control socket in $TMPDIR so repeated
#   commands reuse one sshd session on the measurement host.
#
# usage: scripts/ssh-3900x.sh [command...]
set -eu

HOST=192.168.1.101
USER_AT=wink
DIR=$(dirname "$0")

opts=(
    -o BatchMode=yes
    -o ConnectTimeout=10
    -o UserKnownHostsFile="$DIR/known_hosts-3900x"
    -o ControlMaster=auto
    -o ControlPath="${TMPDIR:-/tmp}/cm-%C"
    -o ControlPersist=600
)

[ -r ~/.ssh/config ] || opts+=(-F none)

if [ -n "${HTTPS_PROXY:-}" ]; then
    # http://user:pass@host:port -> socat CONNECT tunnel
    auth=${HTTPS_PROXY#*://}; auth=${auth%@*}
    hostport=${HTTPS_PROXY##*@}; port=${hostport##*:}; port=${port%/}
    opts+=(-o ProxyCommand="socat - PROXY:localhost:%h:%p,proxyport=$port,proxyauth=$auth")
fi

exec ssh "${opts[@]}" "$USER_AT@$HOST" "$@"
