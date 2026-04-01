{ pkgs, ... }:

let
  release =
    {
      x86_64-linux = {
        asset = "turso_cli-x86_64-unknown-linux-gnu.tar.xz";
        hash = "sha256-tGWFKdHygrP2cdE37mYP+/9TXDdniDNQZdeC0kH9Ff8=";
      };
    }
    .${pkgs.stdenv.hostPlatform.system}
      or (throw "Unsupported system for pinned tursodb preview: ${pkgs.stdenv.hostPlatform.system}");

  version = "0.6.0-pre.11";

  src = pkgs.fetchurl {
    url = "https://github.com/tursodatabase/turso/releases/download/v${version}/${release.asset}";
    hash = release.hash;
  };

  tursodbPreview =
    pkgs.runCommand "tursodb-${version}"
      {
        nativeBuildInputs = with pkgs; [
          gnutar
          xz
        ];
      }
      ''
        set -euo pipefail
        tmpdir="$(mktemp -d)"
        trap 'rm -rf "$tmpdir"' EXIT

        mkdir -p "$tmpdir/unpack" "$out/bin"
        tar -xJf "${src}" -C "$tmpdir/unpack"
        install -m755 "$(find "$tmpdir/unpack" -type f -name tursodb -print -quit)" "$out/bin/tursodb"
      '';
in
{
  packages = [
    pkgs.findutils
    pkgs.git
    pkgs.gnugrep
    pkgs.gnused
    tursodbPreview
  ];

  scripts.rebuild-kb.exec = ''
    repo_root="''${DEVENV_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || true)}"
    if [ -z "$repo_root" ]; then
      case "$PWD" in
        */tursodb) repo_root="''${PWD%/tursodb}" ;;
        *) repo_root="$PWD" ;;
      esac
    fi
    exec "$repo_root/tursodb/scripts/rebuild-local-kb.sh" "$@"
  '';

  scripts.sync-kb.exec = ''
    repo_root="''${DEVENV_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || true)}"
    if [ -z "$repo_root" ]; then
      case "$PWD" in
        */tursodb) repo_root="''${PWD%/tursodb}" ;;
        *) repo_root="$PWD" ;;
      esac
    fi
    exec "$repo_root/tursodb/scripts/sync-local-kb.sh" "$@"
  '';

  scripts.search-kb.exec = ''
    repo_root="''${DEVENV_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || true)}"
    if [ -z "$repo_root" ]; then
      case "$PWD" in
        */tursodb) repo_root="''${PWD%/tursodb}" ;;
        *) repo_root="$PWD" ;;
      esac
    fi
    exec "$repo_root/tursodb/scripts/search-local-kb.sh" "$@"
  '';

  scripts.search-kb-fts.exec = ''
    repo_root="''${DEVENV_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || true)}"
    if [ -z "$repo_root" ]; then
      case "$PWD" in
        */tursodb) repo_root="''${PWD%/tursodb}" ;;
        *) repo_root="$PWD" ;;
      esac
    fi
    exec "$repo_root/tursodb/scripts/search-local-kb-fts.sh" "$@"
  '';

  scripts.search-kb-vector.exec = ''
    repo_root="''${DEVENV_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || true)}"
    if [ -z "$repo_root" ]; then
      case "$PWD" in
        */tursodb) repo_root="''${PWD%/tursodb}" ;;
        *) repo_root="$PWD" ;;
      esac
    fi
    exec "$repo_root/tursodb/scripts/search-local-kb-vector.sh" "$@"
  '';

  enterShell = ''
    repo_root="''${DEVENV_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || true)}"
    if [ -z "$repo_root" ]; then
      case "$PWD" in
        */tursodb) repo_root="''${PWD%/tursodb}" ;;
        *) repo_root="$PWD" ;;
      esac
    fi
    mkdir -p "$repo_root/.turso"
    export TURSODB_DB_PATH="$repo_root/.turso/knowledge.db"

    if [ -t 1 ] && [ -t 2 ]; then
      echo "tursodb devenv shell" >&2
      echo "  package: tursodb ${version}" >&2
      echo "  database path: $TURSODB_DB_PATH" >&2
      echo "  start shell: tursodb" >&2
      echo "  rebuild local kb: rebuild-kb" >&2
      echo "  incremental kb sync: sync-kb [path ...]" >&2
      echo "  search local kb: search-kb \"query text\"" >&2
      echo "  fts-only search: search-kb-fts \"query text\"" >&2
      echo "  vector-only search: search-kb-vector \"query text\"" >&2
      echo "  open persistent db: .open $TURSODB_DB_PATH" >&2
    fi
  '';
}
