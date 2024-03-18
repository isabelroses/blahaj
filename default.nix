{buildNpmPackage}:
buildNpmPackage {
  pname = "blahaj";
  version = "0.1.0";

  src = ./.;

  dontNpmBuild = true;

  npmDepsHash = "sha256-mRmu2UIJTWj4d/UypUAM4+3Q8cbuVpazMuv4b21Yxho=";
}
