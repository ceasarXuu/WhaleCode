# Whale npm Publishing Runbook

Date: 2026-04-27

This runbook covers the Whale CLI npm package. It intentionally uses the
unscoped package name `whalecode` because the unscoped `whale` package is
already taken on npm and a scoped package such as `@whalecode/whale` requires
control of that npm scope.

## Package Boundary

- npm package: `whalecode`
- installed command: `whale`
- launcher: `third_party/codex-cli/codex-cli/bin/whale.js`
- native binary path inside npm tarballs: `vendor/<target>/whale/whale(.exe)`
- forbidden package identity: `@openai/codex`
- forbidden npm command exposure: `codex`

Run this guard after any npm packaging edit:

```powershell
Set-Location D:\WhaleCode
.\scripts\check-codex-collision-risk.ps1
```

## Account Setup

Use your normal npm account locally:

```powershell
npm login
npm whoami
```

If the account has publish-time 2FA enabled, `npm publish` requires either a
current one-time password or a granular access token with bypass 2FA enabled:

```powershell
npm publish $Tarball --tag dev-win32-x64 --otp 123456
```

Check package availability before the first public publish:

```powershell
npm view whalecode name version --json
```

Expected first-time result is npm `E404`. If the package exists and is not
owned by you or the project, stop and choose a different Whale-owned package
name before publishing.

## Local Dry Run

The meta package can be packed without native binaries. Use an empty staging
directory:

```powershell
Set-Location D:\WhaleCode\third_party\codex-cli
$Version = "0.0.0-dev"
$Stage = "D:\BuildCache\whalecode\npm-stage\whalecode-meta-$Version"
$Tarball = "D:\BuildCache\whalecode\npm\whalecode-$Version.tgz"
python .\codex-cli\scripts\build_npm_package.py `
  --package whalecode `
  --version $Version `
  --staging-dir $Stage `
  --pack-output $Tarball
npm publish $Tarball --dry-run --tag dev
```

The explicit `--tag dev` keeps prerelease dry-runs away from the default
`latest` dist-tag.

Do not run `npm install -g .` from this repository as a normal development
install path. Use `scripts/install-whale-local.ps1` for local builds so npm
global state and official Codex stay isolated.

## First Publish Probe

To prove npm ownership without moving the root package's `latest` tag, publish a
single platform dev package first:

```powershell
npm publish .\dist\npm\whalecode-npm-win32-x64-0.0.0-dev.tgz --tag dev-win32-x64
```

If npm returns `E403` with a two-factor-authentication message, rerun with
`--otp <current-code>` or use a publish token that is allowed to bypass 2FA.

## Release Tarballs

For a real release, stage the CLI package from the repository root of the
vendored CLI checkout after the Rust release workflow has produced native
artifacts:

```powershell
Set-Location D:\WhaleCode\third_party\codex-cli
python .\scripts\stage_npm_packages.py `
  --release-version 0.1.0 `
  --package whalecode `
  --output-dir .\dist\npm `
  --keep-staging-dirs
```

Publish platform tarballs before the root wrapper. The root wrapper references
platform-specific optional dependency versions, so those versions must exist
first:

```powershell
npm publish .\dist\npm\whalecode-npm-linux-x64-0.1.0.tgz --tag linux-x64
npm publish .\dist\npm\whalecode-npm-linux-arm64-0.1.0.tgz --tag linux-arm64
npm publish .\dist\npm\whalecode-npm-darwin-x64-0.1.0.tgz --tag darwin-x64
npm publish .\dist\npm\whalecode-npm-darwin-arm64-0.1.0.tgz --tag darwin-arm64
npm publish .\dist\npm\whalecode-npm-win32-x64-0.1.0.tgz --tag win32-x64
npm publish .\dist\npm\whalecode-npm-win32-arm64-0.1.0.tgz --tag win32-arm64
npm publish .\dist\npm\whalecode-npm-0.1.0.tgz
```

For alpha releases, add an alpha dist-tag to the root publish and prefix the
platform dist-tags consistently, for example `--tag alpha` for the root package
and `--tag alpha-win32-x64` for the Windows x64 platform tarball.

Because `whalecode` is unscoped, do not add `--access public`. If the package is
changed later to a scoped name, the first public scoped publish must use
`--access public`.

## Post-Publish Smoke

Use a fresh terminal after publication:

```powershell
npm install -g whalecode@latest --include=optional
where.exe whale
whale --version
whale debug models
where.exe codex
codex --version
```

Expected result: `whale` resolves through npm only when deliberately installed,
and `codex` still resolves to official Codex rather than any Whale-managed path.

## References

- npm publish command: https://docs.npmjs.com/cli/v11/commands/npm-publish/
- unscoped public package flow: https://docs.npmjs.com/creating-and-publishing-unscoped-public-packages
- scoped public package flow: https://docs.npmjs.com/creating-and-publishing-scoped-public-packages
