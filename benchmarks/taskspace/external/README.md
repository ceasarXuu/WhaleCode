# TaskSpace External Benchmark Intake

This directory tracks external benchmark sources used for E3 TaskSpace evidence.

Rules:

- Store source pointers, pinned revisions, sample ids, checksums, adapter versions, and license notes.
- Do not vendor full benchmark datasets into this repository.
- Do not store gold patches, solutions, hidden tests, secrets, or private user data here.
- External tasks must be materialized into temporary run directories before execution.
- Benchmark prompts must remain natural user instructions and must not mention TaskSpace internals.

Current priority:

1. DeepSWE adapter spike.
2. Terminal-Bench adapter spike.
3. SWE-bench Multilingual feasibility intake.

E3 results are not valid until an external sample has:

- pinned source metadata
- original instruction checksum
- validator checksum
- paired standard/taskspace artifacts
- external validator output
- artifact audit review
