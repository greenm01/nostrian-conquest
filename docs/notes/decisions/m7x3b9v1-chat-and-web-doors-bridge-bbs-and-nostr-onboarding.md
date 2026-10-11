---
id: m7x3b9v1
date: 2026-10-11
kind: adr
status: accepted
tags: [adr, architecture, hosting]
---
# Chat and web doors bridge BBS and Nostr onboarding

## Context

Nostrian Conquest was conceived with two distinct public deployment faces
bound by a single universal game engine (`nc-engine`):
1. **Phase 1: The Classic BBS Door** (`nc-door` over Telnet/SSH dropfiles),
   preserving authentic DOS/BBS aesthetics and drop-in compatibility.
2. **Phase 2: Decentralized Nostr Multiplayer** (`nc-host` + `nc-helm` over
   relays), offering asynchronous, cryptographic, sovereign campaigns.

While technically sound, this macro split leaves a severe **friction chasm**
for onboarding new players:
* **Telnet BBSes** require specialized terminal software (SyncTERM, NetRunner),
  port configurations, and dedicated BBS account creation before entering a
  door. This restricts player growth to active BBS enthusiasts.
* **Nostr Multiplayer** has a cold-start problem: players rarely generate
  cryptographic keypairs and download custom client binaries (`nc-helm`) for
  games they have never played or seen in action.

Modern group chat ecosystems (Discord Embedded Activities, Matrix Widgets) and
standard browser webviews allow embedding full HTML5 canvas/terminal frames
(`xterm.js`) directly into channels where gaming communities already socialize.

## Decision

Adopt modern chat rooms (Discord Activities, Matrix Widgets) and browser-based
web doors as an intermediate, first-class hosted deployment tier using
`nc-door` and the structured **[`DOOR.KDL`](../../sysop/door-kdl-spec.md)**
specification.

Under this model:
1. **Engine and door sovereignty are preserved:** `nc-door` remains protocol-
   agnostic. The host chat room is treated as an external, untrusted "dumb
   glass" terminal proxy (identical to a classic BBS node).
2. **Chat/Web doors serve as the frictionless on-ramp:** Players join games
   with zero client installation directly from voice or text channels.
3. **Identity bridges forward to Nostr:** The `DOOR.KDL` specification includes
   an optional `public_key` field (`npub`), allowing players to bind a sovereign
   cryptographic key to their campaign seat and graduate to `nc-helm` without
   abandoning their empires.

## Alternatives

1. **Keep only BBS and Nostr faces:** Rejected. Keeps prospective campaigns
   isolated and severely throttles playtesting discovery.
2. **Build a Discord-native bot (Slash commands / UI buttons):** Rejected.
   Replacing the 80x25 terminal with Discord buttons destroys the authentic
   game loop, hits Discord's strict message edit rate limits (HTTP 429), and
   creates tight coupling to a centralized proprietary platform.
3. **Rely on Telnet-in-the-browser (fTelnet) into a full BBS:** Rejected.
   Forces users through a multi-step BBS registration and menu maze, losing
   the instant 2-second onboarding advantage of modern chat activities.

## Consequences

* **Immediate Community Activation:** Sysops can host live campaigns inside
  existing Discord servers or Matrix spaces where friends already hang out.
* **Shared Technology Stack:** The browser iframe client (`xterm.js` + CP437
  rendering + WebSocket bridge) is written once and shared identically across
  Discord Activities, Matrix Widgets, and standalone web browsers.
* **Ecosystem Progression:** Players move along an onboarding funnel:
  `Chat Door (Zero Install)` ──► `Web Door` ──► `Sovereign Nostr (nc-helm)` or
  `BBS Telnet`.
* **Specification Work:** Formalized in
  [`docs/sysop/discord-door-architecture.md`](../../sysop/discord-door-architecture.md)
  and [`docs/sysop/door-kdl-spec.md`](../../sysop/door-kdl-spec.md).

## Acceptance and connections

Proposed and accepted on 2026-10-11.
* Architectural spec:
  [`docs/sysop/discord-door-architecture.md`](../../sysop/discord-door-architecture.md)
* Dropfile spec: [`docs/sysop/door-kdl-spec.md`](../../sysop/door-kdl-spec.md)
* Macro vision: [`docs/grand-vision.md`](../../grand-vision.md)
