from pathlib import Path

Path(".large_output_probe_ran").write_text("ran\n", encoding="utf-8")

for index in range(900):
    print(f"{index:04d} middle-secret-marker diagnostic payload for output reference smoke")
