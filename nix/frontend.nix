{ lib, buildNpmPackage }:

buildNpmPackage {
  pname = "spacer-desktop-frontend";
  version = (lib.importJSON ../desktop/package.json).version;
  src = lib.fileset.toSource {
    root = ../desktop;
    fileset = lib.fileset.unions [
      ../desktop/package.json
      ../desktop/package-lock.json
      ../desktop/tsconfig.json
      ../desktop/index.html
      ../desktop/src
      ../desktop/public
    ];
  };
  npmDepsHash = "sha256-RQn7ueOrn1RgzNvYmwrwaAXtqLy7gjLpntKVOp8IDFI=";
  installPhase = ''
    runHook preInstall
    mkdir -p "$out"
    cp -r dist/. "$out/"
    runHook postInstall
  '';
}
