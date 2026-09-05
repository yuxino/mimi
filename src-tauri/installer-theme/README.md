# yuxino installer theme v1.0.0

Shared lavender/cat NSIS branding for Kiri, Mimi, Satori, Viva, Tick and WNACG.
Fuwa is excluded. The source illustration is cropped from the user-approved design.
Each app keeps its real name and icon; English, Simplified Chinese and Japanese
welcome/finish text follows the installer's language.

## Boundaries

Keep Tauri's stock installer. This changes only NSIS artwork and presentation.
It does not change app versions, bundle IDs, install-directory policy, resources,
shortcuts, user data, updater feeds, signing keys or install/update/uninstall logic.
Native controls and progress remain native, not pixel-identical rounded cards from
the concept board. In-app updater UI, MSI and macOS packages are unchanged.

## Build and reuse

The Windows-only before-build command generates the BMPs, then runs the project's
original frontend build. Node 22+ is required; no npm dependency is added.
`artwork.lock.json` pins an immutable Kiri commit, exact file sizes and SHA-256.
The first build fetches only these small artwork data files from that commit.
Later builds reuse the verified cache. No remote JavaScript or executable is loaded.
Missing network access fails closed. For offline builds, copy the exact locked
`artwork/` directory from Kiri in advance. No font files are distributed.

```sh
node src-tauri/installer-theme/build.mjs
node --test src-tauri/installer-theme/theme.test.mjs
node src-tauri/installer-theme/build.mjs --check
```

For direct `tauri bundle`, run the generator first because that command can skip
before-build hooks. To reuse in another app, copy this folder and merge the NSIS
appearance fields without replacing existing configuration. Keep the original build
command after the generator. Update source art and all six locks together; remove
only this theme's obsolete cached art when deliberately updating its version.

The focused Windows workflow compiles `preview.nsi`; its preview-only executable
installs no application. Before a release, still test real app installation,
cancellation, upgrade, uninstall, all three languages, keyboard focus and 100/150/200%
DPI scaling on each supported Windows architecture. No release is created here.
