{
  clippy,
  rustfmt,
  callPackage,
  rust-analyzer,
  kittysay,
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

    # runtime things for testing
    kittysay
  ] ++ (oa.nativeBuildInputs or [ ]);
})
