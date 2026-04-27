# Whale npm Publishing Runbook

Date: 2026-04-28

This runbook covers the Whale CLI npm package. It intentionally uses the scoped
package name `@ceasarxuu/whalecode` because npm rejects the unscoped
`whalecode` name as too similar to the existing `whale-code` package, and the
unscoped `whale` package is already taken.

## Package Boundary

- npm package: `@ceasarxuu/whalecode`
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

For automation, keep the npm token outside git. This repo's `.env.local` file is
ignored and may contain `npm_access_token=...`. Load it into a process-local
temporary npm config instead of writing to the user-level `.npmrc`:

```powershell
$Line = Get-Content D:\WhaleCode\.env.local -Encoding UTF8 |
  Where-Object { $_ -match '^npm_access_token=' } |
  Select-Object -First 1
$Token = $Line -replace '^npm_access_token=', ''
$NpmUserConfig = 'D:\BuildCache\whalecode\npm-auth\.npmrc'
New-Item -ItemType Directory -Force (Split-Path $NpmUserConfig) | Out-Null
"//registry.npmjs.org/:_authToken=$Token" |
  Set-Content -Path $NpmUserConfig -Encoding ASCII -NoNewline
npm --userconfig $NpmUserConfig whoami
```

After publishing, overwrite and remove the temporary npm config:

```powershell
Set-Content -Path $NpmUserConfig -Value 'removed' -Encoding ASCII
Remove-Item -LiteralPath $NpmUserConfig
```

Check package availability before the first public publish:

```powershell
npm view @ceasarxuu/whalecode name version --json
```

Expected first-time result is npm `E404`. If the package exists and is not
owned by you or the project, stop and choose a different Whale-owned package
name before publishing.

## Local Dry Run

The meta package can be packed without native binaries. Use an empty staging
directory. `--package whalecode` is the local builder selector; the package name
inside the staged `package.json` is still `@ceasarxuu/whalecode`.

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
npm publish .\dist\npm\whalecode-npm-win32-x64-0.0.0-dev.tgz --tag dev-win32-x64 --access public
```

If npm returns `E403` with a two-factor-authentication message, rerun with
`--otp <current-code>` or use a publish token that is allowed to bypass 2FA.
If npm returns `E403` saying the package name is too similar to another package,
use the scoped package name and include `--access public`.

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
npm publish .\dist\npm\whalecode-npm-linux-x64-0.1.0.tgz --tag linux-x64 --access public
npm publish .\dist\npm\whalecode-npm-linux-arm64-0.1.0.tgz --tag linux-arm64 --access public
npm publish .\dist\npm\whalecode-npm-darwin-x64-0.1.0.tgz --tag darwin-x64 --access public
npm publish .\dist\npm\whalecode-npm-darwin-arm64-0.1.0.tgz --tag darwin-arm64 --access public
npm publish .\dist\npm\whalecode-npm-win32-x64-0.1.0.tgz --tag win32-x64 --access public
npm publish .\dist\npm\whalecode-npm-win32-arm64-0.1.0.tgz --tag win32-arm64 --access public
npm publish .\dist\npm\whalecode-npm-0.1.0.tgz --access public
```

For alpha releases, add an alpha dist-tag to the root publish and prefix the
platform dist-tags consistently, for example `--tag alpha` for the root package
and `--tag alpha-win32-x64` for the Windows x64 platform tarball.

Because `@ceasarxuu/whalecode` is scoped, public publishes must include
`--access public`.

## 2026-04-28 Dev Publish Record

The unscoped `whalecode` publish probe reached npm auth successfully, but npm
rejected the package identity:

```text
E403 Package name too similar to existing package whale-code.
```

The first usable package identity is therefore `@ceasarxuu/whalecode`. The
published dev versions are:

```text
@ceasarxuu/whalecode@0.0.1-dev
@ceasarxuu/whalecode@0.0.1-dev-win32-x64
latest -> 0.0.1-dev
dev-win32-x64 -> 0.0.1-dev-win32-x64
```

The earlier probe versions `0.0.0-dev` and `0.0.0-dev-win32-x64` were
deprecated because they were staged from an older `release\whale.exe` that still
exposed stale model catalog output. Keep using `0.0.1-dev` or newer.

The isolated install smoke passed from a fresh prefix:

```powershell
npm install -g @ceasarxuu/whalecode@latest --include=optional `
  --prefix D:\BuildCache\whalecode\npm-smoke\install-20260428011837-001
whale --version
whale debug models
```

Expected result: `whale debug models` lists only `deepseek-v4-pro` and
`deepseek-v4-flash`. No GPT, ChatGPT, OpenAI, or Codex model names should appear
in the picker.

## Post-Publish Smoke

Use a fresh terminal after publication:

```powershell
npm install -g @ceasarxuu/whalecode@latest --include=optional
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
