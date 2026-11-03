cd "$(dirname "$0")"
seed(){ rm -rf d_$1; mkdir d_$1; echo "{\"perfPreset\":\"$1\",\"fpsCap\":\"$2\",\"animQuality\":\"$3\",\"lyricsScroll\":\"$4\",\"fxLevel\":\"$5\",\"vizLevel\":\"$6\",\"pcRotate\":$7}" > d_$1/settings.json; }
seed max auto smooth smooth on on 3
seed balanced auto smooth smooth on reduced 5
seed eco 30 light line reduced reduced 8
for v in max balanced eco; do
  EAVELY_BENCH=1 EAVELY_BENCH_EXIT=1 EAVELY_FAKE_MEDIA=1 EAVELY_DATA_DIR="$(cygpath -w $PWD/d_$v)" EAVELY_FRAMELOG="$(cygpath -w $PWD/p_$v.csv)" ./after.exe
  echo "$v exit=$?"; sleep 2
done
