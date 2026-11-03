Set-Location $PSScriptRoot
$out = @()
foreach ($v in 'before','max','balanced','eco') {
  $exe = if ($v -eq 'before') { '.\before.exe' } else { '.\after.exe' }; $d = "d_$v"
  if ($v -eq 'before') { Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue; New-Item -ItemType Directory $d | Out-Null }
  $env:EAVELY_BENCH='1'; $env:EAVELY_BENCH_EXIT='1'; $env:EAVELY_FAKE_MEDIA='1'; $env:EAVELY_DATA_DIR="$PWD\$d"; $env:EAVELY_FRAMELOG="$PWD\c_$v.csv"
  $t0 = Get-Date; $p = Start-Process $exe -PassThru; $last = 0; $peak = 0
  while (-not $p.HasExited) { try { $p.Refresh(); $last = $p.TotalProcessorTime.TotalSeconds; if ($p.PrivateMemorySize64 -gt $peak) { $peak = $p.PrivateMemorySize64 } } catch {}; Start-Sleep -Milliseconds 500 }
  $wall = ((Get-Date) - $t0).TotalSeconds
  $out += "{0} cpu_s={1:N2} wall_s={2:N0} cpu_pct_core={3:N3} peak_priv_MB={4:N1}" -f $v, $last, $wall, (100*$last/$wall), ($peak/1MB)
  Start-Sleep 2
}
$out | Set-Content cpu.txt
