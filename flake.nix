{
  description = "Development shell xmake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/0f2f740139ea83741ba38413ce427b0e222467e3";
    idxd-config = {
      url = "git+https://github.com/intel/idxd-config.git?ref=stable&rev=b78f65f5d342643815124d36a52023fda4a945e3";
      flake = false;
    };
    stdexec = {
      url = "git+https://github.com/NVIDIA/stdexec.git?rev=7bb7b5b6d26e5ccedd88a2d7a131e86b58270108";
      flake = false;
    };
    proxy = {
      url = "git+https://github.com/microsoft/proxy.git?rev=f88a3a3bcf0f5e87b5817a3619753a982885ef82";
      flake = false;
    };
    tomlplusplus = {
      url = "git+https://github.com/marzer/tomlplusplus.git?ref=master";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      idxd-config,
      stdexec,
      proxy,
      tomlplusplus,
      ...
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        system = system;
        config = { };
        overlays = [ ];
      };
      gcc15 = pkgs.gcc15;
      stdenv = pkgs.impureUseNativeOptimizations pkgs.stdenv;
    in
    {

      idxd-config-package = stdenv.mkDerivation {
        pname = "idxd-config";
        version = "unstable";
        src = idxd-config;

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

      stdexec-pkg = stdenv.mkDerivation {
        pname = "stdexec";
        version = "unstable";
        src = stdexec;

        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        installPhase = ''
          mkdir -p $out/include
          cp -r include/* $out/include/
        '';
      };

      proxy-pkg = stdenv.mkDerivation {
        pname = "proxy";
        version = "unstable";
        src = proxy;

        nativeBuildInputs = with pkgs; [
          pkg-config
          # cmake
          # ninja
        ];

        installPhase = ''
          mkdir -p $out/include
          cp -r include/* $out/include/
        '';
      };

      tomlplusplus-pkg = stdenv.mkDerivation {
        pname = "tomlplusplus";
        version = "unstable";
        src = tomlplusplus;

        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        installPhase = ''
          mkdir -p $out/include
          cp -r include/* $out/include/
        '';
      };

      devShells.${system}.default =
        pkgs.mkShellNoCC.override
          {
            stdenv = pkgs.gcc15.stdenv;
          }
          {
            hardeningDisable = [ "fortify" ];

            NIX_ENFORCE_NO_NATIVE = "0";

            packages = with pkgs; [
              gcc15
              xmake
              self.idxd-config-package
              cmake
              aria2
              llvmPackages.bintools
              self.stdexec-pkg
              self.proxy-pkg
              self.tomlplusplus-pkg
              cpptrace
            ];

            nativeBuildInputs = with pkgs; [
              gcc15
              libuuid
              json_c
              libtool
              autoconf
              automake
              pkg-config
              asciidoc
              which
              xmlto
              fmt_12
              cmake
              ninja
              llvmPackages.bintools
              self.stdexec-pkg
              self.proxy-pkg
              self.tomlplusplus-pkg
              cpptrace
            ];

            shellHook = ''
              # 1. Ask the wrapped GCC where its headers are
              # 2. Clean up the output
              # 3. Export to CPLUS_INCLUDE_PATH so clangd sees it
              export CPLUS_INCLUDE_PATH=$(gcc -E -Wp,-v -xc++ /dev/null 2>&1 | grep '^ ' | awk '{print $1}' | tr '\n' ':')

              echo "Updated CPLUS_INCLUDE_PATH for gcc15"
            '';
          };
    };
}
