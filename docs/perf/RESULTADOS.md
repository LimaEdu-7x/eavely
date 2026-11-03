# v0.1.9 - FPS das letras e Desempenho (medido)

**Causa:** a rolagem das letras (`a.p.scroll`, um `Tw`) nao entrava em `animating_full`; so era repintada pelo laco do equalizador, que acordava a 12 fps com a ilha aberta (`EQ_FPS_OPEN`). Os 75/60 Hz do monitor valiam para forma/pagina, nao para as letras.
**Correcao:** a rolagem passa a pedir um quadro por vblank (`DwmFlush`) enquanto anima; equalizador com 20 fps (reduzido) ou ritmo do monitor (ligado).

Harness interno (devtools, EAVELY_FRAMELOG, midia falsa, cenario 75 = troca de linha com rolagem de 350 ms). Monitor deste PC: 75 Hz.

| Build / modo | Rolagem das letras (intervalo mediano) | Equalizador em regime | CPU do processo no roteiro do bench (250 s) |
|---|---|---|---|
| 0.1.8 (antes) | ~62 ms (16 fps) | 62 ms | 5,01% de 1 nucleo |
| 0.1.9 Maxima fluidez | 13,3 ms (74 fps) | 13,3 ms (monitor) | 10,15% |
| 0.1.9 Equilibrado (padrao) | 13,6 ms (71 fps) | ~61 ms | 5,26% |
| 0.1.9 Economia | salta por linha (sem quadros de rolagem) | ~85 ms | 3,48% |

A CPU inclui capturas de tela e todos os cenarios do bench (mesma carga em todas as linhas: vale a comparacao, nao o valor absoluto).
Exe release final: Eavely.exe 821760 bytes, Eavely-Setup.exe 427226 bytes. Parado: RAM privada 5,6 MB (WS 36 MB), CPU 0,000 s em 30 s. Durante o selftest (ilha aberta/animando): pico 49,8 MB privada.
Arquivos: before.csv/after.csv (quadros), cpu.txt, mem.txt, analyze.py, cpu.ps1, mem.ps1, run*.sh.
