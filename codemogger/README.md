# Local `codemogger` wrapper

This directory is a self-contained `devenv` setup for the `codemogger` CLI.

The CLI is provided through Nix and imported into the repo root shell.
Local codemogger state is kept under `.codemogger/` at the repository root.
The runtime wrapper also redirects Hugging Face model cache writes to `.codemogger/cache/transformers`.

## Usage

```bash
cd /home/hongtao/accel-datapath/agent-env-wt/codemogger
devenv shell
codemogger index .
codemogger search "authentication middleware"
```

From the repo root:

```bash
devenv shell
codemogger index .
codemogger search "query text"
```
