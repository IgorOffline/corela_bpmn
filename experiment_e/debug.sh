#!/bin/sh
# Builds the corela_bpmn gfx app with the C++ bosonoga and runs it in a window:
# its generator writes the C++ of the program, which is compiled with the
# bosonoga runtime and linked with its window, gfx/window.cc.
#
#   BOSONOGA_DIR  the bosonoga directory, ../di_do_bosonoga/bosonoga by default,
#                 whose build.sh must already have built the generator; the
#                 JDK is the one in its ../java
#
# ./debug.sh runs gfx/gfx_corela_bpmn.bosonoga, and ./debug.sh FILE runs
# another gfx program. Needs clang and clang++ on PATH.
set -eu

cd "$(dirname "$0")"

: "${BOSONOGA_DIR:=$PWD/../di_do_bosonoga/bosonoga}"
: "${CC:=clang}"
: "${CXX:=clang++}"

fail() {
  printf '%s\n' "$*" >&2
  exit 1
}

if test "$#" -gt 1; then
  fail "usage: $0 [FILE]"
fi
program=${1:-gfx/gfx_corela_bpmn.bosonoga}

classes=$BOSONOGA_DIR/build/bosonoga/classes
antlr_runtime=$BOSONOGA_DIR/build/antlr/runtime
if ! test -d "$classes" || ! test -d "$antlr_runtime"; then
  fail "bosonoga is not built in $BOSONOGA_DIR, see its build.sh"
fi
set -- "$BOSONOGA_DIR"/../java/jdk*/bin/java
if test "$#" -ne 1 || ! test -x "$1"; then
  fail "$BOSONOGA_DIR/../java must hold exactly one JDK"
fi
java=$1

# corela_bpmn works with the bosonoga of its own version, the one in [meta].
meta_version() {
  awk -F '"' '/^\[/ { meta = ($0 == "[meta]") } meta && /^version *=/ { print $2 }' "$1"
}
corela_bpmn_version=$(meta_version regina.ini)
bosonoga_version=$(meta_version "$BOSONOGA_DIR/regina.ini")
if test -z "$corela_bpmn_version"; then
  fail "regina.ini has no version in [meta]"
fi
if test "$bosonoga_version" != "$corela_bpmn_version"; then
  fail "corela_bpmn $corela_bpmn_version needs bosonoga $corela_bpmn_version, but $BOSONOGA_DIR has bosonoga $bosonoga_version"
fi

CXXFLAGS="-std=c++17 -O0 -g -fsanitize=undefined -fno-sanitize-recover=all -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion -Wshadow"
CFLAGS="-std=c11 -O0 -g -fsanitize=undefined -fno-sanitize-recover=all"
VENDOR="-isystem $BOSONOGA_DIR/sokol -isystem $BOSONOGA_DIR/fontstash -isystem $BOSONOGA_DIR/stb"
RUNTIME="-I $BOSONOGA_DIR/runtime"

# The generator reads regina.ini from here, as the program's paths are.
rm -rf build/cc
mkdir -p build/cc build/obj
"$java" -cp "$classes:$antlr_runtime" bosonoga.generator.Main build/cc "$program"

# The vendored libraries take long to compile, so they are compiled again
# only when bosonoga's copy of them changes.
if ! test -f build/obj/vendor.o || test "$BOSONOGA_DIR/gfx/vendor.c" -nt build/obj/vendor.o; then
  # shellcheck disable=SC2086
  $CC $CFLAGS $VENDOR -c "$BOSONOGA_DIR/gfx/vendor.c" -o build/obj/vendor.o
fi
# bosonoga lets a variable be compared with, or set to, itself, which clang
# would otherwise reject.
# shellcheck disable=SC2086
$CXX $CXXFLAGS -Wno-tautological-compare -Wno-self-assign $RUNTIME -c build/cc/*.cc -o build/obj/program.o
# shellcheck disable=SC2086
$CXX $CXXFLAGS $VENDOR $RUNTIME -c "$BOSONOGA_DIR/gfx/window.cc" -o build/obj/window.o
# shellcheck disable=SC2086
$CXX $CXXFLAGS $RUNTIME -c "$BOSONOGA_DIR/gfx/main.cc" -o build/obj/main.o
# shellcheck disable=SC2086
$CXX $CXXFLAGS build/obj/program.o build/obj/window.o build/obj/main.o build/obj/vendor.o \
  -pthread -lX11 -lXi -lXcursor -lGL -ldl -lm -o build/gfx_runner
exec build/gfx_runner
