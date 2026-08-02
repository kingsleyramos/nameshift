# Name Shift Rewrite — Owner Checklist

Everything the AI builder **can't** do for you. Work top to bottom; each phase says when it's needed. Companion to `SPEC.md` (copy this file into the new repo as `docs/OWNER_CHECKLIST.md` if you want it tracked).

---

## 1. Kick off the build — do now

- [x] **Create the new GitHub repo** — `kingsleyramos/nameshift` (decided; local clone at `/Users/kingsleyramos/Developer/Github/nameshift`). Public = unlimited CI minutes; private is fine to start, flip public before Flathub.
- [x] **Seed it with 3 things:**
    - `SPEC.md` (v2 — includes the UX/UI audit) → save as `docs/SPEC.md`
    - `README.next.md` → drop at repo root (builder installs it as `README.md`)
    - App icon master → 1024×1024 PNG (export from your existing icon assets) → `docs/icon-master.png`
- [x] **Prep the Mac** (one-time):
    ```sh
    xcode-select --install                # Xcode Command Line Tools
    curl https://sh.rustup.rs -sSf | sh   # Rust
    brew install node pnpm                # Node 20+ & pnpm
    ```
- [ ] **Start the builder** in the new repo:
    > Build this project per docs/SPEC.md, following its §0 builder contract (including §0.3 — the reference Swift app lives at ../nameshift-swift, read-only) and §23 milestone order. The repo slug is kingsleyramos/nameshift.
- [ ] **When it finishes:** read `docs/DEVIATIONS.md`, run the app on your real files (small test folder first), and compare side-by-side with the current Name Shift. Your existing presets/history should appear automatically (same data directory).
- [ ] Push → confirm **CI is green on all 3 OSes** before anything else.

## 2. Accounts — start early, approvals take days

| Account                                                             | Cost      | What you do                                                                                                                                                                                                                                                                   |
| ------------------------------------------------------------------- | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Apple Developer Program](https://developer.apple.com/programs/)    | $99/yr    | Enroll (still pending from before). Then in Xcode/portal create:**Developer ID Application** cert (direct downloads) + **Apple Distribution** cert & Mac provisioning profile (MAS). Create the app record in App Store Connect with bundle id `com.kingsleyramos.nameshift`. |
| [Microsoft Partner Center](https://partner.microsoft.com/dashboard) | ~$19 once | Register (individual),**reserve the name "Name Shift"**, copy the three Identity values (Name / Publisher / PublisherDisplayName) into `packaging/msix/AppxManifest.xml`.                                                                                                     |
| [Flathub](https://github.com/flathub/flathub)                       | Free      | Nothing until ship time (needs a public repo + tagged release).                                                                                                                                                                                                               |

## 3. GitHub secrets — before the first signed release

Repo → Settings → Secrets and variables → Actions:

| Secret                                     | Where it comes from                                                                                                                   |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------- |
| `APPLE_CERTIFICATE`                        | Developer ID cert exported as`.p12`, then `base64 -i cert.p12 \| pbcopy`                                                              |
| `APPLE_CERTIFICATE_PASSWORD`               | The`.p12` export password                                                                                                             |
| `APPLE_ID` / `APPLE_TEAM_ID`               | Your Apple ID email / 10-char team id                                                                                                 |
| `APPLE_PASSWORD`                           | App-specific password from[appleid.apple.com](https://account.apple.com) → Sign-In & Security                                         |
| `TAURI_SIGNING_PRIVATE_KEY` (+`_PASSWORD`) | Run`pnpm tauri signer generate` once, store the private key here, **back it up** — losing it breaks auto-update for existing installs |

Until these exist, CI still works — it just produces unsigned artifacts.

## 4. Verification passes — before shipping each channel

- [ ] **macOS:** work through `docs/MANUAL_TESTING.md` on a real build (the E2E suite can't run on macOS).
- [ ] **Windows (one day, on your Windows machine):** install the CI installer artifact and use the app on real files; then clone the repo (Rust + Node + pnpm + VS Build Tools C++ workload) and run `pnpm e2e` locally; run `packaging/msix/make-msix.ps1`.
- [ ] **Linux:** install the AppImage in a VM or WSL2, smoke-test import → apply → revert.

## 5. Ship

- [ ] Tag `v2.0.0` → CI drafts a GitHub Release → review artifacts → **Publish** (this is the direct-download channel + auto-update feed).
- [ ] Capture README screenshots (main window, light + dark) → `docs/screenshots/` — builder wired the paths.
- [ ] **Microsoft Store:** upload the MSIX in Partner Center, fill listing (screenshots, description from README), submit. Certification ≈ 1–3 days.
- [ ] **Mac App Store:** follow `packaging/mas/` runbook (build `channel-mas`, `productbuild`, upload via Transporter); in App Store Connect add screenshots (1280×800 or 2560×1600), privacy = **no data collected**, category Utilities; submit for review.
- [ ] **Flathub:** make the repo public, then PR the manifest from `packaging/flatpak/` to `flathub/flathub` per their submission guide.
- [ ] Un-comment the store badge links in `README.md` as each goes live.

## Running costs

Apple **$99/yr** · Microsoft **~$19 once** · Flathub **free** · GitHub CI **free** (public repo). That's everything.

## Later / optional

- [ ] Decide the old repo's fate once the new app is your daily driver (archive it, or keep as-is).
- [ ] Windows code-signing cert for the _direct_ `.exe` (stores sign for you; unsigned direct installers just show a SmartScreen warning). Optional — ~$100+/yr, skippable at launch.
