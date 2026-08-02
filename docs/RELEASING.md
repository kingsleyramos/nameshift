# Releasing Name Shift

Four channels (§20): direct download (self-updating), Mac App Store,
Microsoft Store, Flathub. Store lanes are deliberately manual.

## Version bump

The single source of truth is the root `package.json` version. Sync the other
two and verify:

1. `package.json` → `"version"`
2. `src-tauri/tauri.conf.json` → `"version"`
3. `Cargo.toml` → `[workspace.package] version`

```sh
node scripts/check-versions.mjs
```

## 1. Direct download (GitHub Releases + auto-update)

1. Ensure CI is green on `main`.
2. Tag and push:
   ```sh
   git tag v2.0.0 && git push origin v2.0.0
   ```
3. `release.yml` builds macOS (universal dmg), Windows (NSIS + MSI), Linux
   (AppImage/deb/rpm), generates the updater `latest.json`, and attaches
   everything to a **draft** GitHub Release.
4. Review the draft's artifacts, then **Publish** — publishing is what makes
   the updater endpoint serve the new version.

Signing (all optional until configured — unsigned artifacts still build):

| Secret | Purpose |
|---|---|
| `APPLE_CERTIFICATE` / `APPLE_CERTIFICATE_PASSWORD` | Developer ID signing (base64 .p12 + password) |
| `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` | Notarization |
| `TAURI_SIGNING_PRIVATE_KEY` / `…_PASSWORD` | Updater artifact signatures (`pnpm tauri signer generate`; **back the key up** — losing it breaks auto-update for existing installs) |

After generating the updater keypair, paste the PUBLIC key into
`src-tauri/tauri.direct.conf.json` → `plugins.updater.pubkey` (replacing the
placeholder) and commit it.

First-time setup: create the GitHub repo secrets under Settings → Secrets and
variables → Actions.

## 2. Mac App Store

Follow `packaging/mas/README.md` (build `channel-mas`, sign with Apple
Distribution, `productbuild`, upload with Transporter). First-time setup —
certificates, provisioning profile, App Store Connect record — is in the same
runbook.

## 3. Microsoft Store

1. On Windows, build the store binary and pack:
   ```powershell
   pnpm tauri build --no-default-features --features channel-msstore
   pwsh packaging/msix/make-msix.ps1
   ```
2. First time: register in Partner Center, reserve the name "Name Shift",
   and copy the three Identity values into `packaging/msix/AppxManifest.xml`.
3. Upload `packaging/msix/NameShift.msix` in Partner Center, fill the listing
   (screenshots, README description), submit. Certification ≈ 1–3 days.

## 4. Flathub

1. The repo must be public with a tagged release.
2. Generate offline cargo/node sources with Flathub's builder-tools and add
   them to `packaging/flatpak/io.github.kingsleyramos.NameShift.yml`.
3. Validate: `flatpak-builder --show-manifest packaging/flatpak/io.github.kingsleyramos.NameShift.yml`
4. PR the manifest to `flathub/flathub` per their submission guide. The
   `--filesystem=host` justification lives as a comment in the manifest
   (§10.3).

## Post-release

- Un-comment the store badge links in `README.md` as each channel goes live.
- Store channels never self-update (§24 Q19) — each store handles updates.
