{ pkgs, lib, ... }:

let
  version = "0.1.4";

  src = pkgs.fetchurl {
    url = "https://registry.npmjs.org/codemogger/-/codemogger-${version}.tgz";
    hash = "sha256-vCLNA0Dc81CToKVVMs3rv/lWLrumOOP/F95TVKG1ZlY=";
  };

  codemoggerPkg = pkgs.buildNpmPackage {
    pname = "codemogger";
    inherit version src;
    sourceRoot = "package";
    postPatch = ''
      cp ${./package-lock.json} package-lock.json
    '';
    npmFlags = [ "--legacy-peer-deps" "--ignore-scripts" ];
    npmDepsHash = "sha256-pnK+a8qoD0bUAA3sHMffLf6rrJaYQZtASQVRi833gxc=";
    dontNpmBuild = true;
    postFixup = ''
      preload="$out/lib/node_modules/codemogger/nix-preload.mjs"
      cat > "$preload" <<'EOF'
      import path from "node:path";
      import { env } from "@huggingface/transformers";

      const codemoggerHome =
        process.env.CODEMOGGER_HOME ||
        path.join(process.cwd(), ".codemogger");
      const cacheRoot =
        process.env.CODEMOGGER_CACHE_DIR ||
        path.join(codemoggerHome, "cache");
      const transformersCache =
        process.env.TRANSFORMERS_CACHE ||
        path.join(cacheRoot, "transformers");

      env.cacheDir = transformersCache;
      EOF

      mv "$out/bin/codemogger" "$out/bin/.codemogger-real"
      cat > "$out/bin/codemogger" <<EOF
      #!${pkgs.runtimeShell}
      set -eu

      export CODEMOGGER_HOME="''${CODEMOGGER_HOME:-\$PWD/.codemogger}"
      export CODEMOGGER_CACHE_DIR="''${CODEMOGGER_CACHE_DIR:-\$CODEMOGGER_HOME/cache}"
      export HF_HOME="''${HF_HOME:-\$CODEMOGGER_CACHE_DIR/huggingface}"
      export TRANSFORMERS_CACHE="''${TRANSFORMERS_CACHE:-\$CODEMOGGER_CACHE_DIR/transformers}"

      mkdir -p "\$CODEMOGGER_HOME" "\$CODEMOGGER_CACHE_DIR" "\$HF_HOME" "\$TRANSFORMERS_CACHE"

      if [ -n "''${NODE_OPTIONS:-}" ]; then
        export NODE_OPTIONS="\$NODE_OPTIONS --import=$preload"
      else
        export NODE_OPTIONS="--import=$preload"
      fi

      exec "$out/bin/.codemogger-real" "\$@"
      EOF
      chmod +x "$out/bin/codemogger"
    '';
  };
in
{
  packages = [
    codemoggerPkg
    pkgs.git
  ];

  enterShell = ''
    repo_root="''${DEVENV_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || true)}"
    if [ -z "$repo_root" ]; then
      case "$PWD" in
        */codemogger) repo_root="''${PWD%/codemogger}" ;;
        *) repo_root="$PWD" ;;
      esac
    fi
    mkdir -p "$repo_root/.codemogger/cache/transformers" "$repo_root/.codemogger/cache/huggingface"
    export CODEMOGGER_HOME="$repo_root/.codemogger"
    export CODEMOGGER_CACHE_DIR="$CODEMOGGER_HOME/cache"
    export HF_HOME="$CODEMOGGER_CACHE_DIR/huggingface"
    export TRANSFORMERS_CACHE="$CODEMOGGER_CACHE_DIR/transformers"

    echo "codemogger devenv shell"
    echo "  package: codemogger ${version}"
    echo "  local state dir: $CODEMOGGER_HOME"
    echo "  model cache: $TRANSFORMERS_CACHE"
    echo "  index repo: codemogger index ."
    echo "  search index: codemogger search \"query text\""
  '';
}
