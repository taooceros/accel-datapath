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
    pkgs.glibc.out
    pkgs.glibc.debug
    numactl
    libcap
    doctest
    # Python packages for benchmark visualization
    python3Packages.pandas
    python3Packages.matplotlib
    python3Packages.numpy
    python3Packages.plotly
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
  scripts.run.exec = ''
    MODE=$(xmake show 2>&1 | sed 's/\x1b\[[0-9;]*m//g' | grep 'mode:' | awk '{print $2}')
    BUILD_DIR="./build/linux/x86_64/$MODE"
    LAUNCHER="$BUILD_DIR/dsa_launcher"
    BENCHMARK="$BUILD_DIR/dsa_benchmark"

    if [ ! -f "$LAUNCHER" ]; then
      echo "Launcher not found. Building dsa_launcher..."
      xmake build dsa_launcher
    fi

    if [ ! -f "$BENCHMARK" ]; then
      echo "Benchmark not found. Building dsa_benchmark..."
      xmake build dsa_benchmark
    fi

    echo "Running: $LAUNCHER $BENCHMARK $@"
    $LAUNCHER "$BENCHMARK" "$@"
    python3 "benchmark/visualize_interactive.py"
  '';

  # https://devenv.sh/scripts/
  scripts.profile.exec = ''
    MODE=$(xmake show 2>&1 | sed 's/\x1b\[[0-9;]*m//g' | grep 'mode:' | awk '{print $2}')
    BUILD_DIR="./build/linux/x86_64/$MODE"
    LAUNCHER="$BUILD_DIR/dsa_launcher"
    BENCHMARK="$BUILD_DIR/dsa_benchmark"

    if [ ! -f "$LAUNCHER" ]; then
      echo "Launcher not found. Building dsa_launcher..."
      xmake build dsa_launcher
    fi

    if [ ! -f "$BENCHMARK" ]; then
      echo "Benchmark not found. Building dsa_benchmark..."
      xmake build dsa_benchmark
    fi

    echo "Running: $LAUNCHER samply record $BENCHMARK $@"
    exec "$LAUNCHER" samply record "$BENCHMARK" "$@"
    exec "python3" "benchmark/visualize_interactive.py"
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
