# web-search

Search the web for information using DuckDuckGo's instant answer API.

## Usage

```bash
web-search "your search query" [max_results]
```

## Arguments

| Argument | Required | Default | Description |
|----------|----------|---------|-------------|
| query | Yes | - | The search query |
| max_results | No | 5 | Maximum number of related topics to show |

## Examples

```bash
# Search for programming concepts
web-search "rust async await"

# Get fewer results
web-search "python list comprehension" 3

# Search for definitions
web-search "what is kubernetes"
```

## Output Format

The skill returns:
1. A summary/abstract if available
2. Related topics as a bulleted list
3. Source attribution

Example output:
```
Searching for: rust async await
---
Summary: Async/await is Rust's built-in tool for writing asynchronous functions...

Related topics:
- Rust async-await - A feature that allows writing asynchronous code...
- Tokio runtime - The most popular async runtime for Rust...
- Futures in Rust - The foundational trait for async programming...

Source: DuckDuckGo Instant Answers
```

## Dependencies

- `curl` - For HTTP requests
- `jq` - For JSON parsing

Both are commonly available on most systems.

## API

This skill uses the free DuckDuckGo Instant Answer API which:
- Requires no API key
- Has no rate limits for reasonable usage
- Returns structured data including abstracts and related topics

## Limitations

- Only returns instant answer data, not full web search results
- Best for factual queries, definitions, and well-known topics
- For comprehensive web search, consider integrating a paid API (Google, Bing, SerpAPI)
