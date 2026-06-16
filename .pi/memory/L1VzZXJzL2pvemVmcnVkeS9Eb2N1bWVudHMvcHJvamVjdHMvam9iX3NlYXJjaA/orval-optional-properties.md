---
type: lesson
tags: [frontend, orval, typescript, api-generation]
created: 2026-06-16
updated: 2026-06-16
---

Orval honors the OpenAPI `required` array. Non-`Option<T>` Rust fields with no `#[serde(default)]` are emitted as non-optional (`a: string`), and `Option<T>` / `#[schema(nullable)]` fields become `?: T | null`. Adding `#[serde(default)]` removes the field from `required` unless you also mark it `#[schema(required = true)]`.
