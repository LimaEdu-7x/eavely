Set-Location $PSScriptRoot
$exe = Join-Path $PSScriptRoot '..\..\Eavely.exe'  # coloque o Eavely.exe (release) na raiz do repositorio
function Snap($p) { $p.Refresh(); [pscustomobject]@{ priv=[math]::Round($p.PrivateMemorySize64/1MB,1); ws=[math]::Round($p.WorkingSet64/1MB,1); cpu=$p.TotalProcessorTime.TotalSeconds } }
$env:EAVELY_DATA_DIR = "$PWD\d_mem"; Remove-Item -Recurse -Force d_mem -ErrorAction SilentlyContinue; New-Item -ItemType Directory d_mem | Out-Null
$p = Start-Process $exe -PassThru; Start-Sleep 8; $a = Snap $p; Start-Sleep 30; $b = Snap $p
$idle = "parado: privada={0} MB, WS={1} MB, CPU em 30 s={2:N3} s ({3:N3}% de 1 nucleo)" -f $b.priv,$b.ws,($b.cpu-$a.cpu),(100*($b.cpu-$a.cpu)/30)
$p | Stop-Process -Force; Start-Sleep 2
$p2 = Start-Process $exe -ArgumentList '--selftest-anim' -PassThru; $peak=0; $samples=@()
while (-not $p2.HasExited) { try { $s = Snap $p2; if ($s.priv -gt $peak) { $peak = $s.priv }; $samples += $s.priv } catch {}; Start-Sleep -Milliseconds 400 }
$exit = $p2.ExitCode
@($idle, "durante o selftest (abrir/fechar/abas): pico privada=$peak MB, ultima amostra=$($samples[-1]) MB, exit=$exit") | Set-Content mem.txt
