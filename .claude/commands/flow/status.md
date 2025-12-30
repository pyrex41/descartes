---
description: Show current flow workflow status across all phases
model: haiku
---

# Flow Status

Display the current state of the flow workflow, including active handoffs, SCUD tags, and suggested next steps.

## Process

### Step 1: Gather Information

Run these commands to collect state:

```bash
# Find recent handoffs
find thoughts/shared/handoffs -name "*.md" -type f 2>/dev/null | head -20

# List active SCUD tags
scud tags 2>/dev/null || echo "No SCUD tags found"

# Get git status
git branch --show-current
git status --porcelain | head -5
```

### Step 2: Analyze Handoffs

For each handoff found, read the frontmatter to determine:
- Phase (research/plan/implement)
- Status (complete/partial)
- Topic
- Timestamp
- Next suggested command

### Step 3: Check SCUD State

For each SCUD tag found:
```bash
scud stats --tag {tag}
```

### Step 4: Present Status

Display in this format:

```
Flow Workflow Status
═══════════════════════════════════════════════════

📍 Current Branch: {branch}
📝 Uncommitted Changes: {yes/no}

═══════════════════════════════════════════════════
Active Flows:
═══════════════════════════════════════════════════

🔬 Research Phase
   {If handoffs exist}
   └─ {topic} (handoff: {date})
      Research: {path}
      Status: Complete
      Next: /flow:plan {handoff-path}
   {If none}
   └─ No active research handoffs

📋 Planning Phase
   {If handoffs exist}
   └─ {topic} (handoff: {date})
      Plan: {path}
      SCUD Tag: {tag}
      Tasks: {N} in {M} waves
      Next: /flow:implement {tag}
   {If none}
   └─ No active planning handoffs

🔨 Implementation Phase
   {If active SCUD tags}
   └─ {tag}
      Progress: {done}/{total} ({percentage}%)
      Waves: {current}/{total}
      Status: {in-progress/blocked/complete}
      {If in-progress}
      Next: /flow:implement {tag}
      {If complete}
      Next: /cl:describe_pr
   {If none}
   └─ No active implementations

═══════════════════════════════════════════════════
Recent Handoffs:
═══════════════════════════════════════════════════

{List recent handoffs by date, most recent first}
- research/{date}_{topic}.md → plan (complete)
- plan/{date}_{topic}.md → implement (complete)
- implement/{date}_{topic}.md (complete)

═══════════════════════════════════════════════════
Quick Commands:
═══════════════════════════════════════════════════

Start new flow:
  /flow:research [topic]   - Begin research on a topic

Continue existing:
  /flow:resume [handoff]   - Resume from any handoff
  /flow:implement [tag]    - Continue SCUD implementation

Planning:
  /flow:plan [handoff]     - Create plan from research

Other:
  /cl:describe_pr          - Create PR description
  /scud:retrospective      - Capture learnings
```

## Compact Mode

If there are no active flows, show a simplified view:

```
Flow Workflow Status
═══════════════════════════════════════════════════

📍 Branch: {branch}

No active flows detected.

Start a new flow:
  /flow:research [topic]   - Research a topic
  /flow:plan [topic]       - Plan an implementation

Recent handoffs: {count} in thoughts/shared/handoffs/
```

## Important Notes

- Check both handoffs directory AND active SCUD tags
- Handoffs may be orphaned (no corresponding SCUD state)
- SCUD tags may exist without handoffs (created manually)
- Show the most actionable next step for each active item
- Use haiku model for efficiency - this is a status check, not deep analysis
