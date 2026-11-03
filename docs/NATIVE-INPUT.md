# Entrada do mouse na ilha nativa — decisão

**Decisão:** Raw Input (`RegisterRawInputDevices`, só mouse, `RIDEV_INPUTSINK`) em vez de `WH_MOUSE_LL`.

- O hook de baixo nível (`SetWindowsHookEx(WH_MOUSE_LL)`) é global, roda em todo evento de mouse do sistema e é o mecanismo clássico que antivírus sinalizam. Foi removido.
- Raw Input só **recebe** `WM_INPUT` do mouse na nossa própria janela (nada é injetado em outros processos). Teclado não é registrado (nunca vemos teclas).
- Evento puro: sem polling. A cada `WM_INPUT` calculamos a classe do cursor (fora / zona de gatilho / sobre a ilha / dock / monitor) com inteiros e só avisamos a ilha quando a classe muda.
- `WM_INPUT` chega antes do cursor andar: um timer de 12 ms (re-armado a cada evento) relê `GetCursorPos`, garantindo a posição final em repouso.
- Rejeitado: polling adaptativo (acorda mesmo sem uso). Mantido como fallback possível se algum ambiente (sessão remota) não entregar `WM_INPUT`.
- A detecção de tela cheia/maximizada usa `SetWinEventHook` (evento) + debounce de 200 ms + `EnumWindows` pontual.
