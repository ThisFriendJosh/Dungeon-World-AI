"""Download verified official NuGet packages and compile the small Windows CEF host."""
import base64
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parent.parent
VERSION = "156.0.150"
PACKAGES = {"cefsharp.winforms": VERSION, "cefsharp.common": VERSION, "chromiumembeddedframework.runtime.win-x64": "156.0.15", "system.runtime.interopservices.runtimeinformation": "4.3.0"}


def fetch_json(url):
    with urllib.request.urlopen(url, timeout=45) as response:
        data = response.read()
    # NuGet's registration5-gz endpoint serves gzip even without Accept-Encoding.
    if data.startswith(b"\x1f\x8b"):
        data = gzip.decompress(data)
    return json.loads(data.decode("utf-8-sig"))


def package(name, version):
    cache = ROOT / ".cache" / "scrying-packages"
    cache.mkdir(parents=True, exist_ok=True)
    archive = cache / (name + "." + version + ".nupkg")
    directory = cache / (name + "." + version)
    receipt = directory / ".verified"
    if receipt.is_file() and archive.is_file():
        with archive.open("rb") as source:
            digest = base64.b64encode(hashlib.file_digest(source, "sha512").digest()).decode("ascii")
        if digest == receipt.read_text(encoding="utf-8"):
            return directory
    record = fetch_json("https://api.nuget.org/v3/registration5-gz-semver2/" + name + "/" + version + ".json")["catalogEntry"]
    if isinstance(record, str):
        if not record.startswith("https://api.nuget.org/v3/catalog0/"):
            raise RuntimeError("Unexpected NuGet catalog location.")
        record = fetch_json(record)
    if record.get("packageHashAlgorithm", "").upper() != "SHA512":
        raise RuntimeError("NuGet did not provide a SHA512 package hash.")
    expected = record["packageHash"]
    def valid():
        if not archive.is_file():
            return False
        with archive.open("rb") as source:
            digest = hashlib.file_digest(source, "sha512").digest()
        return base64.b64encode(digest).decode("ascii") == expected
    if not valid():
        print("Downloading " + name + " " + version + "…", flush=True)
        url = "https://api.nuget.org/v3-flatcontainer/" + name + "/" + version + "/" + name + "." + version + ".nupkg"
        with urllib.request.urlopen(url, timeout=180) as source, archive.open("wb") as target:
            shutil.copyfileobj(source, target)
        if not valid():
            raise RuntimeError("The NuGet package hash did not match: " + name)
    if not (directory / ".verified").is_file():
        directory.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(archive) as source:
            for entry in source.infolist():
                target = (directory / entry.filename).resolve()
                if not target.is_relative_to(directory.resolve()):
                    raise RuntimeError("Unsafe package path.")
                source.extract(entry, directory)
        (directory / ".verified").write_text(expected, encoding="utf-8")
    return directory


def main():
    if os.name != "nt":
        raise RuntimeError("The CEF host setup currently targets Windows x64.")
    compiler = Path(os.environ["WINDIR"]) / "Microsoft.NET/Framework64/v4.0.30319/csc.exe"
    if not compiler.is_file():
        raise RuntimeError("The Windows .NET Framework C# compiler is required.")
    packages = {name: package(name, version) for name, version in PACKAGES.items()}
    source = ROOT / "apps/scrying-cef/ScryingPane.cs"
    fingerprint = hashlib.sha256(source.read_bytes()).hexdigest()[:12]
    output = ROOT / "build/scrying-cef" / (VERSION + "-" + fingerprint)
    output.mkdir(parents=True, exist_ok=True)
    common = packages["cefsharp.common"]
    for folder in [common / "lib/net462", common / "CefSharp/x64", packages["cefsharp.winforms"] / "lib/net462"]:
        for file in folder.iterdir():
            if file.suffix.lower() in (".dll", ".exe", ".config"):
                shutil.copy2(file, output / file.name)
    runtime = packages["chromiumembeddedframework.runtime.win-x64"]
    native = next(runtime.rglob("libcef.dll")).parent
    for file in native.iterdir():
        if file.is_file() and file.suffix.lower() not in (".pdb", ".lib", ".exp"):
            shutil.copy2(file, output / file.name)
    for name in ["icudtl.dat", "resources.pak", "chrome_100_percent.pak", "chrome_200_percent.pak", "v8_context_snapshot.bin"]:
        candidates = list(runtime.rglob(name))
        if candidates:
            shutil.copy2(candidates[0], output / name)
    locale = next(runtime.rglob("en-US.pak"))
    (output / "locales").mkdir(exist_ok=True)
    shutil.copy2(locale, output / "locales/en-US.pak")
    info = list(packages["system.runtime.interopservices.runtimeinformation"].rglob("System.Runtime.InteropServices.RuntimeInformation.dll"))
    chosen = next((p for p in info if "runtimes/win/lib" in p.as_posix()), None) or next((p for p in info if "/lib/net45/" in p.as_posix()), None)
    if chosen:
        shutil.copy2(chosen, output / chosen.name)
    notices = output / "licenses"
    notices.mkdir(exist_ok=True)
    for name, directory in packages.items():
        for file in directory.rglob("*"):
            if file.is_file() and (file.name.lower().startswith("license") or file.name.lower() in ("credits.html", "readme.txt")):
                shutil.copy2(file, notices / (name + "-" + file.name))
    command = [str(compiler), "/nologo", "/target:winexe", "/platform:x64", "/optimize+", "/out:" + str(output / "ScryingPane.exe"), "/r:System.Windows.Forms.dll", "/r:System.Drawing.dll", "/r:System.Web.Extensions.dll"]
    command += ["/r:" + str(output / name) for name in ["CefSharp.dll", "CefSharp.Core.dll", "CefSharp.WinForms.dll"]]
    subprocess.run(command + [str(source)], check=True)
    config = '<?xml version="1.0" encoding="utf-8"?><configuration><startup useLegacyV2RuntimeActivationPolicy="true"><supportedRuntime version="v4.0" sku=".NETFramework,Version=v4.6.2"/></startup></configuration>'
    (output / "ScryingPane.exe.config").write_text(config, encoding="utf-8")
    for required in ["ScryingPane.exe", "CefSharp.dll", "CefSharp.Core.Runtime.dll", "CefSharp.BrowserSubprocess.exe", "libcef.dll", "icudtl.dat", "resources.pak", "locales/en-US.pak"]:
        if not (output / required).is_file():
            raise RuntimeError("CEF setup is incomplete: " + required)
    (output.parent / "current.json").write_text(json.dumps({"directory": output.name, "cefSharpVersion": VERSION, "runtimeVersion": PACKAGES["chromiumembeddedframework.runtime.win-x64"]}, indent=2), encoding="utf-8")
    print("Scrying Pane is ready. Open the updated desktop app to browse.", flush=True)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print("Scrying Pane setup could not finish: " + str(error), file=sys.stderr, flush=True)
        sys.exit(1)
