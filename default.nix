{buildNpmPackage}:
buildNpmPackage {
  pname = "blahaj";
  version = "0.1.0";

  src = ./.;

  dontNpmBuild = true;

  npmDepsHash = "sha256-hjExjokjK3HZssWOkARDJY1m0+SxsQsxT2WaoBYqqe8=";
}
