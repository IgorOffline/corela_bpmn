#!/bin/sh
# Builds the corela_bpmn gfx runner against a built bosonoga library and runs a
# gfx app in a window. Bosonoga is found with pkg-config, through its
# bosonoga.pc and bosonoga_gfx.pc:
#
#   BOSONOGA_DIR  the directory of bosonoga.pc, ../di_do_bosonoga/bosonoga by
#                 default, whose build/libbosonoga.a and
#                 build/libbosonoga_gfx.a must already be built
#   GLIB_BUILD    the GLib build, for glib-2.0.pc and the rpath
#
# ./debug.sh runs gfx/gfx_corela_bpmn.bosonoga, and ./debug.sh FILE runs
# another gfx lesson. Set G_MESSAGES_DEBUG=all to see why a program fails.
set -eu

cd "$(dirname "$0")"

: "${BOSONOGA_DIR:=$PWD/../di_do_bosonoga/bosonoga}"
: "${GLIB_BUILD:=/home/igor/dev/glib-2.90.1/build}"
export PKG_CONFIG_PATH="$BOSONOGA_DIR:$GLIB_BUILD/meson-uninstalled"

if test "$#" -gt 1; then
  printf 'usage: %s [FILE]\n' "$0" >&2
  exit 1
fi

if ! pkg-config --exists bosonoga_gfx || ! test -f "$BOSONOGA_DIR/build/libbosonoga.a" ||
  ! test -f "$BOSONOGA_DIR/build/libbosonoga_gfx.a"; then
  printf 'bosonoga is not built in %s, see its build.sh\n' "$BOSONOGA_DIR" >&2
  exit 1
fi

# corela_bpmn works with the bosonoga of its own version, the one in [meta].
corela_bpmn_version=$(awk -F '"' '/^\[/ { meta = ($0 == "[meta]") } meta && /^version *=/ { print $2 }' regina.ini)
bosonoga_version=$(pkg-config --modversion bosonoga)
if test -z "$corela_bpmn_version"; then
  printf 'regina.ini has no version in [meta]\n' >&2
  exit 1
fi
if test "$bosonoga_version" != "$corela_bpmn_version"; then
  printf 'corela_bpmn %s needs bosonoga %s, but %s has bosonoga %s\n' \
    "$corela_bpmn_version" "$corela_bpmn_version" "$BOSONOGA_DIR" "$bosonoga_version" >&2
  exit 1
fi

CFLAGS="-std=c11 -O0 -g -fsanitize=undefined -fno-sanitize-recover=all -fno-omit-frame-pointer -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion -Wshadow -Wcast-qual -Wcast-align -Wwrite-strings -Wstrict-prototypes -Wmissing-prototypes -Wmissing-variable-declarations -Wredundant-decls -Wnull-dereference -Wdouble-promotion -Wfloat-equal -Wformat=2 -Wundef -Wswitch-enum -Wunused -Wunreachable-code -Wvla"

mkdir -p build
clang-format -i --style=Google runner/gfx.c
# shellcheck disable=SC2086
clang $CFLAGS $(pkg-config --cflags bosonoga_gfx | sed 's/-I/-isystem /g') runner/gfx.c $(pkg-config --static --libs bosonoga_gfx) -Wl,-rpath,"$GLIB_BUILD/glib" -o build/gfx_runner
exec build/gfx_runner "${1:-gfx/gfx_corela_bpmn.bosonoga}"
