self: {
  pkgs,
  config,
  lib,
  ...
}: let
  inherit (lib) mkIf mkEnableOption;
in {
  options.services.blahaj.enable = mkEnableOption "blahaj";

  config = mkIf config.services.blahaj.enable {
    systemd.services."blahaj" = {
      description = "blahaj";
      after = ["network.target"];
      wantedBy = ["multi-user.target"];
      path = [pkgs.nodejs];

      serviceConfig = {
        Type = "simple";
        DynamicUser = true;
        ExecStart = "node ${self.packages.${pkgs.stdenv.hostPlatform.system}.default}/lib/node_modules/blahaj";
        Restart = "always";
      };
    };
  };
}
