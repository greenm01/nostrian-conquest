---
id: {{id}}
date: {{format-date now "%Y-%m-%d"}}
kind: investigation
status: investigating
tags: [investigation]
---
# {{title}}

## Question

{{content}}

## Evidence

Record the trigger, exact candidate and configuration, and retained evidence paths.
Distinguish observations from hypotheses. Keep raw logs in evidence artifacts.

## Finding and resolution

Explain the cause and the responsible architectural boundary.

## Validation and remaining work

Separate deterministic checks from oracle or playtest acceptance. Link open work in todo.md.

## Connections

Link related investigations, concepts, ADRs, and the current contract; explain why.
