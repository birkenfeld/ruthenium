#!/bin/zsh

export RUST_BACKTRACE=1
NEEDLE="$1"
if [ -z "$NEEDLE" ]; then NEEDLE=p.th; fi

run-grep() {
  time grep -ri "$@" $NEEDLE tst > /dev/null
}

run-ag() {
  time ag "$@" $NEEDLE tst > /dev/null
}

run-ru() {
  time target/release/ru "$@" $NEEDLE tst > /dev/null
}

run-all() {
  echo -n "Grep: "
  run-grep "$@"
  echo -n "Ag:   "
  run-ag "$@"
  echo -n "Ru:   "
  run-ru "$@"
}

cargo build --release || exit 1

echo "List matches"
run-all
echo
echo "List matches with context"
run-all -C 10
echo
echo "List inverted matches"
run-all -v
echo
echo "List files"
run-all -l
