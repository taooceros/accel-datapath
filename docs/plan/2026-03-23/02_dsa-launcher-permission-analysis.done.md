# Analyze `dsa_launcher` permission failure

## Goal

Determine why `tools/build/dsa_launcher` cannot successfully grant
`CAP_SYS_RAWIO` to child processes on this machine.

## Plan

1. Inspect launcher documentation, source, and prior notes for intended
   capability flow.
2. Check the built launcher capability bits and reproduce the current failure.
3. Explain the failure mode and document the concrete code-level fix needed.
