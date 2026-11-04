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
- `state.log` in the same folder records system diagnostic events (start, exit and its reason, power, brightness, network, monitors, hotkeys, Shelf). It contains no track titles, artists, file names or content.
- `%TEMP%\eavely-spike.log` is a diagnostic log written **only** when the `EAVELY_LOG` environment variable is set or after a crash (panic). It can contain the name of an audio device and error messages. It is capped at 512 KB, never contains track titles, artists or file names, and is never sent anywhere; you can delete it at any time.
- CPU, RAM, GPU and network statistics shown in the PC compact view are read locally and never leave your computer.
- Media information (title, cover, progress) comes from the player through the Windows media session and is only displayed.
- File Shelf stores only references to the files you drop; it never uploads or deletes them.

## Registry and system changes
- "Start with Windows" (optional, off by default) adds one value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`. It is removed when you turn the option off or uninstall.
- The installer is per-user and does not need administrator rights.

## Contact
Questions: open an issue at https://github.com/LimaEdu-7x/eavely/issues or write to contact.eavely@gmail.com
