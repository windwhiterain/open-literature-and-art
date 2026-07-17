import os, subprocess
from build import generate

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

if __name__ == "__main__":
    generate()
    subprocess.run(["zola", "serve"], cwd=ROOT)
