# Workspace development and live testing

The installed beta remains the stable tester copy. Workspace updates build from the current source and keep their own selected campaign folder. They do not replace the installed program, rebuild the distributed MSI, or copy installed campaigns into the workspace.

## Open the workspace app

Open **Start-Dungeon-World-AI.cmd** from File Explorer or run it in your normal PowerShell window. The launcher checks the source files and selected executable against its build record. It builds the current UI and release app automatically when the record is missing, the source changes, or a different executable is selected. A failed update stops the launch and leaves the prior selection available; it does not silently open an older app.

The window title says **Dungeon World AI - Workspace** followed by the app version. Close a previously opened workspace window before testing the new one: rebuilding cannot replace code already loaded in a running process.

To prepare a build without opening it, use **Update-Dungeon-World-AI.cmd** or `npm run build:native`. Automated development sessions can run this build command. The successful build writes `build/native/build-info.json` with its source fingerprint, executable checksum, app version, build time and release profile. Save-folder selection stays in ignored `build/native/launch-settings.json`; the existing folder is preserved. A new workspace defaults to `build/native/saved-games`.

Read the selected app, save folder, model and runtime paths without starting any process:

```powershell
node scripts/native-dm.mjs --check
```

The report includes `build.current` and `interactiveLaunchAllowed`. Fingerprints cover desktop, frontend, engine, narrator, embedded art worker, bundled reference/art files and build configuration. Saved campaigns, models and logs are excluded. Existing local AI, art and CEF runtimes are reused; they have their own setup/build steps.

## Windows socket error 10013

Microsoft defines 10013 (`WSAEACCES`) as socket access denied by permissions. In the restricted development runner used for this project, `CODEX_SANDBOX_NETWORK_DISABLED=1` indicates that networking is disabled. A child app launched there inherits those restrictions; the local model can load while its loopback connection fails. Desktop integration can also differ from a normal user-launched app.

The workspace and hot-reload launchers now stop immediately when that known restriction is present. They print instructions for the user to open the workspace launcher in a normal Windows session. No alternate launch surface, firewall change, administrator elevation or sandbox exemption is attempted. This is a diagnostic guard, not a change to Windows permissions.

Compilation, unit tests, read-only build checks and direct offline model inference can run in the restricted session. Live DM networking, clipboard/file-dialog acceptance checks, and installation tests need an appropriately permitted normal test session. A 10013 error in a normal user launch still needs its own log investigation; the code does not assume every 10013 is caused by this runner.

Reference: [Microsoft Windows Sockets error codes](https://learn.microsoft.com/en-us/windows/win32/winsock/windows-sockets-error-codes-2).
