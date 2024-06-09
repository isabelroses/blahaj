{ lib, rustPlatform }:
let
  p = (lib.importTOML ./Cargo.toml).package;
in
rustPlatform.buildRustPackage {
  pname = "blahaj";
  inherit (p) version;

  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;

  meta = {
    inherit (p) description homepage;
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [ isabelroses ];
    mainProgram = "blahaj";
  };
}
