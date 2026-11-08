"""Explicit, GUI-owned optional setup. The art worker never downloads dependencies."""
import importlib.metadata
from collections import deque
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

REVISION = "b261bac6fd2cf515557d5d0707481eafa0485ec2"
PACKAGES = ["diffusers==0.42.1", "transformers==5.5.0", "accelerate==1.14.0",
            "Pillow==13.0.0", "safetensors==0.8.0", "huggingface_hub==1.33.0",
            "PyMuPDF==1.27.2.3"]


class SetupError(RuntimeError):
    pass


def require_virtual_environment():
    if sys.prefix == sys.base_prefix:
        raise SetupError("The private AI art environment is not initialized. Install the updated beta and run Set up local AI again.")


def progress(text):
    print("DWA_PROGRESS " + text, flush=True)


def torch_channel():
    """Use a CPU wheel unless an installed NVIDIA driver supports CUDA 13."""
    tool = shutil.which("nvidia-smi")
    if not tool:
        candidate = Path(os.environ.get("WINDIR", "C:/Windows")) / "System32/nvidia-smi.exe"
        tool = str(candidate) if candidate.is_file() else None
    if tool:
        try:
            result = subprocess.run([tool, "--query-gpu=driver_version", "--format=csv,noheader"],
                                    capture_output=True, text=True, timeout=15,
                                    creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
            if result.returncode == 0 and any(int(s.strip().split(".")[0]) >= 580 for s in result.stdout.splitlines() if s.strip()):
                return "cu130"
        except (OSError, ValueError, subprocess.TimeoutExpired):
            pass
    return "cpu"


def pip(*arguments):
    # Only binary wheels: a beta tester never needs compilers or a build toolchain.
    require_virtual_environment()
    command = [sys.executable, "-I", "-X", "utf8", "-u", "-m", "pip", "--isolated", "--no-input", "install",
               "--disable-pip-version-check", "--only-binary=:all:", "--retries", "3", "--timeout", "60", *arguments]
    tail = deque(maxlen=30)
    # Explicit pipes keep subprocess diagnostics available when the GUI has no console.
    with subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT, text=True, encoding="utf-8", errors="replace",
                          creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0)) as process:
        for line in process.stdout:
            print(line.rstrip(), flush=True)
            tail.append(line.strip())
        code = process.wait()
    if code:
        errors = [line for line in tail if line.startswith("ERROR:") or "externally-managed" in line]
        reason = (errors[-1] if errors else next((line for line in reversed(tail) if line), "Package installer exited with code " + str(code)))
        raise SetupError("AI art package setup failed: " + reason[:300] + " See the setup log for details.")


def packages_ready():
    expected = {"torch": "2.13.0", "torchvision": "0.28.0", **dict(s.split("==") for s in PACKAGES)}
    # A separate process releases Windows DLL locks before pip repairs any packages.
    probe = ("import importlib.metadata as m,sys; "
             "import torch,torchvision,diffusers,transformers,accelerate,PIL,safetensors,huggingface_hub,pymupdf; "
             "expected=" + repr(expected) + "; "
             "sys.exit(0 if all(m.version(k).split('+')[0]==v for k,v in expected.items()) else 1)")
    result = subprocess.run([sys.executable, "-I", "-X", "utf8", "-c", probe],
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                            encoding="utf-8", errors="replace",
                            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
    return result.returncode == 0


def main():
    require_virtual_environment()
    destination = Path(sys.argv[1]).resolve()
    if not packages_ready():
        channel = torch_channel()
        progress("Downloading the image engine (NVIDIA GPU). This can take several minutes." if channel != "cpu"
                 else "Downloading the image engine (CPU). This can take several minutes.")
        pip("torch==2.13.0", "torchvision==0.28.0", "--index-url", "https://download.pytorch.org/whl/" + channel)
        progress("Preparing image generation and document imports...")
        pip(*PACKAGES, "--index-url", "https://pypi.org/simple")
        # Fresh interpreter verifies newly installed native DLLs and package imports.
        subprocess.run([sys.executable, "-I", "-c",
                        "import torch, torchvision, diffusers, transformers, accelerate, PIL, safetensors, huggingface_hub, pymupdf; print('Image engine verified')"],
                       stdout=sys.stdout, stderr=sys.stderr, check=True,
                       creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
    from huggingface_hub import snapshot_download
    progress("Downloading SD-Turbo image weights (about 3 GB). Existing downloads are reused.")
    destination.mkdir(parents=True, exist_ok=True)
    snapshot_download(repo_id="stabilityai/sd-turbo", revision=REVISION, local_dir=str(destination),
                      allow_patterns=["model_index.json", "scheduler/*.json", "tokenizer/*",
                                      "text_encoder/config.json", "text_encoder/*.fp16.safetensors",
                                      "unet/config.json", "unet/*.fp16.safetensors",
                                      "vae/config.json", "vae/*.fp16.safetensors", "LICENSE.md", "README.md"])
    required = ["model_index.json", "scheduler/scheduler_config.json", "tokenizer/tokenizer_config.json",
                "tokenizer/vocab.json", "tokenizer/merges.txt", "text_encoder/model.fp16.safetensors",
                "unet/diffusion_pytorch_model.fp16.safetensors", "vae/diffusion_pytorch_model.fp16.safetensors"]
    for name in required:
        if not (destination / name).is_file():
            raise RuntimeError("The image download is incomplete: " + name)
    (destination / "dwa-model.json").write_text(json.dumps({"model": "stabilityai/sd-turbo", "revision": REVISION,
                                                         "format": "safetensors-fp16"}, indent=2), encoding="utf-8")
    progress("Local AI art is ready. Restart the app to use newly added features.")


if __name__ == "__main__":
    try:
        main()
    except SetupError as error:
        print("DWA_ERROR " + str(error), flush=True)
        sys.exit(1)
