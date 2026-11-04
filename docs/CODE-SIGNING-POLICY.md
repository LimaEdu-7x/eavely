# Code signing status

**The official binaries are currently NOT code-signed.** Code signing is under evaluation (distribution through the Microsoft Store/MSIX and other options are being considered). Releases say so in their notes.

## How to verify a download
- Compare the SHA256 of `Eavely.exe` / `Eavely-Setup.exe` with the `SHA256SUMS.txt` attached to the same GitHub Release.
- Or build it yourself: `powershell -File scripts/build-release.ps1` (see the README). Do not export `RUSTFLAGS` manually; the script sets them.
- The release binaries are built from the public source in this repository by `scripts/build-release.ps1`; no local paths are embedded.

## What Windows may show
- **SmartScreen:** *More info → Run anyway*.
- **Smart App Control** (Windows 11): may block unsigned binaries without a "run anyway" option. Building from source is the safe alternative.

## Safety statements
- The program contains no malware and no features that bypass security measures, and it does not change the system configuration without the user's choice (start with Windows is optional and off by default; an uninstaller is provided).
- Third-party components are listed in `THIRD-PARTY-LICENSES.md`.

## Privacy
See [PRIVACY.md](PRIVACY.md).

## Reporting problems
Please report problems through [GitHub Issues](https://github.com/LimaEdu-7x/eavely/issues), GitHub's private vulnerability reporting, or by e-mail to contact.eavely@gmail.com. If signing is adopted in the future, this page will be updated with the signing process.
