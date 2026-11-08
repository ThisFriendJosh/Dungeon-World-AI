# Windows beta installer

Testers open `Dungeon-World-AI-0.2.2-beta.1-Windows-Setup.msi`, choose Install, then Finish. The MSI installs for the current Windows account, creates desktop and Start menu shortcuts, and opens the graphical local AI setup when selected. No Node, Rust, terminal, or developer folder is needed by a tester.

The package includes the release app, complete current CEF Scrying Pane runtime, portable CPU llama.cpp engine, private Python distribution, and native runtime DLLs including OpenMP. Models are explicit, separate downloads. Narration uses the CPU in this beta; 16 GB RAM is recommended. Optional art setup selects CUDA 13 packages when a compatible NVIDIA driver is present, and CPU packages otherwise. CPU image generation can be slow.

The narrator is Qwen3-4B Q4_K_M at revision `bc640142c66e1fdd12af0bd68f40445458f3869b`, verified against SHA-256 `7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5`. Interrupted downloads resume where supported. Optional art setup uses the app's existing pinned SD-Turbo revision, installs pinned top-level package versions, and verifies imports before marking the model ready. Inference never uses a hosted AI provider.

WebView2 is detected using Microsoft's registered runtime. If absent, the setup assistant downloads the official bootstrapper, checks Windows Authenticode trust and Microsoft publisher identity, and runs its per-user setup. This is a setup-only network operation. The beta MSI itself is unsigned. Windows security settings are never changed.

## Data and removal

| Content | Location |
| --- | --- |
| Program | `%LOCALAPPDATA%\Programs\Dungeon World AI Beta` |
| Campaigns and uploaded art | `%LOCALAPPDATA%\DungeonWorldAI\Beta\saved-games` |
| Models | `%LOCALAPPDATA%\DungeonWorldAI\models` |
| Optional art Python | `%LOCALAPPDATA%\DungeonWorldAI\runtimes\art-env\Scripts\python.exe` |
| Setup log | `%LOCALAPPDATA%\DungeonWorldAI\Beta\setup.log` |

Windows Installer owns copying, rollback, repair, upgrade detection, shortcuts, and Add/Remove Programs registration. Uninstall removes registered program files and retains campaign data and models. The builder uses an explicit allowlist and verifies every file's SHA-256 before packaging. Developer launch settings, saved campaigns, imported source books, model caches and logs are excluded. A public checksum and file manifest accompany each package.

The art setup creates a real virtual environment from the bundled Python, then bootstraps pip from its bundled ensurepip wheel without a network request. The base's `EXTERNALLY-MANAGED` marker is retained. Packages install only into the art environment. This fixes beta 0.2.0's attempt to install into a managed base distribution; no package-protection override is used. Existing model downloads and campaign saves are reused when upgrading to 0.2.1. The legacy `art-python` folder is left untouched.

Pip output is explicitly captured from both streams and forwarded into the setup log, including the error shown in the assistant. Package readiness is checked in a fresh process so imported Windows DLLs are released before a repair. Concurrent stdout/stderr log writes are serialized.

## Maintainer build

Build the UI and release app, then prepare the existing Scrying Pane runtime:

```powershell
npm run build:ui
cargo build --offline --release --manifest-path apps/desktop/src-tauri/Cargo.toml --features tauri/custom-protocol
```

Build a portable CPU-only llama-server from a licensed llama.cpp source checkout. Disable CUDA and native tuning; the baseline build also disables SSE4.2, AVX, AVX2, BMI2, FMA and F16C. Disable the web UI, remote prebuilt UI download, OpenSSL and llama-connect. MSBuild may need one worker inside an app-container workspace. None of these compiler requirements apply to testers.

Run `python scripts/build-beta.py --help`. Supply the release app, CPU runtime directory, Python 3.13 base distribution with pip, licensed Visual C++ x64 CRT redist directory (with the adjacent OpenMP directory), and llama source directory for notices. Install the Windows SDK's Windows Installer development tools on the build PC. The builder performs no downloads. It compiles the GUI helper, generates declarative IDT tables, and invokes Microsoft's signed MsiDb and Windows MakeCab to create an MSI with an embedded LZX cabinet. Outputs go to ignored `build/beta` unless `--output` is supplied. Use a new output directory for each build.

Run `tests/beta-installer.test.py` with `DWA_BETA_TEST_HELPER` pointing at the compiled DungeonWorldAI.exe and `DWA_BETA_MANIFEST` at package-manifest.json. Tests cover the payload exclusions, helper path confinement and atomic state changes, and CPU/NVIDIA setup choices. Cabinet expansion and SHA-256 comparison verify the packaged bytes independently.

Run `tests/beta-art-runtime.test.py` with `DWA_BETA_ART_BASE_PYTHON` pointing at a staged copy of the exact packaged Python. The offline tests create a virtual environment in a path containing spaces, bootstrap pip, install a local fixture wheel through the real setup wrapper, retain a real pip failure, and verify the managed base is unchanged.

Every successful build refreshes `build/beta-release/DungeonWorldAI-Beta-Setup.msi` with its checksum and version sidecar. Additional local distribution filenames can be listed in ignored `build/beta-publish-settings.json` using `{"schemaVersion":1,"destinations":["absolute/path/to/setup.msi"]}`. These local destinations are never packaged or published. Perform SDK cabinet extraction on a disposable copy of the MSI so inspection cannot change already published installer bytes or checksums. Refresh copies of an existing verified release with `python scripts/build-beta.py --stage-existing path/to/versioned-installer.msi`; a mismatched version or checksum is rejected.

Workspace launches honor canonical, timestamped and versioned executable filenames. An invalid selection fails clearly instead of opening an older build. Rebuilding preserves the existing workspace save folder. `node scripts/native-dm.mjs --check` prints the selected executable and checksum without opening the app. Installed beta saves and workspace saves remain separate unless deliberately migrated.

For interactive QA, install to an isolated `INSTALLDIR` with `CREATE_DESKTOP=0`, an isolated `DwaMenu`, and `DWA_NO_LAUNCH=1`. Set the helper's `DWA_SETUP_TEST_ROOT` to the same path to keep test campaigns beneath it. Do not target existing user data. Full install/repair/uninstall and initial model downloads still require a normal Windows test session when automation cannot access Windows Installer or the network.
