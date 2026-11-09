"""Build the portable Windows CPU runtime with per-machine instruction selection. No downloads."""
import argparse
from pathlib import Path
import shutil
import subprocess

FLAGS = {
    "GGML_NATIVE": "OFF", "GGML_BACKEND_DL": "ON", "GGML_CPU_ALL_VARIANTS": "ON",
    "GGML_CUDA": "OFF", "GGML_BLAS": "OFF", "GGML_OPENMP_FETCH": "OFF",
    "LLAMA_BUILD_TESTS": "OFF", "LLAMA_BUILD_EXAMPLES": "OFF", "LLAMA_BUILD_TOOLS": "ON",
    "LLAMA_BUILD_SERVER": "ON", "LLAMA_BUILD_APP": "OFF", "LLAMA_BUILD_UI": "OFF",
    "LLAMA_USE_PREBUILT_UI": "OFF", "LLAMA_OPENSSL": "OFF", "LLAMA_CONNECT": "OFF",
}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--cmake", default=shutil.which("cmake"))
    args = parser.parse_args()
    if not args.cmake:
        parser.error("Provide --cmake or install the Windows CMake build tools")
    subprocess.run([args.cmake, "-S", str(args.source.resolve()), "-B", str(args.output.resolve()),
                    "-G", "Visual Studio 17 2022", "-A", "x64", *["-D" + k + "=" + v for k, v in FLAGS.items()]], check=True)
    # A single MSBuild worker also runs in restricted maintainer environments.
    subprocess.run([args.cmake, "--build", str(args.output.resolve()), "--config", "Release",
                    "--target", "llama-server", "llama-completion", "-j", "1"], check=True)
    runtime = args.output.resolve() / "bin/Release"
    for name in ("llama-server.exe", "ggml-cpu-x64.dll", "ggml-cpu-haswell.dll"):
        if not (runtime / name).is_file():
            raise RuntimeError("The portable runtime is missing " + name)
    print("CPU runtime ready: " + str(runtime))

if __name__ == "__main__":
    main()
