# Whale npm releases

Use the staging helper from `third_party/codex-cli/` to generate Whale npm
tarballs. Native packages require an explicitly approved Whale workflow run:

```bash
python3 scripts/stage_npm_packages.py \
  --release-version 0.0.5 \
  --package whalecode \
  --workflow-url https://github.com/ceasarXuu/WhaleCode/actions/runs/<approved-run-id>
```

This downloads the required native package archive artifacts, hydrates `vendor/` for
each package, and writes tarballs to `dist/npm/`.

The helper builds the lightweight `@ceasarxuu/whalecode` meta package plus six
platform-native aliases published as platform-suffixed versions of the same
scoped package. It does not publish the upstream SDK or responses proxy.

Direct `build_npm_package.py` invocations are still useful for package-specific
debugging, but native packages expect `--vendor-src` to point at a prehydrated
`vendor/` tree. Release packaging should use `scripts/stage_npm_packages.py`.

The workflows and package manifests retained elsewhere in the vendor snapshot
are upstream provenance and are quarantined by `DISTRIBUTION_QUARANTINE.md`.
