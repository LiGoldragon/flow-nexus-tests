# Flow Nexus v1 schema-only acceptance

Immutable implementation targets:

- `flow-nexus@db7423cdbd38b8d2834b793416e6b6141f8594b8`
- `signal-flow-nexus@0bc74491f682ec5d0f037c64611f9fa30a574346`
- `meta-signal-flow-nexus@145eaa3d2fc1a6796549c68dbb7d3fadb80c026e`

These tests cover generated schema vocabulary, Datom projections and a no-Datom normal Cargo dependency graph. They make no launch/runtime/transport/Operation, new Input/Output/implied-trait, Translator or release claim. Nix assembles the immutable targets as child source trees; no production source is copied into this test repository.

## Results

`nix build --no-link .#checks.x86_64-linux.normal-closure` passed. The normal no-default-feature Cargo graph contains neither datom-codec nor protos. This is library dependency-graph evidence, not a runnable Nexus closure proof.

`nix build --no-link .#checks.x86_64-linux.schema` currently fails intentionally against the supplied external-form expectation: the Datom suite has **4 passing and 2 failing** tests. The supplied Launch form expects bare Topic/Remembering/Brief strings; `End.a7b8c9` expects a bare FlowId. The current newtype derive instead requires a Struct. The failures are preserved pending Primary judgment, not ignored or rewritten to fit the implementation. Failure derivation: `/nix/store/6ys1w8i7x36anz4p9g99hbcfw3s2mgkv-flow-nexus-schema-tests-test-0.1.0.drv`.

An earlier weaker five-test suite passed roundtrips and manifest checks, which did not detect this projection mismatch. Source-text optional-feature assertions remain narrowly named as declaration evidence; the separate Cargo graph check supplies actual normal-dependency evidence.

The expected-field fixtures were strengthened by the integrating flow after the independent initial suite. Full runtime acceptance still requires independent scenarios against the eventual immutable executable.
