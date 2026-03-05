{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
let
  perfetto-package = pkgs.stdenv.mkDerivation {
    pname = "perfetto";
    version = "unstable";
    src = inputs.perfetto;

    nativeBuildInputs = [ pkgs.gcc15 ];

    buildPhase = ''
      g++ -o perfetto.o -c $src/sdk/perfetto.cc -I$src
      ar rcs libperfetto.a perfetto.o
    '';

    installPhase = ''
      mkdir -p $out/lib
      cp libperfetto.a $out/lib
      mkdir -p $out/include
      cp $src/sdk/perfetto.h $out/include
    '';

  };
in
{
  languages.c.enable = true;
  languages.cplusplus.enable = true;

  packages = with pkgs; [
    perfetto-package
  ];
}
