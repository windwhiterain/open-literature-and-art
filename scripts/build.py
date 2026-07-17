import os, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

def generate():
    content_dir = os.path.join(ROOT, "content")
    for dirpath, dirnames, filenames in os.walk(content_dir):
        if "meta.toml" not in filenames or "body.md" not in filenames:
            continue
        meta = os.path.join(dirpath, "meta.toml")
        body = os.path.join(dirpath, "body.md")
        index = os.path.join(dirpath, "index.md")

        src_mtime = max(os.path.getmtime(meta), os.path.getmtime(body))
        if os.path.exists(index) and os.path.getmtime(index) >= src_mtime:
            continue

        with open(index, "w", encoding="utf-8") as out:
            out.write("+++\n")
            out.write(open(meta, encoding="utf-8").read())
            out.write("+++\n")
            out.write(open(body, encoding="utf-8").read())
        print(f"Generated: {os.path.relpath(index, ROOT)}")

def main():
    generate()
    subprocess.run(["zola", "build"], cwd=ROOT, check=True)

if __name__ == "__main__":
    main()
