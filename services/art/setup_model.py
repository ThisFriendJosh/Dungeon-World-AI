"""One-time explicit download. Generation itself never downloads files."""
import json
import sys
from pathlib import Path
from huggingface_hub import snapshot_download

MODEL = "stabilityai/sd-turbo"
REVISION = "b261bac6fd2cf515557d5d0707481eafa0485ec2"
destination = Path(sys.argv[1]).resolve()
print("Downloading SD-Turbo's FP16 weights (about 1.7 GB). Keep this window open.", flush=True)
snapshot_download(repo_id=MODEL, revision=REVISION, local_dir=str(destination), allow_patterns=["model_index.json", "scheduler/*.json", "tokenizer/*", "text_encoder/config.json", "text_encoder/*.fp16.safetensors", "unet/config.json", "unet/*.fp16.safetensors", "vae/config.json", "vae/*.fp16.safetensors", "LICENSE.md", "README.md"])
required = ["model_index.json", "scheduler/scheduler_config.json", "tokenizer/tokenizer_config.json", "tokenizer/vocab.json", "tokenizer/merges.txt", "text_encoder/model.fp16.safetensors", "unet/diffusion_pytorch_model.fp16.safetensors", "vae/diffusion_pytorch_model.fp16.safetensors"]
for name in required:
    if not (destination / name).is_file():
        raise RuntimeError("The download is incomplete: " + name)
(destination / "dwa-model.json").write_text(json.dumps({"model":MODEL,"revision":REVISION,"format":"safetensors-fp16"}, indent=2), encoding="utf-8")
print("Local image model ready. Restart Dungeon World AI and open Token workshop.", flush=True)
