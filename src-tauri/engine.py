import sys
from markitdown import MarkItDown

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8")


def main():
    if len(sys.argv) < 2:
        print("Error: missing file path argument", file=sys.stderr)
        sys.exit(1)

    file_path = sys.argv[1]
    md = MarkItDown()
    result = md.convert(file_path)
    print(result.text_content)


if __name__ == "__main__":
    main()
