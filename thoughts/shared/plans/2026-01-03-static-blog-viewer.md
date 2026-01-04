# Static Blog Viewer Implementation Plan

## Overview

Create a single self-contained HTML file that displays the 12 Descartes blog posts in a clean, minimal interface. Developer clones repo, clicks `index.html`, and reads documentation in the browser with zero build steps or dependencies.

## Current State Analysis

- **Blog posts location**: `/descartes/docs/blog/` (12 numbered MD files + README)
- **Existing web infrastructure**: None (Rust desktop GUI only)
- **Content**: Well-structured markdown with code blocks, tables, ASCII diagrams

### Blog Files:
1. `01-introduction-the-pi-philosophy.md`
2. `02-getting-started.md`
3. `03-cli-commands.md`
4. `04-providers-configuration.md`
5. `05-session-management.md`
6. `06-agent-types.md`
7. `07-flow-workflow.md`
8. `08-skills-system.md`
9. `09-gui-features.md`
10. `10-subagent-tracking.md`
11. `11-advanced-features.md`
12. `12-iterative-loops.md`

## Desired End State

A single `index.html` file in `descartes/docs/blog/` that:
- Opens directly in any browser via `file://` protocol
- Displays one blog post at a time (tab/accordion style)
- Has a navigation sidebar listing all 12 posts
- Uses minimal, clean styling (nikolai.fyi aesthetic)
- Contains zero external dependencies
- Requires zero build steps

### Verification:
- Open `descartes/docs/blog/index.html` in browser
- All 12 posts are navigable and readable
- Code blocks have syntax highlighting (CSS-only, monospace)
- Tables render correctly
- Works offline with `file://` protocol

## What We're NOT Doing

- No build system (webpack, vite, etc.)
- No external CDN dependencies
- No server required
- No markdown-to-HTML conversion at runtime
- No framework (React, Vue, etc.)
- No separate CSS files (all inline)

## Implementation Approach

Create a single HTML file with:
1. Inline CSS (~80 lines) for minimal styling
2. Inline JavaScript (~30 lines) for tab switching
3. Pre-rendered HTML content from all 12 blog posts
4. Semantic HTML structure with `<article>` sections

## Phase 1: Create Static Blog Viewer

### Overview
Build the complete `index.html` file with all content and styling.

### Changes Required:

#### 1.1 Create index.html

**File**: `descartes/docs/blog/index.html`

**Structure**:
```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Descartes Documentation</title>
  <style>
    /* Inline CSS - nikolai.fyi inspired */
  </style>
</head>
<body>
  <div class="container">
    <header>
      <h1>Descartes</h1>
      <p>A minimal, observable AI coding agent framework</p>
    </header>

    <nav>
      <!-- Navigation links for all 12 posts -->
      <a href="#" data-post="01">The Pi Philosophy</a>
      <a href="#" data-post="02">Getting Started</a>
      <!-- ... etc -->
    </nav>

    <main>
      <article id="post-01" class="post active">
        <!-- Pre-rendered HTML from 01-introduction-the-pi-philosophy.md -->
      </article>
      <article id="post-02" class="post">
        <!-- Pre-rendered HTML from 02-getting-started.md -->
      </article>
      <!-- ... all 12 posts -->
    </main>
  </div>

  <script>
    // ~30 lines for tab switching
  </script>
</body>
</html>
```

#### 1.2 CSS Styling (Inline)

**Design specs** (nikolai.fyi inspired):
- Max-width: 80ch for content
- Font: System font stack (sans-serif)
- Colors: White bg (#fff), dark text (#232323), light borders (#e9e9e9)
- Code blocks: Light gray background, monospace font
- Tables: Simple borders, alternating row colors
- Navigation: Sidebar on left (desktop) or top (mobile)
- Active post highlighted in nav

```css
* { box-sizing: border-box; margin: 0; padding: 0; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  line-height: 1.6;
  color: #232323;
  background: #fff;
}
.container {
  display: flex;
  max-width: 1200px;
  margin: 0 auto;
  padding: 2rem;
}
nav {
  width: 250px;
  flex-shrink: 0;
  border-right: 1px solid #e9e9e9;
  padding-right: 2rem;
}
nav a {
  display: block;
  padding: 0.5rem 0;
  color: #232323;
  text-decoration: none;
  border-bottom: 1px solid #e9e9e9;
}
nav a:hover, nav a.active {
  color: #000;
  font-weight: 600;
}
main {
  flex: 1;
  max-width: 80ch;
  padding-left: 2rem;
}
.post { display: none; }
.post.active { display: block; }
h1, h2, h3, h4 { margin: 1.5rem 0 0.5rem; }
h1 { font-size: 2rem; }
h2 { font-size: 1.5rem; }
p { margin: 1rem 0; }
pre {
  background: #f6f6f6;
  padding: 1rem;
  overflow-x: auto;
  border-radius: 4px;
}
code {
  font-family: "SF Mono", Monaco, Consolas, monospace;
  font-size: 0.9em;
}
table {
  width: 100%;
  border-collapse: collapse;
  margin: 1rem 0;
}
th, td {
  border: 1px solid #e9e9e9;
  padding: 0.5rem;
  text-align: left;
}
tr:nth-child(even) { background: #f9f9f9; }
hr { border: none; border-top: 1px solid #e9e9e9; margin: 2rem 0; }
@media (max-width: 768px) {
  .container { flex-direction: column; }
  nav { width: 100%; border-right: none; border-bottom: 1px solid #e9e9e9; padding: 0 0 1rem; margin-bottom: 1rem; }
  main { padding-left: 0; }
}
```

#### 1.3 JavaScript (Inline)

**Tab switching logic** (~30 lines):

```javascript
document.addEventListener('DOMContentLoaded', function() {
  const navLinks = document.querySelectorAll('nav a');
  const posts = document.querySelectorAll('.post');

  navLinks.forEach(link => {
    link.addEventListener('click', function(e) {
      e.preventDefault();
      const postId = this.dataset.post;

      // Update nav active state
      navLinks.forEach(l => l.classList.remove('active'));
      this.classList.add('active');

      // Show selected post, hide others
      posts.forEach(p => p.classList.remove('active'));
      document.getElementById('post-' + postId).classList.add('active');

      // Scroll to top
      window.scrollTo(0, 0);
    });
  });
});
```

#### 1.4 Content Conversion

Convert each of the 12 markdown files to HTML and embed in `<article>` sections:

- Headings: `#` → `<h1>`, `##` → `<h2>`, etc.
- Code blocks: ``` → `<pre><code>`
- Tables: Markdown tables → `<table>` HTML
- Lists: `-` → `<ul><li>`, `1.` → `<ol><li>`
- Links: `[text](url)` → `<a href="url">text</a>`
- Emphasis: `*text*` → `<em>`, `**text**` → `<strong>`
- Horizontal rules: `---` → `<hr>`

### Success Criteria:

#### Automated Verification:
- [ ] File exists: `ls descartes/docs/blog/index.html`
- [ ] Valid HTML: Can be parsed without errors
- [ ] Contains all 12 post sections: `grep -c 'class="post"' index.html` returns 12

#### Manual Verification:
- [ ] Open `descartes/docs/blog/index.html` directly in browser (file:// protocol)
- [ ] All 12 navigation links work and switch content
- [ ] Code blocks display with monospace font
- [ ] Tables render correctly with borders
- [ ] ASCII diagrams preserve formatting (in `<pre>` blocks)
- [ ] Responsive: works on narrow viewport (mobile simulation)
- [ ] Clean, readable typography matching nikolai.fyi aesthetic

---

## Testing Strategy

### Manual Testing Steps:
1. Open `index.html` in Chrome, Firefox, Safari via file:// protocol
2. Click through all 12 navigation links
3. Verify code blocks have proper formatting
4. Verify tables render with borders
5. Resize browser to test responsive layout
6. Check that ASCII architecture diagrams display correctly

## Performance Considerations

- Single file load (~100-150KB total including all content)
- No network requests after initial load
- Instant tab switching (CSS display toggle, no fetch)

## References

- Design inspiration: https://nikolai.fyi
- Blog source files: `descartes/docs/blog/*.md`
