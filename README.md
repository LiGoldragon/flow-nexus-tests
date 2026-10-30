# Flow Nexus v1 contract and Memory checks

Immutable production targets are declared in flake.nix:

- flow-nexus b161702b9fe9fc678e4a55e81be9341b076fa30f
- signal-flow-nexus 11734f0502586378c2b055e51e81b8c4f9e6d1fd
- meta-signal-flow-nexus 7d47476a61e3a5ec7b9b813a910a6c40798ba4bf

Nix assembles these sources as sibling directories. No production source is copied into this repository. Every check runs in its own Nix sandbox.

## Witnessed checks

- `checks.x86_64-linux.memory`: four tests of actual Flow RecordState, MemoryStore and MemoryPort. Counter regression/deletion, duplicate batch rejection, rejected multi-row batch unchanged, durable reopen, typed actor committed/rejected/read replies. Latest output `/nix/store/273icmkdjq8x993wp8mc5i3xg2m1zfm9-flow-nexus-memory-tests-test-0.1.0`.
- `checks.x86_64-linux.identity`: three expected-invariant tests failed on prior cd4a184, then passed on b161702: global FlowId reservation uniqueness, launch/end RequestId separation, completed-launch Flow reference. Output `/nix/store/c5r24n2xh0qsmvwzczw59jia09bf7h24-flow-nexus-identity-tests-test-0.1.0`.
- `checks.x86_64-linux.anatomy`: four static schema checks. Independently enumerates actual authored Memory-root rows against typed owner Record union and generic read/mutation surface; Request variants and Declaration blacklist survive generated projection. Output `/nix/store/04saj3vk83ifl2knj2dd4vlcjd8gapip-flow-nexus-anatomy-tests-test-0.1.0`.
- `checks.x86_64-linux.normal-closure`: actual no-default-feature normal Cargo dependency graph excludes datom-codec/protos. Output `/nix/store/jy87d7chk9ydvhnrzcdrn7z0p53a4q2r-flow-nexus-normal-closure-build-0.1.0`. This proves a library graph, not a runnable Nexus closure.

## Preserved textual failures

`checks.x86_64-linux.newtype-projection` expects transparent newtypes from independently specified forms: Topic scalar `flowNexus`, Address struct `{ flowNexus }`. All three tests fail on pinned shared derives: Topic writer emits `{ flowNexus }`, Topic reader fails at [], Address reader at [0], expected Struct/found Bare. Exact failed derivation `/nix/store/qb79jz323fs6cgb9ykzaj0mp3dazfh1x-flow-nexus-newtype-projection-test-0.1.0.drv`. The supplied Launch and End fixtures in the broader schema check also remain unchanged and failing. No braces were added to make the tests green.

Commit cea1aa8 mistakenly weakened the minimal newtype oracle and tested only upstream Memory with an invented Reply. The following correction restores the external oracle and exercises the actual Flow types; that earlier green upstream-only test is not Flow evidence.

## Evidence boundaries

Identity fixtures were authored independently following a read-only audit. The integrating flow authored actual Memory tests and strengthened schema/newtype oracles after finding weaknesses in the initial suite. Static source checks do not prove full transitive privacy or runtime meta authorization. No crash injection, lost actor reply, store-failure injection, live verification/fingerprint, socket identity, namespace/pane inheritance, launch/end, Input/Output/root traits, Translator or release acceptance is claimed. These require further scenarios against an executable; none exists yet.
