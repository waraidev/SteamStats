### Bundle 8: Distribution Packaging
> Stage: integration | Parallel: yes (file-disjoint with Bundle 7; start after Bundle 1 completes) | Files: .github/workflows/release.yml, install.sh, scoop/steam-stats.json, README.md

**Bundle Verify**: Release workflow configuration covers 3 targets; install.sh detects architecture correctly.
- **Level**: inspection
- **Given**: release.yml, install.sh, scoop/steam-stats.json, README.md all created
- **Action**: `shellcheck install.sh` + read release.yml matrix
- **Outcome**: shellcheck passes; workflow matrix includes x86_64-apple-darwin, aarch64-apple-darwin, x86_64-pc-windows-msvc; install.sh has arch detection branch for arm64/x86_64

> **Context**
>
> **Applicable ACs**
> - **AC-8.1**: Given: git tag matching v* pushed / When: GitHub Actions workflow runs / Then: Binaries built for x86_64-apple-darwin, aarch64-apple-darwin, x86_64-pc-windows-msvc and attached to GitHub Release
> - **AC-8.2**: Given: GitHub Release exists / When: `brew install` (via install.sh) / Then: Binary installs and `steam-stats --version` works
> - **AC-8.3**: Given: GitHub Release exists / When: `scoop install steam-stats` / Then: Binary installs and `steam-stats --version` works
>
> **Architecture Decisions**
> - **AD-7: All distribution from the main repo** — No separate Homebrew tap repo needed. Mac/Linux: `install.sh` in repo root (curl | sh). Windows: `scoop/steam-stats.json` manifest; users add this repo as a Scoop bucket. Release workflow auto-updates Scoop manifest SHA256 + URL on each tagged release.
>
> **Standards**
> - **S-3**: API keys must not appear in source code, git history, or log output — README must not instruct users to hardcode key anywhere except config.toml (Domain: security)
>
> **Constraints**
> - reqwest must use rustls-tls (no OpenSSL) for cross-compilation to work in CI (Category: compatibility)
>
> **Risks**
> - Cross-compilation for Windows may fail in CI due to missing MSVC target (Impact: Medium | Mitigation: Use GitHub-hosted windows-latest runner rather than cross-compilation from Linux)

---

#### STEP-26: Create .github/workflows/release.yml
[FR-8 -> AC-8.1] | create `.github/workflows/release.yml` | Effort: M

> **Intent**: Windows cross-compilation from Linux is risky (Risk table) — use `windows-latest` GitHub runner instead to get native MSVC toolchain. The `--locked` flag on `cargo build --release` ensures reproducible builds using the committed `Cargo.lock`. The release workflow must compute SHA256 of the Windows binary to update `scoop/steam-stats.json` — do this in the post-release job using `shasum -a 256` (macOS/Linux) or `certutil -hashfile` (Windows).

- Trigger: `on: push: tags: ['v*']`
- `env: CARGO_TERM_COLOR: always`
- Matrix jobs: `{ name: mac-x86, os: macos-13, target: x86_64-apple-darwin }`, `{ name: mac-arm64, os: macos-latest, target: aarch64-apple-darwin }`, `{ name: windows, os: windows-latest, target: x86_64-pc-windows-msvc }`
- Each job: `rustup target add $TARGET`, `cargo build --release --locked --target $TARGET`, archive binary as `steam-stats-$TARGET.tar.gz` (or `.zip` for Windows), `actions/upload-artifact`
- Release job (after matrix): `gh release create $GITHUB_REF_NAME` with all artifacts
- Scoop update job: download Windows artifact, compute SHA256, use `jq` to update `scoop/steam-stats.json` `hash` and `url` fields, `git commit` and `git push`
- Include `--no-default-features --features rustls-tls` if needed by reqwest config

**Verify**:
- Level: inspection | Given: release.yml created | Action: Read workflow file | Outcome: 3 matrix targets present; trigger is `v*` tag; `--locked` flag present on cargo build

> **Standards**:
> - S-3: No secrets hardcoded in workflow — GITHUB_TOKEN used for release creation; no Steam API key in CI

> Depends on: STEP-1 (Cargo.lock must exist for --locked to work; will exist after first `cargo build`) | Enables: STEP-27 | Parallel with: STEP-23

---

#### STEP-27: Create install.sh + scoop/steam-stats.json + README.md
[FR-8 -> AC-8.2, AC-8.3] | create `install.sh`, create `scoop/steam-stats.json`, create `README.md` | Effort: S

> **Intent**: `install.sh` must detect architecture with `ARCH=$(uname -m)` and branch on `arm64` vs `x86_64` — a hardcoded ARM URL fails silently on Intel Macs without an error message. The Scoop manifest `hash` and `url` fields are placeholders that the release workflow overwrites — they should contain clear placeholder text (e.g., `"PLACEHOLDER_UPDATED_BY_CI"`) so a stale manifest is obvious. README must document that Steam profile must be set to Public for the API to return data (AC per spec).

- `install.sh`: `#!/bin/bash set -e`; detect `ARCH=$(uname -m)`; set `BINARY_URL` based on arch (arm64 → aarch64-apple-darwin, x86_64 → x86_64-apple-darwin); `curl -sSfL "$BINARY_URL" | tar xz -C /tmp`; `sudo mv /tmp/steam-stats /usr/local/bin/steam-stats`; `echo "steam-stats installed"`
- `scoop/steam-stats.json`: valid Scoop manifest with `version`, `url: "PLACEHOLDER_UPDATED_BY_CI"`, `hash: "PLACEHOLDER_UPDATED_BY_CI"`, `bin: "steam-stats.exe"`, `checkver` config
- `README.md`: Quick Start section (config prompt on first run), Install section (install.sh curl | sh, Scoop bucket add command), Config section (config.toml location, Steam ID format), Limitations section (session estimation label, snapshot history needed), public profile requirement note

**Verify**:
- Level: inspection | Given: install.sh created | Action: `shellcheck install.sh` | Outcome: No shellcheck errors; arch detection branch for arm64 and x86_64 present
- Level: inspection | Given: README.md | Action: Read | Outcome: Steam profile must be public is documented; config.toml path is stated; "Est. Sessions" is explained as an estimate

> **Standards**:
> - S-3: README must not instruct hardcoding API key anywhere except config.toml

> Depends on: STEP-26 | Enables: STEP-28 | Parallel with: STEP-25
