{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

let
  pkgs-stable = import inputs.nixpkgs-stable { system = pkgs.stdenv.system; };
in
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
    python = {
      enable = true;
      package = pkgs.python3;
      libraries = with pkgs; [
        zlib
        stdenv.cc.cc.lib
      ];
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
    mold-wrapped
    linuxPackages.perf
    glibc.out
    glibc.debug
    numactl
    libcap
    doctest
    clang
    libclang
    # Python packages for benchmark visualization
    python3Packages.pandas
    python3Packages.matplotlib
    python3Packages.numpy
    pkgs-stable.python3Packages.plotly
  ];

  # 1. Force C/C++ Compiler Flags
  # -g: Generate debug info
  # -fno-omit-frame-pointer: Crucial for 'perf' to unwind stacks correctly
  env.CFLAGS = "-g -fno-omit-frame-pointer";
  env.CXXFLAGS = "-g -fno-omit-frame-pointer";
  env.LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
  env.NIX_ENFORCE_NO_NATIVE = "0";

  # https://devenv.sh/languages/
  # languages.rust.enable = true;

  # https://devenv.sh/processes/
  # processes.dev.exec = "${lib.getExe pkgs.watchexec} -n -- ls -la";

  # https://devenv.sh/services/
  # services.postgres.enable = true;

  # https://devenv.sh/scripts/
  scripts.launch.exec = ''
    REPO_ROOT="$(git rev-parse --show-toplevel)"
    TOOLS_DIR="$REPO_ROOT/tools"
    BUILD_DIR="$TOOLS_DIR/build"
    LAUNCHER="$BUILD_DIR/dsa_launcher"

    if [ ! -f "$LAUNCHER" ]; then
      echo "Building dsa_launcher..."
      mkdir -p "$BUILD_DIR"
      gcc -o "$LAUNCHER" "$TOOLS_DIR/dsa_launcher.c"
      sudo setcap cap_sys_rawio+eip "$LAUNCHER"
    fi

    if [ $# -eq 0 ]; then
      echo "Usage: launch <command> [args...]"
      echo "Runs <command> with CAP_SYS_RAWIO via dsa_launcher"
      exit 1
    fi

    echo "Running: $LAUNCHER $@"
    exec "$LAUNCHER" "$@"
  '';

  scripts.run.exec = ''
    REPO_ROOT="$(git rev-parse --show-toplevel)"
    DSA_DIR="$REPO_ROOT/dsa-stdexec"
    MODE=$(cd "$DSA_DIR" && xmake show 2>&1 | sed 's/\x1b\[[0-9;]*m//g' | grep 'mode:' | awk '{print $2}')
    BENCHMARK="$DSA_DIR/build/linux/x86_64/$MODE/dsa_benchmark"

    if [ ! -f "$BENCHMARK" ]; then
      echo "Benchmark not found. Building dsa_benchmark..."
      (cd "$DSA_DIR" && xmake build dsa_benchmark)
    fi

    echo "Running: launch $BENCHMARK $@"
    exec launch "$BENCHMARK" "$@"
  '';

  # https://devenv.sh/scripts/
  scripts.profile.exec = ''
    REPO_ROOT="$(git rev-parse --show-toplevel)"
    DSA_DIR="$REPO_ROOT/dsa-stdexec"
    MODE=$(cd "$DSA_DIR" && xmake show 2>&1 | sed 's/\x1b\[[0-9;]*m//g' | grep 'mode:' | awk '{print $2}')
    BENCHMARK="$DSA_DIR/build/linux/x86_64/$MODE/dsa_benchmark"

    if [ ! -f "$BENCHMARK" ]; then
      echo "Benchmark not found. Building dsa_benchmark..."
      (cd "$DSA_DIR" && xmake build dsa_benchmark)
    fi

    echo "Running: launch samply record $BENCHMARK $@"
    exec launch samply record "$BENCHMARK" "$@"
  '';

  # https://devenv.sh/basics/
  enterShell = ''
    # 1. Ask the wrapped GCC where its headers are
    # 2. Clean up the output
    # 3. Export to CPLUS_INCLUDE_PATH so clangd sees it
    export CPLUS_INCLUDE_PATH=$(gcc -E -Wp,-v -xc++ /dev/null 2>&1 | grep '^ ' | awk '{print $1}' | tr '\n' ':')

    if [ -t 1 ] && [ -t 2 ]; then
      echo "Updated CPLUS_INCLUDE_PATH for gcc15" >&2
    fi
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
