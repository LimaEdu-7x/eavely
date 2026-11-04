# Privacy policy (Eavely)

Last updated: 2026-11-04

Eavely is a local application. It has no account, no telemetry and no analytics, and it does not update itself.

## What the app sends over the network
- **Synced lyrics (optional, on by default):** when a song is playing, Eavely asks [LRCLIB](https://lrclib.net) for its lyrics. The request contains the **track title, artist, album and duration** of the current song and a generic User-Agent (`Eavely/0.1`). Nothing else is sent: no identifier, no file names, no other apps. LRCLIB's own policy applies to that service.
- You can turn this off in **Settings → Music and lyrics → Show lyrics** (*Mostrar letras*). Lyrics already downloaded stay in the local cache and keep working offline.
- The installer shows this policy before installation.

This program will not transfer any other information to other networked systems.

## What stays on your PC
- Settings, tasks, shelf references and the lyrics cache are stored in `%APPDATA%\Eavely`. Deleting that folder removes them.
- `state.log` in the same folder records start, exit and the reason for each exit. It contains no track titles, artists or file names.
- CPU, RAM, GPU and network statistics shown in the PC compact view are read locally and never leave your computer.
- Media information (title, cover, progress) comes from the player through the Windows media session and is only displayed.
- File Shelf stores only references to the files you drop; it never uploads or deletes them.

## Registry and system changes
- "Start with Windows" (optional, off by default) adds one value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`. It is removed when you turn the option off or uninstall.
- The installer is per-user and does not need administrator rights.

## Contact
Questions: open an issue at https://github.com/LimaEdu-7x/eavely/issues
