"""Build a standard Windows MSI using already prepared local runtimes.

No network activity. No campaign files, developer settings, or private source books are packaged.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import uuid

ROOT = Path(__file__).resolve().parent.parent
VERSION = "0.2.1-beta.1"
PRODUCT = "ai.dungeonworld.beta"


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def source_files(app, llama, python, crt):
    """Deliberate allowlist. Never traverse the project or user profile as a payload."""
    files = {"dungeon-world-ai-desktop.exe": app}
    config = json.loads((ROOT / "build/scrying-cef/current.json").read_text(encoding="utf-8-sig"))
    directory = config["directory"]
    if Path(directory).name != directory or directory in (".", ".."):
        raise ValueError("Invalid Scrying Pane runtime directory")
    cef = ROOT / "build/scrying-cef" / directory
    if not (cef / "ScryingPane.exe").is_file():
        raise ValueError("Run the Scrying Pane build first")
    files["scrying-cef/current.json"] = ROOT / "build/scrying-cef/current.json"
    for f in cef.rglob("*"):
        if f.is_file():
            files["scrying-cef/" + directory + "/" + f.relative_to(cef).as_posix()] = f
    for f in llama.iterdir():
        if f.is_file() and (f.name == "llama-server.exe" or f.suffix.lower() == ".dll"):
            if "cuda" in f.name.lower() or f.name.lower().startswith(("cublas", "cudart")):
                raise ValueError("Use the CPU-only portable narrator runtime for this installer")
            files["ai/" + f.name] = f
    if "ai/llama-server.exe" not in files:
        raise ValueError("The llama CPU runtime is incomplete")
    # Redistributable private Python: no development venv, unrelated packages, credentials or caches.
    for name in ("python.exe", "pythonw.exe", "python3.dll", "python313.dll", "vcruntime140.dll", "vcruntime140_1.dll", "LICENSE.txt"):
        f = python / name
        if f.is_file():
            files["python/" + name] = f
    for folder in ("DLLs", "Lib"):
        for f in (python / folder).rglob("*"):
            relative = f.relative_to(python)
            if not f.is_file() or "__pycache__" in relative.parts or f.suffix == ".pyc":
                continue
            if "site-packages" in relative.parts:
                tail = relative.parts[relative.parts.index("site-packages") + 1:]
                if not tail or not (tail[0] == "pip" or tail[0].startswith("pip-") and tail[0].endswith(".dist-info")):
                    continue
            if folder == "Lib" and relative.parts[1] in ("test", "idlelib", "tkinter", "turtledemo"):
                continue
            files["python/" + relative.as_posix()] = f
    for f in crt.glob("*.dll"):
        files[f.name] = f
        files["ai/" + f.name] = f
        files["scrying-cef/" + directory + "/" + f.name] = f
    openmp = crt.parent / "Microsoft.VC143.OpenMP" / "vcomp140.dll"
    if not openmp.is_file():
        raise ValueError("The licensed x64 Visual C++ OpenMP runtime (vcomp140.dll) is required")
    files["ai/vcomp140.dll"] = openmp
    if "python/python.exe" not in files or not any(p.startswith("python/Lib/pip") or p.startswith("python/Lib/site-packages/pip/") for p in files):
        raise ValueError("The private Python runtime needs its bundled pip module")
    if "python/Lib/ensurepip/__init__.py" not in files or not any(p.startswith("python/Lib/ensurepip/_bundled/pip-") and p.endswith(".whl") for p in files):
        raise ValueError("The Python distribution needs ensurepip and its offline pip wheel to create the private virtual environment")
    files["setup/setup_art.py"] = ROOT / "apps/beta-setup/setup_art.py"
    files["LICENSE.txt"] = ROOT / "LICENSE"
    files["Beta-start-here.txt"] = ROOT / "docs/beta-start-here.txt"
    files["Third-party-notices.txt"] = ROOT / "apps/beta-setup/Third-party-notices.txt"
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", required=True, type=Path)
    parser.add_argument("--llama", required=True, type=Path)
    parser.add_argument("--python", default=Path(sys.base_prefix), type=Path)
    parser.add_argument("--crt", required=True, type=Path)
    parser.add_argument("--llama-source", required=True, type=Path)
    parser.add_argument("--output", default=ROOT / "build/beta", type=Path)
    args = parser.parse_args()
    if os.name != "nt":
        raise SystemExit("Build this Windows installer on Windows.")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    helper = output / "DungeonWorldAI.exe"
    compiler = Path(os.environ["WINDIR"]) / "Microsoft.NET/Framework64/v4.0.30319/csc.exe"
    subprocess.run([str(compiler), "/nologo", "/target:winexe", "/platform:x64", "/optimize+", "/utf8output",
                    "/out:" + str(helper), "/win32icon:" + str(ROOT / "apps/desktop/src-tauri/icons/icon.ico"),
                    "/win32manifest:" + str(ROOT / "apps/beta-setup/setup.manifest"),
                    "/reference:System.Windows.Forms.dll", "/reference:System.Drawing.dll", "/reference:System.Net.Http.dll",
                    "/reference:System.Web.Extensions.dll", "/reference:Microsoft.CSharp.dll",
                    str(ROOT / "apps/beta-setup/Setup.cs")], check=True)
    check_env = dict(os.environ, DWA_SETUP_TEST_ROOT=str(output / "installer-checks"))
    subprocess.run([str(helper), "/self-test"], check=True, env=check_env)
    files = source_files(args.app.resolve(), args.llama.resolve(), args.python.resolve(), args.crt.resolve())
    # Source offer for the AGPL app and MIT redistributable native helpers.
    files["licenses/llama-MIT.txt"] = args.llama_source / "LICENSE"
    # Include llama's own third-party license texts when supplied by its source tree.
    for folder in ("licenses",):
        base = args.llama_source / folder
        if base.is_dir():
            for f in base.rglob("*"):
                if f.is_file():
                    files["licenses/llama-third-party/" + f.relative_to(base).as_posix()] = f
    for name in ("cpp-httplib/LICENSE", "nlohmann/LICENSE-jsonhpp", "miniaudio/LICENSE", "stb/LICENSE", "sheredom/LICENSE"):
        path = args.llama_source / "vendor" / name
        if path.is_file():
            files["licenses/" + name.replace("/", "-")] = path
    files = {"app/" + name: file for name, file in files.items()}
    files["DungeonWorldAI.exe"] = helper
    print("Hashing %d public app/runtime files..." % len(files), flush=True)
    manifest_files = [{"path": name, "source": str(file), "size": file.stat().st_size, "sha256": digest(file)} for name, file in sorted(files.items())]
    fingerprint = hashlib.sha256(json.dumps([{k:v for k,v in f.items() if k != "source"} for f in manifest_files], sort_keys=True).encode()).hexdigest()
    product_code = "{" + str(uuid.uuid5(uuid.NAMESPACE_URL, PRODUCT + ":" + VERSION + ":" + fingerprint)).upper() + "}"
    state = output / "installed.json"
    state.write_text(json.dumps({"product": PRODUCT, "version": VERSION, "directory": "app", "productCode": product_code}), encoding="utf-8")
    manifest_files.append({"path": "installed.json", "source": str(state), "size": state.stat().st_size, "sha256": digest(state)})
    pack = {"product": PRODUCT, "version": VERSION, "productCode": product_code,
            "upgradeCode": "{725F73BA-BD25-58E7-A6E3-667C88AC70A5}",
            "expandedBytes": sum(f["size"] for f in manifest_files), "files": manifest_files}
    manifest = output / "payload-files.json"
    manifest.write_text(json.dumps(pack, ensure_ascii=False), encoding="utf-8")
    setup = output / ("Dungeon-World-AI-" + VERSION + "-Windows-Setup.msi")
    subprocess.run([sys.executable, "-I", str(ROOT / "scripts/msi-sdk.py"), str(manifest), str(setup)], check=True)
    (output / (setup.name + ".sha256")).write_text(digest(setup) + "  " + setup.name + "\n", encoding="ascii")
    public_manifest = dict(pack, files=[{k:v for k,v in f.items() if k != "source"} for f in manifest_files])
    (output / "package-manifest.json").write_text(json.dumps(public_manifest, indent=2), encoding="utf-8")
    print("Verified installer: %s (%.1f MB)" % (setup, setup.stat().st_size / 1048576), flush=True)


if __name__ == "__main__":
    main()
