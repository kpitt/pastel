#!/usr/bin/env bash

# Usage: scripts/all_completions.sh [pastel_cmd]
#
# This script generates completion files for all supported shells.
# The output directory can be specified with either the SHELL_COMPLETIONS_DIR
# or OUT_DIR environment variables, which are checked in that order.  If
# neither variable is set, the script will try to create an `autocomplete`
# directory in the Git repository root.
#
# The only argument to the script is an optional path to a pre-built `pastel`
# executable to use for generating the completion files.  If not specified,
# `cargo run` will be used to run the `completions` command for each shell.

cmd="pastel"

exe="${1:-cargo run --}"
outdir=${SHELL_COMPLETIONS_DIR:-$OUT_DIR}
if [[ -z $outdir ]]; then
    repo_root="$(git rev-parse --show-toplevel)"
    [[ -n $repo_root ]] || exit 1
    outdir="$repo_root/autocomplete"
fi

mkdir -p "$outdir" 2> /dev/null

function gen_completion() {
    local shell=$1
    local fname=$2
    $exe completions $shell > "$outdir/$fname" 2> /dev/null
}

gen_completion bash "$cmd.bash"
gen_completion elvish "$cmd.elv"
gen_completion fish "$cmd.fish"
gen_completion powershell "_$cmd.ps1"
gen_completion zsh "_$cmd"
