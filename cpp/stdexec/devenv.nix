{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
let
  stdexec-package = pkgs.stdenv.mkDerivation {
    pname = "stdexec";
    version = "unstable";
    src = inputs.stdexec;

    nativeBuildInputs = with pkgs; [
      gcc15
      meson
      pkg-config
      ninja
    ];
  };
in
{
  languages = {
    cplusplus = {
      enable = true;
    };
    c = {
      enable = true;
    };
  };

  packages = with pkgs; [
    stdexec-package
  ];
}