{
  description = "ccp: snapshot, blueprint, and scaffold project directory trees";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      version = cargoToml.package.version or cargoToml.workspace.package.version;
      withClipboard = true;
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          ccp = pkgs.rustPlatform.buildRustPackage {
          pname = "ccp_tree";
          inherit version;
          
          src = self;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          buildFeatures = pkgs.lib.optionals withClipboard [ "clipboard" ];

          nativeBuildInputs = pkgs.lib.optionals withClipboard [
            pkgs.pkg-config
            pkgs.git
          ];

          buildInputs = pkgs.lib.optionals (withClipboard && pkgs.stdenv.isLinux) [
            pkgs.wl-clipboard
            pkgs.xclip
            pkgs.git
          ];
          
          postInstall = ''
            mkdir -p $out/share/man/man1
            "$out/bin/ccp-mangen" --output-dir $out/share/man/man1
            rm $out/bin/ccp-mangen
          '';

          meta = with pkgs.lib; {
            description = "Snapshot, blueprint, and scaffold project directory trees to Markdown/.tree and back";
            homepage = "https://github.com/AradPilevarJavid/ccp_tree";
            license = licenses.mit;
            mainProgram = "ccp";
            platforms = platforms.unix;
          };
        };
          default = self.packages.${system}.ccp;
        }
      );

      overlays.default = final: prev: {
        ccp = self.packages.${final.system}.ccp;
      };

      nixosModules.default = { config, pkgs, ... }: {
        nixpkgs.overlays = [ self.overlays.default ];
      };

      apps = forAllSystems (system: {
        ccp = {
          type = "app";
          program = "${self.packages.${system}.ccp}/bin/ccp";
        };
        default = self.apps.${system}.ccp;
      });
    };
}
