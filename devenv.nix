{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
{

  languages = {
    c = {
      enable = true;
    };
    cplusplus = {
      enable = true;
    };
    rust = {
      enable = true;
    };
  };

  # 3. Environment Variables (The Fix)
  # This ensures build systems use GCC 15 instead of the default stdenv compiler.
  env.CC = "gcc";
  env.CXX = "g++";

  # https://devenv.sh/packages/
  packages = with pkgs; [
    git
    gcc15
    xmake
    cmake
    aria2
    llvmPackages.bintools
    cpptrace
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
    ninja
    samply
    linuxPackages.perf
    pkgs.glibc.out
    pkgs.glibc.debug
    numactl
  ];

  # 1. Force C/C++ Compiler Flags
  # -g: Generate debug info
  # -fno-omit-frame-pointer: Crucial for 'perf' to unwind stacks correctly
  env.CFLAGS = "-g -fno-omit-frame-pointer";
  env.CXXFLAGS = "-g -fno-omit-frame-pointer";

  env.NIX_ENFORCE_NO_NATIVE = "0";

  # https://devenv.sh/languages/
  # languages.rust.enable = true;

  # https://devenv.sh/processes/
  # processes.dev.exec = "${lib.getExe pkgs.watchexec} -n -- ls -la";

  # https://devenv.sh/services/
  # services.postgres.enable = true;

  # https://devenv.sh/scripts/
  scripts.hello.exec = ''
    echo hello from $GREET
  '';

  # https://devenv.sh/basics/
  enterShell = ''
    # 1. Ask the wrapped GCC where its headers are
    # 2. Clean up the output
    # 3. Export to CPLUS_INCLUDE_PATH so clangd sees it
    export CPLUS_INCLUDE_PATH=$(gcc -E -Wp,-v -xc++ /dev/null 2>&1 | grep '^ ' | awk '{print $1}' | tr '\n' ':')

    echo "Updated CPLUS_INCLUDE_PATH for gcc15"
  '';

  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };

  # https://devenv.sh/tests/
  enterTest = ''
    echo "Running tests"
    git --version | grep --color=auto "${pkgs.git.version}"
  '';

  # https://devenv.sh/git-hooks/
  # git-hooks.hooks.shellcheck.enable = true;

  # See full reference at https://devenv.sh/reference/options/
}
