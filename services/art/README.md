# Local token artwork

`worker.py` uses a local SD-Turbo diffusion model to draw 512×512 sprites from descriptions. The desktop owns a single Python worker over bounded JSON-line pipes. No image server, account, or cloud API is used during play. Loading and generation run outside the UI thread; Stop kills the worker and leaves campaign data untouched. Successful images remain previews until Save PNG + SVG is selected.

Open `Setup-Local-Art.cmd` once from the repository folder. It uses the existing image-capable Python environment, downloads FP16 safetensors at revision `b261bac6fd2cf515557d5d0707481eafa0485ec2`, and keeps the model card and license with the weights. The runtime requires this completed setup marker and loads with `local_files_only=True`, `HF_HUB_OFFLINE=1` and `TRANSFORMERS_OFFLINE=1`. Application builds do not include or redistribute model weights.

The default Python path is `%USERPROFILE%\.unsloth\studio\unsloth_studio\Scripts\python.exe`. For another installation, set `DWA_ART_PYTHON_PATH` to a Python environment containing torch, diffusers, transformers, accelerate, Pillow, safetensors, and huggingface_hub. Set `DWA_ART_MODEL_DIR` for a different model folder; by default it is `%LOCALAPPDATA%\DungeonWorldAI\models\sd-turbo`.

Generation uses the model's one-step, guidance-zero recipe and a CPU generator seed. Weights stay warm in RAM between requests; GPU memory is released for the DM after generation. CPU inference is supported but slower. The optional cutout removes near-white pixels connected to the image edge; it is approximate and can be disabled. Frame and label composition happen separately, so changing them requires no new model inference. The self-contained SVG embeds the generated PNG; it does not vectorize the illustration. Saved recipes retain the requested prompt, seed, model revision, and cutout setting. Exact output can vary with hardware and library versions.

Model sources and separate terms: [SD-Turbo model card](https://huggingface.co/stabilityai/sd-turbo) and [model license](https://huggingface.co/stabilityai/sd-turbo/blob/main/LICENSE.md). Commercial deployment requires checking the applicable model terms separately from the application's license.

Diagnostics: `<DWA_DATA_DIR>/logs/art-runtime.log`. The embedded worker source is extracted to the same local log directory. Run `python -I services/art/test_worker.py` for cutout tests. The image weights are required for a real generation acceptance test.
