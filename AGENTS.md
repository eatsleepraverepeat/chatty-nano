# Commit Message Guidelines

This document provides instructions for generating commit messages for the jetson-stt project.

## Commit Message Pattern

All commit messages must follow this pattern:

```
UPD|ADD|DEL: <short message describing changes>
```

## Commit Types

### UPD (Update)
Use `UPD` when modifying existing code, fixing bugs, refactoring, or making changes to existing functionality.

**Examples:**
- `UPD: Fix audio buffer overflow issue`
- `UPD: Improve speech recognition accuracy`
- `UPD: Refactor model loading logic`
- `UPD: Update dependencies to latest versions`
- `UPD: Optimize memory usage in inference pipeline`

### ADD (Add)
Use `ADD` when introducing new features, files, or functionality that didn't exist before.

**Examples:**
- `ADD: Implement real-time transcription feature`
- `ADD: Add support for additional audio formats`
- `ADD: Create configuration file parser`
- `ADD: Add unit tests for audio processor`
- `ADD: Implement WebSocket server for streaming`

### DEL (Delete)
Use `DEL` when removing code, files, or features that are no longer needed.

**Examples:**
- `DEL: Remove deprecated audio processing module`
- `DEL: Delete unused configuration options`
- `DEL: Remove legacy model files`
- `DEL: Clean up obsolete test cases`
- `DEL: Remove temporary debugging code`

## Best Practices

1. **Keep it concise**: The short message should be clear and to the point (ideally under 50 characters)
2. **Use imperative mood**: Write as if you're commanding someone (e.g., "Fix bug" not "Fixed bug")
3. **Be specific**: Describe what changed and why, not just that something changed
4. **One commit, one change**: Each commit should address a single logical change
5. **Avoid combining types**: Don't mix UPD, ADD, and DEL in a single commit message

## Examples of Good Commit Messages

```bash
ADD: Implement VAD (Voice Activity Detection) module
UPD: Fix memory leak in audio buffer handling
DEL: Remove unused dependency on numpy 1.19
ADD: Add support for custom vocabulary lists
UPD: Improve error handling in model initialization
```

## Examples of Bad Commit Messages

```bash
# Too vague
UPD: stuff

# Combines multiple types
UPD/ADD: Fix bug and add new feature

# Doesn't follow pattern
Fixed the audio issue

# Too long
ADD: Implement a comprehensive solution for handling multiple audio input streams with various sample rates and formats
```

## Enforcement

These commit message conventions help maintain a clean, readable git history and make it easier to track changes and understand the evolution of the project. Please follow these guidelines for all commits to this repository.