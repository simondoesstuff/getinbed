# Release process

## Steps

1. Ensure `version` in `mix.exs` and `native/getinbed/Cargo.toml` match and are bumped.

2. Push a version tag:
   ```sh
   git tag v0.1.0 && git push --tags
   ```

3. The `Precompile NIFs` workflow triggers on the tag. It builds the native library for each target, uploads the `.tar.gz` archives to a GitHub release, then opens a PR updating `checksum-Elixir.GetInBed.exs` with the verified SHA256s.

4. Merge the checksums PR.

5. Publish to Hex:
   ```sh
   mix hex.publish
   ```

## Targets

| Target | Runner |
|---|---|
| `aarch64-apple-darwin` | macOS (native) |
| `x86_64-apple-darwin` | macOS (native) |
| `x86_64-unknown-linux-gnu` | Ubuntu (native) |
| `aarch64-unknown-linux-gnu` | Ubuntu via `cross` |

## Source builds

Users can opt out of the precompiled NIF and compile from source by setting `GETINBED_BUILD=1` (or `true`) before `mix deps.compile`. This requires a Rust toolchain.

musl targets (`x86_64-unknown-linux-musl`, Alpine Linux) are not offered as precompiled NIFs — Rust's cdylib output is not supported for musl. musl users must build from source.
