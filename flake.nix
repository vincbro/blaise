{
  description = "Rust devshell";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      devShells.x86_64-linux.default = pkgs.mkShell {
        buildInputs = [
          pkgs.cargo
          pkgs.clippy
          pkgs.rustc
          pkgs.rustfmt
          pkgs.rust-analyzer
          pkgs.openssl
          pkgs.nodePackages.vscode-json-languageserver
          pkgs.taplo
          pkgs.dockerfile-language-server
        ];

        shellHook = ''
          export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig";          
          # alias test="cargo test --all-features -- --no-capture";          
        '';
      };
    };
}
