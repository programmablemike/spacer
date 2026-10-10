{ lib, stdenv, rustPlatform, cargo-tauri, pkg-config, makeBinaryWrapper
, git, openssl, webkitgtk_4_1, gtk3, librsvg, wrapGAppsHook3, xdg-utils
, callPackage, nativeSmoke ? false
}:

let
  frontend = callPackage ./frontend.nix { };
in
rustPlatform.buildRustPackage {
  pname = "spacer-desktop" + lib.optionalString nativeSmoke "-smoke";
  version = (lib.importTOML ../desktop/src-tauri/Cargo.toml).package.version;
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../desktop/src-tauri
      ../desktop/tests/native-smoke.js
      ../spacer-core
      ../spacer-data
    ];
  };
  cargoLock.lockFile = ../desktop/src-tauri/Cargo.lock;
  cargoRoot = "desktop/src-tauri";
  buildAndTestSubdir = "desktop/src-tauri";
  nativeBuildInputs = [ cargo-tauri.hook pkg-config makeBinaryWrapper ]
    ++ lib.optionals stdenv.hostPlatform.isLinux [ wrapGAppsHook3 ];
  buildInputs = lib.optionals stdenv.hostPlatform.isLinux [
    openssl webkitgtk_4_1 gtk3 librsvg
  ];
  nativeCheckInputs = [ git ];
  buildFeatures = lib.optional nativeSmoke "native-smoke";
  # The frontend is built in its own sandboxed derivation. Signing credentials
  # and conventional installer generation do not belong in this Nix package.
  postPatch = ''
    substituteInPlace desktop/src-tauri/tauri.conf.json \
      --replace-fail '"beforeBuildCommand": "npm run build"' '"beforeBuildCommand": ""' \
      --replace-fail '"frontendDist": "../dist"' '"frontendDist": "${frontend}"'
  '';
  postInstall = lib.optionalString stdenv.hostPlatform.isDarwin ''
    wrapProgram "$out/Applications/Spacer.app/Contents/MacOS/spacer-desktop" \
      --prefix PATH : ${lib.makeBinPath [ git ]}
    mkdir -p "$out/bin"
    ln -s ../Applications/Spacer.app/Contents/MacOS/spacer-desktop "$out/bin/spacer-desktop"
  '';
  preFixup = lib.optionalString stdenv.hostPlatform.isLinux ''
    gappsWrapperArgs+=(--prefix PATH : ${lib.makeBinPath [ git xdg-utils ]})
  '';
  meta = {
    description = "Graphical workspace and Git worktree manager";
    homepage = "https://github.com/programmablemike/spacer";
    license = lib.licenses.mit;
    mainProgram = "spacer-desktop";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
