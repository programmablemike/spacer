# Install with Nix

[Documentation](../README.md) / How-to

Spacer exposes source-built flake packages for macOS and Linux, on both ARM64
and x86-64. Nix builds the frontend and Rust executable from the repository's
locked inputs. It does not download a prebuilt GitHub release.

| Package | Contents |
| --- | --- |
| `cli` | `spacer`, including the terminal browser and Git runtime dependency |
| `desktop` | `spacer-desktop`, application resources, and runtime dependencies |
| `default` | Alias for `cli`, preserving existing `nix build` usage |

Enable flakes in your Nix installation. To try either presentation:

```sh
nix run github:programmablemike/spacer#cli -- --help
nix run github:programmablemike/spacer#desktop
```

For a persistent standalone installation:

```sh
nix profile install github:programmablemike/spacer#cli
nix profile install github:programmablemike/spacer#desktop
```

## Home Manager

Add Spacer to your configuration flake's inputs:

```nix
inputs.spacer.url = "github:programmablemike/spacer";
```

Pass `inputs` to your Home Manager modules with
`extraSpecialArgs = { inherit inputs; };`. For Home Manager embedded in NixOS or
nix-darwin, use `home-manager.extraSpecialArgs = { inherit inputs; };`.
Then add the packages in a Home Manager module:

```nix
{ inputs, pkgs, ... }:
let
  spacer = inputs.spacer.packages.${pkgs.stdenv.hostPlatform.system};
in {
  home.packages = [ spacer.cli spacer.desktop ];
}
```

Apply your configuration with your usual Home Manager rebuild command. Either
package can be installed independently. The CLI's shell navigation wrapper is
still configured separately; see [shell navigation](shell-navigation.md).

On macOS, the desktop provides `Applications/Spacer.app` in its package output.
Recent Home Manager versions can copy it into `~/Applications/Home Manager Apps`
for Finder and Spotlight integration. If your configuration still uses the older
symlink behavior, these options explicitly select copying:

```nix
targets.darwin.linkApps.enable = false;
targets.darwin.copyApps.enable = true;
```

These are macOS-only options. Home Manager may request App Management permission
when activating applications. See its [application module](https://github.com/nix-community/home-manager/blob/master/modules/targets/darwin/copyapps.nix).

## nix-darwin

Add the same flake input, pass `specialArgs = { inherit inputs; };` to
`nix-darwin.lib.darwinSystem`, and include this module:

```nix
{ inputs, pkgs, ... }:
let
  spacer = inputs.spacer.packages.${pkgs.stdenv.hostPlatform.system};
in {
  environment.systemPackages = [ spacer.cli spacer.desktop ];
}
```

Rebuild with your usual `darwin-rebuild switch --flake ...` command. nix-darwin
collects the app bundle into `/Applications/Nix Apps`. Choose either Home Manager
or nix-darwin to manage the desktop to avoid duplicate application entries.
See [nix-darwin's application integration](https://github.com/nix-darwin/nix-darwin/blob/master/modules/system/applications.nix).

## Updates and runtime behavior

Your configuration's `flake.lock` pins Spacer to a commit. Update that input with
`nix flake update spacer`, review the lockfile change, and rebuild. Your normal
generation rollback restores the previous package version; it does not roll back
workspace data or Git operations.

Spacer retains its own pinned nixpkgs input. Making it follow a different nixpkgs
revision changes its build dependencies and is outside the tested configuration.
There is no public project binary cache configured: Nix can reuse dependencies
from its usual cache, but may build Spacer locally. GitHub Actions caches its
Nix store to speed up CI; that cache is not a substituter for user installations.

The packages supply Git even when launched from Finder or a desktop menu.
Git identity and user configuration remain your responsibility. Spacer does not
set a commit author or create commits. Workspace data stays in the folder you
select, outside the Nix store, and remains shared with other Spacer installations.

On Linux the package provides a desktop entry, icon, and WebKitGTK dependencies.
It requires a graphical session. Non-NixOS installations may additionally need
their normal Nix GUI/graphics integration.

## Build and verify locally

```sh
nix build .#cli
nix build .#desktop --out-link result-desktop
nix flake check --print-build-logs
```

The Nix workflow builds and tests both packages on all four supported systems.
It also runs a separate test-only desktop through the real webview lifecycle.
The normal `desktop` package excludes that harness.

Nix packages depend on their Nix store closure. Copying only the executable or
app bundle to a machine without Nix is unsupported. Conventional DMG, Debian,
and Windows installers remain a separate native build pipeline. Developer ID
signing, Apple notarization, and Windows publisher signing are not configured;
Nix package builds do not require signing credentials.
