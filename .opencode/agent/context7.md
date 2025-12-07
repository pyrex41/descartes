---
description: context7 CLI - MCP server wrapper
mode: all
tools:
  bash: true
---
# context7 CLI

This project has the context7 CLI available for interacting with the context7 MCP server.

## Available Commands

- `context7 list` - Show all available tools
- `context7 <tool> [--arg value]` - Call a specific tool
- `context7 scaffold` - Install skills for AI agents

## Tools

### resolve-library-id
Resolves a package/product name to a Context7-compatible library ID and returns a list of matching libraries.

You MUST call this function before 'get-library-docs' to obtain a valid Context7-compatible library ID UNLESS the user explicitly provides a library ID in the format '/org/project' or '/org/project/version' in their query.

Selection Process:
1. Analyze the query to understand what library/package the user is looking for
2. Return the most relevant match based on:
- Name similarity to the query (exact matches prioritized)
- Description relevance to the query's intent
- Documentation coverage (prioritize libraries with higher Code Snippet counts)
- Source reputation (consider libraries with High or Medium reputation more authoritative)
- Benchmark Score: Quality indicator (100 is the highest score)

Response Format:
- Return the selected library ID in a clearly marked section
- Provide a brief explanation for why this library was chosen
- If multiple good matches exist, acknowledge this but proceed with the most relevant one
- If no good matches exist, clearly state this and suggest query refinements

For ambiguous queries, request clarification before proceeding with a best-guess match.

Arguments: --libraryName

### get-library-docs
Fetches up-to-date documentation for a library. You must call 'resolve-library-id' first to obtain the exact Context7-compatible library ID required to use this tool, UNLESS the user explicitly provides a library ID in the format '/org/project' or '/org/project/version' in their query. Use mode='code' (default) for API references and code examples, or mode='info' for conceptual guides, narrative information, and architectural questions.

Arguments: --context7CompatibleLibraryID --mode --topic --page

## Examples

```bash
context7 resolve-library-id --libraryName "value"
```

```bash
context7 get-library-docs --context7CompatibleLibraryID "value" --mode "value" --topic "value" --page "value"
```

