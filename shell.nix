{
  clippy,
  rustfmt,
  kittysay,
  cargo-shear,
  callPackage,
  rust-analyzer,
}:
let
  mainPkg = callPackage ./default.nix { };
in
mainPkg.overrideAttrs (oa: {
  nativeBuildInputs = [
    # Additional rust tooling
    clippy
    rustfmt
    rust-analyzer
    cargo-shear

    # runtime things for testing
    kittysay
  ] ++ (oa.nativeBuildInputs or [ ]);
})
