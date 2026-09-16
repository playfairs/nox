import sys

code = int(sys.argv[1]) if len(sys.argv) > 1 else 0
print(f"exiting with status {code}")
sys.exit(code)
