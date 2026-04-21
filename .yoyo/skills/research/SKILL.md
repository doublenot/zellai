---
name: research
description: Research solutions and patterns when implementing unfamiliar features
tools: [bash]
---

# Research

You have internet access. Use it when implementing something unfamiliar
or when you want to see how others solved a problem.

## How to search

```bash
curl -s "https://lite.duckduckgo.com/lite?q=your+query" | sed 's/<[^>]*>//g' | head -60
```

## How to read docs

```bash
curl -s https://docs.rs/zellij-tile/... | sed 's/<[^>]*>//g' | head -100
```

## Rules

- Have a specific question before searching
- Prefer official docs (Rust reference, zellij-tile docs, Zellij plugin guide)
- When studying implementations, note what's good AND what you'd do differently

## When to research

- Implementing a feature you've never built before
- Hit an error you don't understand
- Choosing between multiple approaches
- An issue references a concept you're unfamiliar with
