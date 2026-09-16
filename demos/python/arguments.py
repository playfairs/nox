import sys

print(f"received {len(sys.argv) - 1} arguments")
for index, argument in enumerate(sys.argv[1:], start=1):
    print(f"[{index}] {argument}")
