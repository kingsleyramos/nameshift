# Mac App Store lane (manual, §20.3)

Behavior deltas in MAS builds are exactly §10.2 (security-scoped bookmarks +
the folder-grant prompt) — nothing else may differ.

## One-time portal setup

1. Apple Developer Program membership (paid).
2. Certificates: **Apple Distribution** and **3rd Party Mac Developer Installer**
   (Keychain Access → Certificate Assistant, upload CSRs in the developer portal).
3. Identifier `com.kingsleyramos.nameshift` with the App Sandbox capability.
4. A Mac App Store provisioning profile for that identifier; download it as
   `packaging/mas/NameShift.provisionprofile` (gitignored).
5. App record in App Store Connect (name "Name Shift", bundle id above,
   category Utilities, privacy: **no data collected**).

## Build & upload

```sh
# 1. Build the sandboxed .app (unsigned builds also work for local smoke).
pnpm tauri build --config src-tauri/tauri.mas.conf.json --no-default-features --features channel-mas

# 2. Sign with the distribution identity + embed the profile.
APP="target/release/bundle/macos/Name Shift.app"
cp packaging/mas/NameShift.provisionprofile "$APP/Contents/embedded.provisionprofile"
codesign --deep --force --options runtime \
  --entitlements packaging/mas/entitlements.plist \
  --sign "Apple Distribution: <YOUR NAME> (<TEAMID>)" "$APP"

# 3. Wrap in an installer package.
productbuild --component "$APP" /Applications \
  --sign "3rd Party Mac Developer Installer: <YOUR NAME> (<TEAMID>)" \
  "Name Shift.pkg"

# 4. Upload with Transporter (App Store Connect → my apps → add build),
#    then submit for review with screenshots (1280×800 or 2560×1600).
```

Verification before upload: run the signed .app, import loose files, decline
the folder-grant prompt → rows badge with the permission message; grant it →
renaming works; quit/relaunch restores watched folders via bookmarks.
