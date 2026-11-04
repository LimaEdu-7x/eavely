# Code signing policy

Free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org).

> Status: the signing program is **being applied for**. Until it is approved, releases are published **unsigned** and say so in their notes.

## Scope
Only the official release artifacts of this repository (`Eavely.exe`, `Eavely-Setup.exe`) are signed. They are built from the source code in https://github.com/LimaEdu-7x/eavely by a GitHub Actions workflow on GitHub-hosted runners (in preparation: `.github/workflows/release.yml.draft`, to be activated when signing is approved). Artifacts built anywhere else are never submitted for signing.

## Team roles
| Role | Member |
|---|---|
| Authors (commit access) | @LimaEdu-7x |
| Reviewers | @LimaEdu-7x (changes from outside contributors are reviewed before merging) |
| Approvers (signing requests) | @LimaEdu-7x |

## Rules
- Each release is signed only after a manual approval of the signing request.
- Multi-factor authentication is required for the GitHub and SignPath accounts of all team members.
- The build script (`scripts/build-release.ps1`), the CI definition and the signing configuration live in this repository and are reviewed like source code.
- Product name (`Eavely`) and version are taken from `app/Cargo.toml` and are the same across the signed files; the workflow fails if the executable's metadata does not match.
- Only software built from this project's own source code is signed. Third-party libraries are built as part of the project and are listed in `THIRD-PARTY-LICENSES.md`.
- The program contains no malware, no hacking tools and no features that bypass security measures, and does not change the system configuration without the user's choice (start with Windows is optional and off by default; an uninstaller is provided).

## Privacy
See [PRIVACY.md](PRIVACY.md).

## Verification
Compare the file hash with `SHA256SUMS.txt` in the release and check the digital signature in the file properties.

## Revocation and incidents
If a signed binary is found to be harmful or a key/account is compromised, we will stop signing, report it to SignPath Foundation and publish a notice in the repository. Please report problems through [GitHub Issues](https://github.com/LimaEdu-7x/eavely/issues) or GitHub's private vulnerability reporting.
