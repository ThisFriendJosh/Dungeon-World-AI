"""App-owned, offline SD-Turbo worker. JSON lines on stdin/stdout; diagnostics on stderr."""
import base64
import contextlib
import io
import json
import os
import sys
import time
from collections import deque

os.environ.update(HF_HUB_OFFLINE="1", TRANSFORMERS_OFFLINE="1", HF_HUB_DISABLE_TELEMETRY="1", DO_NOT_TRACK="1")
MODEL = "stabilityai/sd-turbo"
REVISION = "b261bac6fd2cf515557d5d0707481eafa0485ec2"


def emit(value):
    sys.stdout.write(json.dumps(value, ensure_ascii=True) + "\n")
    sys.stdout.flush()


def cutout(image):
    """Remove near-white pixels connected to an edge; enclosed white details stay intact."""
    image = image.convert("RGBA")
    width, height = image.size
    pixels = image.load()
    seen = bytearray(width * height)
    queue = deque([(x, 0) for x in range(width)] + [(x, height - 1) for x in range(width)] + [(0, y) for y in range(height)] + [(width - 1, y) for y in range(height)])
    while queue:
        x, y = queue.popleft()
        index = y * width + x
        if seen[index]:
            continue
        seen[index] = 1
        r, g, b, a = pixels[x, y]
        if min(r, g, b) < 230 or max(r, g, b) - min(r, g, b) > 24:
            continue
        pixels[x, y] = (r, g, b, 0)
        for nx, ny in ((x-1,y),(x+1,y),(x,y-1),(x,y+1)):
            if 0 <= nx < width and 0 <= ny < height and not seen[ny*width+nx]:
                queue.append((nx, ny))
    return image


def main():
    model_dir = sys.argv[1]
    pipe = None
    for line in sys.stdin:
        try:
            if len(line) > 10000:
                raise ValueError("The image request is too long.")
            request = json.loads(line)
            prompt = request["prompt"].strip()
            seed = request["seed"]
            transparent = request["transparentBackground"]
            if not prompt or len(prompt) > 1500 or type(seed) is not int or not 0 <= seed <= 0xffffffff or type(transparent) is not bool:
                raise ValueError("Use a prompt up to 1500 characters and a valid seed.")
            started = time.monotonic()
            with contextlib.redirect_stdout(sys.stderr):
                import torch
                from diffusers import AutoPipelineForText2Image
                device = "cuda" if torch.cuda.is_available() else "cpu"
                if pipe is None:
                    emit_progress = True
                else:
                    emit_progress = False
            if emit_progress:
                emit({"type":"progress", "message":"Loading the local image model…"})
                with contextlib.redirect_stdout(sys.stderr):
                    pipe = AutoPipelineForText2Image.from_pretrained(model_dir, torch_dtype=torch.float16 if device == "cuda" else torch.float32, variant="fp16", use_safetensors=True, local_files_only=True)
                    pipe.set_progress_bar_config(disable=True)
            emit({"type":"progress", "message":"Drawing your sprite on " + ("the GPU…" if device == "cuda" else "the CPU (this may take longer)…")})
            full_prompt = "A single " + prompt + ", fantasy tabletop game sprite, entire subject centered, simple clear silhouette, hand painted game art, isolated on a plain white background, no letters, no border"
            with contextlib.redirect_stdout(sys.stderr), torch.inference_mode():
                pipe.to(device)
                generator = torch.Generator(device="cpu").manual_seed(seed)
                output = pipe(prompt=full_prompt, num_inference_steps=1, guidance_scale=0.0, height=512, width=512, generator=generator)
                if getattr(output, "nsfw_content_detected", None) and any(output.nsfw_content_detected):
                    raise ValueError("Try another description for this sprite.")
                image = output.images[0]
                # Leave GPU memory available for the DM between requests; weights remain warm in RAM.
                pipe.to("cpu")
                if device == "cuda":
                    torch.cuda.empty_cache()
            if transparent:
                image = cutout(image)
            buffer = io.BytesIO()
            image.save(buffer, format="PNG")
            emit({"type":"result", "pngDataUrl":"data:image/png;base64," + base64.b64encode(buffer.getvalue()).decode("ascii"), "recipe":{"prompt":prompt,"seed":seed,"model":MODEL,"revision":REVISION,"transparentBackground":transparent}, "seconds":round(time.monotonic()-started, 2)})
        except Exception as exc:
            import traceback
            traceback.print_exc(file=sys.stderr)
            emit({"type":"error", "message":str(exc)[:1500]})
            return


if __name__ == "__main__":
    main()
