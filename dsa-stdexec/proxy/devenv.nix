{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
let
  proxy-package = pkgs.stdenv.mkDerivation {
    pname = "proxy";
    version = "unstable";
    src = inputs.proxy;

    nativeBuildInputs = with pkgs; [
      gcc15
      cmake
      pkg-config
      ninja
    ];

  };
in
{
  languages.c.enable = true;
  languages.cplusplus.enable = true;

  packages = with pkgs; [
    proxy-package
  ];
}
