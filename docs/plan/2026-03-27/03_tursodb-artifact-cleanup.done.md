# Clean up obsolete nested `tursodb` artifacts

## Goal

Remove the leftover imperative download artifacts from `/home/hongtao/accel-datapath/agent-env-wt/tursodb` now that the setup is Nix-native.

## Plan

1. Delete the no-longer-used local `bin/` and `vendor/` artifacts from the nested setup.
2. Remove any now-empty helper directories left by the old workflow.
3. Update the nested docs and ignore rules to match the cleaned layout.
