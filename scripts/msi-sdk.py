"""Assemble declarative MSI tables with Microsoft's signed Windows SDK tools."""
import csv
import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import uuid

ROOT = Path(__file__).resolve().parent.parent
TEMPLATES = ROOT / "apps/beta-setup/msi"


def load_tables():
    return {p.stem: list(csv.reader(p.read_text(encoding="cp1252").splitlines(), delimiter="\t"))
            for p in TEMPLATES.glob("*.idt")}


def prepare_tables(pack):
    tables = load_tables()
    dirs = {}

    def directory(path):
        if str(path) in ("", "."):
            return "INSTALLDIR"
        text = path.as_posix()
        if text in dirs:
            return dirs[text]
        key = "D" + hashlib.md5(text.lower().encode()).hexdigest().upper()
        parent = directory(path.parent)
        dirs[text] = key
        tables["Directory"].append([key, parent, "D%06d|%s" % (len(dirs), path.name)])
        return key

    helper = None
    names = set()
    for i, file in enumerate(pack["files"], 1):
        path = PurePosixPath(file["path"])
        if path.is_absolute() or ":" in str(path) or ".." in path.parts or str(path).lower() in names:
            raise ValueError("Invalid or duplicate MSI payload path")
        names.add(str(path).lower())
        key = "F%05d" % i
        component = "C" + hashlib.md5(str(path).lower().encode()).hexdigest().upper()
        guid = "{" + str(uuid.uuid5(uuid.NAMESPACE_URL, "ai.dungeonworld.beta:component:" + str(path).lower())).upper() + "}"
        tables["Component"].append([component, guid, directory(path.parent), "256", "", key])
        tables["FeatureComponents"].append(["Core", component])
        tables["File"].append([key, component, "F%06d|%s" % (i, path.name), str(file["size"]), "", "", "512", str(i)])
        if str(path) == "DungeonWorldAI.exe":
            helper = key
    if helper is None:
        raise ValueError("The local AI setup helper is missing")
    for row in tables["Property"][3:]:
        if row[0] == "ProductCode":
            row[1] = pack["productCode"]
        elif row[0] == "ProductVersion":
            row[1] = pack["version"].split("-")[0]
    for row in tables["Upgrade"][3:]:
        row[tables["Upgrade"][0].index("VersionMax")] = pack["version"].split("-")[0]
    for row in tables["CustomAction"][3:]:
        if row[0] == "LaunchAI":
            row[2] = helper
    for row in tables["_SummaryInformation"][3:]:
        if row[0] == "9":
            row[1] = "{" + str(uuid.uuid4()).upper() + "}"
    tables["Media"].append(["1", str(len(pack["files"])), "", "#payload.cab", "", ""])
    # Shortcuts use their target EXE's embedded icon; no MSI binary icon table is needed.
    for row in tables["Shortcut"][3:]:
        row[tables["Shortcut"][0].index("Icon_")] = ""
        row[tables["Shortcut"][0].index("IconIndex")] = ""
    # A registry-key component must refer to its Registry row, including its desktop condition.
    for row in tables["Component"][3:]:
        if row[0] in ("AppLinks", "DesktopLink"):
            row[4] = "CREATE_DESKTOP = 1" if row[0] == "DesktopLink" else ""
            row[5] = row[0] + "Reg"
    return tables


def build(manifest, output):
    pack = json.loads(manifest.read_text(encoding="utf-8"))
    if output.exists():
        raise ValueError("Use a new output directory; existing installers are never overwritten")
    sdk = Path("C:/Program Files (x86)/Windows Kits/10/bin")
    candidates = sorted(sdk.glob("*/x86/MsiDb.exe"), reverse=True)
    if not candidates:
        raise RuntimeError("Install the Windows SDK's Windows Installer development tools on the build PC")
    msidb = candidates[0]
    idt = output.parent / "msi-tables"
    idt.mkdir(parents=True, exist_ok=True)
    for i, (name, rows) in enumerate(sorted(prepare_tables(pack).items()), 1):
        # MsiDb's import command accepts eight-character archive basenames.
        with (idt / ("T%07d.idt" % i)).open("w", encoding="cp1252", newline="") as target:
            csv.writer(target, delimiter="\t", lineterminator="\r\n").writerows(rows)
    lines = [".OPTION EXPLICIT", ".Set CabinetNameTemplate=payload.cab",
             '.Set DiskDirectoryTemplate="' + str(output.parent) + '"',
             ".Set CompressionType=LZX", ".Set CompressionMemory=21", ".Set Cabinet=on",
             ".Set Compress=on", ".Set MaxDiskSize=0"]
    for i, file in enumerate(pack["files"], 1):
        source = Path(file["source"])
        with source.open("rb") as stream:
            if hashlib.file_digest(stream, "sha256").hexdigest() != file["sha256"]:
                raise ValueError("A payload file changed during the build: " + file["path"])
        lines.append('"' + str(source) + '" F%05d' % i)
    ddf = output.parent / "payload.ddf"
    ddf.write_text("\r\n".join(lines) + "\r\n", encoding="cp1252")
    print("Packing %d verified app/runtime files with Windows MakeCab..." % len(pack["files"]), flush=True)
    result = subprocess.run([str(Path("C:/Windows/System32/makecab.exe")), "/F", str(ddf)], capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stdout.decode(errors="replace") + result.stderr.decode(errors="replace"))
    subprocess.run([str(msidb), "-d", str(output), "-f", str(idt), "-c", "-i", "*.idt"], check=True,
                   creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
    subprocess.run([str(msidb), "-d", str(output), "-a", "payload.cab"], cwd=output.parent, check=True,
                   creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
    print("Built standard Windows MSI with Microsoft SDK tools: " + str(output), flush=True)


if __name__ == "__main__":
    build(*(Path(s).resolve() for s in sys.argv[1:3]))
