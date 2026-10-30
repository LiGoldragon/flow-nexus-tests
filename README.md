# Flow Nexus v1 contract and Memory checks

Immutable production targets are declared in flake.nix:

- flow-nexus f5ce26dd62565745caee85c81005f12d383c98c6
- signal-flow-nexus 8faf87334ff40a0d84884c04046b3c5d6d989887
- meta-signal-flow-nexus 743900f3ea07f7722c066b7dbd271bd9dfab8308

Nix assembles these sources as sibling directories. No production source is copied into this repository. Every check runs in its own Nix sandbox.

## Witnessed checks

- `checks.x86_64-linux.memory`: four tests of actual Flow RecordState, MemoryStore and MemoryPort. Counter regression/deletion, duplicate batch rejection, rejected multi-row batch unchanged, durable reopen, typed actor committed/rejected/read replies. Latest output `/nix/store/273icmkdjq8x993wp8mc5i3xg2m1zfm9-flow-nexus-memory-tests-test-0.1.0`.
- `checks.x86_64-linux.identity`: three expected-invariant tests failed on prior cd4a184, then passed on b161702: global FlowId reservation uniqueness, launch/end RequestId separation, completed-launch Flow reference. Output `/nix/store/c5r24n2xh0qsmvwzczw59jia09bf7h24-flow-nexus-identity-tests-test-0.1.0`.
- `checks.x86_64-linux.anatomy`: four static schema checks. Independently enumerates actual authored Memory-root rows against typed owner Record union and generic read/mutation surface; Request variants and Declaration blacklist survive generated projection. Output `/nix/store/04saj3vk83ifl2knj2dd4vlcjd8gapip-flow-nexus-anatomy-tests-test-0.1.0`.
- `checks.x86_64-linux.normal-closure`: actual no-default-feature normal Cargo dependency graph excludes datom-codec/protos. Output `/nix/store/jy87d7chk9ydvhnrzcdrn7z0p53a4q2r-flow-nexus-normal-closure-build-0.1.0`. This proves a library graph, not a runnable Nexus closure.

## Historical textual failures, fixed upstream

`checks.x86_64-linux.newtype-projection` expects transparent newtypes from independently specified forms: Topic scalar `flowNexus`, Address struct `{ flowNexus }`. All three tests failed on earlier shared derive4dff16b: Topic writer emits `{ flowNexus }`, Topic reader fails at [], Address reader at [0], expected Struct/found Bare. Exact failed derivation `/nix/store/qb79jz323fs6cgb9ykzaj0mp3dazfh1x-flow-nexus-newtype-projection-test-0.1.0.drv`. The supplied Launch and End fixtures remained unchanged during that failure. No braces were added to make the tests green.

Commit cea1aa8 mistakenly weakened the minimal newtype oracle and tested only upstream Memory with an invented Reply. The following correction restores the external oracle and exercises the actual Flow types; that earlier green upstream-only test is not Flow evidence.

## Evidence boundaries

Identity fixtures were authored independently following a read-only audit. The integrating flow authored actual Memory tests and strengthened schema/newtype oracles after finding weaknesses in the initial suite. Static source checks do not prove full transitive privacy or runtime meta authorization. No crash injection, lost actor reply, store-failure injection, live verification/fingerprint, socket identity, namespace/pane inheritance, launch/end, Input/Output/root traits, Translator or release acceptance is claimed. These require further scenarios against an executable; none exists yet.

## Authoritative Datom correction

Datom eb10157a0336e3aa2401e5b2046211fc8e736397 (0.32.3) and Ethos Zero b8052c2404374265dfcddbde83fef76977f78acd (19.0.1) fix the transparent tuple-newtype projection upstream. No local workaround or oracle change. Full schema check now passes all20 tests (6 Datom,3 identity,4 Memory,3 minimal-newtype,4 static schema): /nix/store/smg81m70aaqd4vd9x2g7cw9qsap5nvl4-flow-nexus-schema-tests-test-0.1.0. Normal binary graph also passes /nix/store/201kq8c5b2v1cbn59258ii08kfrr91a1-flow-nexus-normal-closure-build-0.1.0. These outputs target the three immutable dependency-upgrade commits above; later structural/OriginAdmission changes need their own tests.

## Structural identity/evidence slice

Current targets use main f5ce26d with the same upgraded wire commits. Full schema suite now passes28 tests:6 Datom,11 identity/structural cases,4 actual Memory,3 minimal-newtype,4 static schema. Output /nix/store/sp2dpzncqxrmwmlpy7hczm6mf26m42zx-flow-nexus-schema-tests-test-0.1.0. New positives accept the intentional same-flow reservation/Flow link and distinct admission/launch ids linked by OriginAdmissionId. Existing-state collision, cross-admission reuse, locator mismatch, prepared/no-resource contradiction and configured alphabet domain are rejected. Fresh tester authored the positive/collision/admission fixtures; integrating flow reviewed and corrected invalid drafts before running, and added locator/phase/alphabet cases. These are structural checks, not external witness authority or atomic allocator acceptance. Atomic reservation/allocation is not yet implemented.
