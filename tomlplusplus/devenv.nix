{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
let
  tomlplusplus-package = pkgs.stdenv.mkDerivation {
    pname = "tomlplusplus";
    version = "unstable";
    src = inputs.tomlplusplus;

    nativeBuildInputs = with pkgs; [
      gcc15
      pkg-config
    ];

    installPhase = ''
      mkdir -p $out/include
      cp -r include/* $out/include/
    '';
  };
in
{
  languages.c.enable = true;
  languages.cplusplus.enable = true;

  packages = with pkgs; [
    tomlplusplus-package
  ];
}
