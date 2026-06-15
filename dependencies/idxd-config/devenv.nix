{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
let
  idxd-config-package = pkgs.stdenv.mkDerivation {
    pname = "idxd-config";
    version = "unstable";
    src = inputs.idxd-config;

    nativeBuildInputs = with pkgs; [
      xmake
      libtool
      autoconf
      automake
      pkg-config
      asciidoc
      which
      xmlto
      libuuid
      json_c
      autoreconfHook
      gcc15
    ];

    patchPhase = ''
      patchShebangs ./git-version
      patchShebangs ./git-version-gen
      patchShebangs ./autogen.sh
      ./autogen.sh
    '';
    configurePhase = ''
      # autoupdate
      ./configure CFLAGS='-g -O2' --disable-docs  --prefix=$out --libdir=$out/lib64
    '';
    buildPhase = ''
      make -j
    '';
    checkPhase = ''
      make check
    '';
    installPhase = ''
      make install
    '';
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
    idxd-config-package
  ];
}