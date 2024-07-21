{
 description = "A flake for regtesting bitcoin core";

 inputs = {
   nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
   flake-utils.url = "github:numtide/flake-utils";
 };

 outputs = { self, nixpkgs, flake-utils }:
   flake-utils.lib.eachDefaultSystem (system:
     let
       pkgs = nixpkgs.legacyPackages.${system};
     in {
       devShells = {
         default = pkgs.mkShell {
           name = "bitcoin-dev-shell";

           packages = with pkgs; [
             bitcoind
           ];

           shellHook = ''
             export PATH_TO_BITCOIN=$(pwd)/.bitcoin
             export PATH_TO_MAINNET=~/.bitcoin

             mkdir -p $PATH_TO_BITCOIN

             printf "regtest=1\ndaemon=1\nrpcuser=bitcoin\nrpcpassword=password" > $PATH_TO_BITCOIN/bitcoin.conf

             alias btcd="bitcoind -datadir=$PATH_TO_BITCOIN"
             alias btcli="bitcoin-cli -datadir=$PATH_TO_BITCOIN"

             function reset() {
               btcli stop 2>/dev/null
               rm -rf $PATH_TO_BITCOIN
               mkdir -p $PATH_TO_BITCOIN
             }
           '';
         };
      };
   });
}


