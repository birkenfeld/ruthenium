#!/bin/zsh

export RUST_BACKTRACE=1
NEEDLE="$1"
if [ -z "$NEEDLE" ]; then NEEDLE=p.th; fi

run-timed() {
  /usr/bin/time --format="%Us user %Ss system %P%% cpu %e total, max RSS %Mk" "$@"
}

run-grep() {
  run-timed grep --color=always -n -E -ri "$@" $NEEDLE tst > /dev/null
}

run-ag() {
  run-timed ag --color "$@" $NEEDLE tst > /dev/null
}

run-ru-lr() {
  run-timed target/release/ru-line-regex --color "$@" $NEEDLE tst > /dev/null
}

run-ru-lp() {
  run-timed target/release/ru-line-pcre --color "$@" $NEEDLE tst > /dev/null
}

run-ru() {
  run-timed target/release/ru --color "$@" $NEEDLE tst > /dev/null
}

run-all() {
  echo -n "Grep: "
  run-grep "$@"
  echo -n "Ag:   "
  run-ag "$@"
  echo -n "RuLR: "
  run-ru-lr "$@"
  echo -n "RuLP: "
  run-ru-lp "$@"
  echo -n "Ru:   "
  run-ru "$@"
}

#cargo build --release || exit 1

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
