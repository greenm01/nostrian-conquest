---
id: om9r4ejo
date: 2026-10-11
kind: plan
tags: [plan, door]
---
# Keep command prompts short and input focused

### t107

Audit the door command prompts for embedded instructions. Keep the action,
choices, input limits and defaults; put notation explanations in help.
Do not change key handling, selection rules or confirmation steps.

## Findings and changes

- Detach and transfer repeated `n! = crippled only` on the quantity line.
  Both now use `CA to stage (max 4) [1] <Q>`. The help still explains `n!`.
  Removing the conditional text also removes an unnecessary damage lookup.
- Commissioning used full sentences in the table footer. It now says
  `<ENTER> Commission fleet <Q>` or `<ENTER> Commission starbase <Q>`.
  This preserves the existing explicit Enter action on that screen.
- Mass commissioning repeated its title before the question. It now asks
  `Commission all ships and starbases? [Y]/N`.

Reviewed fleet orders, planet build/transport/Scorch flows, startup reports,
naming, messaging, starbase movement, map and sort/filter prompts, plus
corresponding helm popups. Most already use short input labels. Keep numeric
limits and defaults. Keep `NS` labelled non-stop in the report reader, where
it identifies an otherwise obscure choice. Keep destructive confirmations
and their existing steps. Instructions in help or screen bodies are outside
this prompt cleanup.

## Validation

All 674 nc-game tests pass. The release build passes. Existing rendering
tests cover the changed labels; condition-aware staging tests retain their
original behavior checks. `git diff --check` is clean. The notebook link check
reports the same 24 pre-existing notes; all links added here resolve.

## Connections

- [Prompt standard](../../dev/ec-game-prompt-standard.md)
- [Damage notation and staging](tafp4e55-compact-crippled-ship-notation-and-condition-aware-detach-and-transfer.md)
- [Available ship-class prompts](knkaambn-show-only-available-ship-classes-in-staging-prompts.md)

### t112

The post-send reminder now points to the inbox: `In your inbox: Enter reads;
D cancels it before maintenance.` The recipient-list shortcut is still valid,
but there D opens the outgoing list rather than reading or cancelling an
individual message. Inbox OUT rows already support Enter to read and D to
cancel. No input behavior changes. The existing player manual already explains
both paths. All 682 nc-game tests and the release build pass. The existing
rendering test checks the reminder and spacing within 80 by 25.

[t113](0wc8ysbt-organize-inbox-into-reports-messages-and-sent.md#t113)
subsequently places outgoing mail in Sent and shortens the reminder to
`Inbox > Sent: Enter reads; D cancels before maintenance.`
