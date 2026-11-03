cd "$(dirname "$0")"
for v in before after; do rm -rf d_$v; mkdir d_$v
  EAVELY_BENCH=1 EAVELY_BENCH_EXIT=1 EAVELY_FAKE_MEDIA=1 EAVELY_DATA_DIR="$(cygpath -w $PWD/d_$v)" EAVELY_FRAMELOG="$(cygpath -w $PWD/$v.csv)" ./$v.exe
  echo "$v exit=$?"; sleep 2
done
