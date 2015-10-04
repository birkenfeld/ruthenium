#!/bin/zsh

export RUST_BACKTRACE=1

run-grep() {
  time grep -ri "$@" p.th tst > /dev/null
}

run-ag() {
  time ag -u "$@" p.th tst > /dev/null
}

run-ru() {
  time target/release/ru "$@" p.th tst > /dev/null
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
echo "List files"
run-all -l
