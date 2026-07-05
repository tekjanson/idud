# Contributing to idud

Thank you for contributing to idud! This document explains how to work effectively on the project.

## Before You Start

**Read first**:
- [README.md](./README.md) - Project vision and architecture
- [.github/copilot-instructions.md](./.github/copilot-instructions.md) - How AI systems should work in this codebase

## Development Setup

```bash
git clone https://github.com/yourusername/idud.git
cd idud
npm install
cp .env.example .env
npm run dev
```

## Test-Driven Development

**All code requires 100% UAT coverage.** No exceptions.

### Writing Tests

1. **Write tests first** from a user's perspective (not implementation details)
2. **Organize by workflow**: `tests/workflows/` (user actions), `tests/pipelines/` (data ingestion)
3. **Test end-to-end**: Include database operations, API calls, UI interactions

Example:

```typescript
describe('Workflow: User adds a new proof to a concept', () => {
  it('should extract proof metadata from URL', async () => {
    const url = 'https://github.com/org/repo/blob/main/README.md#architecture';
    const proof = await extractProofFromUrl(url);
    
    expect(proof).toEqual({
      source: url,
      hash: expect.any(String),
      extracted: expect.any(Date),
      type: 'README',
    });
  });

  it('should deduplicate existing proofs by hash', async () => {
    const proof1 = await addProof(conceptId, { url: 'https://...' });
    const proof2 = await addProof(conceptId, { url: 'https://...' }); // Same source
    
    expect(proof2).toEqual(proof1); // No duplicate created
  });

  it('should update concept relationships when new proofs arrive', async () => {
    // ...test reactive updates
  });
});
```

### Running Tests

```bash
# Run all tests
npm run test

# Run tests for a specific workflow
npm run test -- tests/workflows/add-proof.test.ts

# Run with coverage report
npm run test:coverage
```

Coverage must be 100%. This is enforced pre-commit.

## Extraction Pipelines

When adding new data sources, create a deterministic extraction pipeline, not an LLM agent.

### Example: Adding a New Document Source

```typescript
// src/pipelines/extract-docs.ts
import { extractFromUrl, parseMarkdown } from './utils';

export async function extractConceptsFromDocs(
  entity: string,
  docUrl: string,
): Promise<Concept[]> {
  const html = await fetch(docUrl).then(r => r.text());
  const sections = parseMarkdown(html);

  return sections.map(section => ({
    name: section.title,
    description: section.content,
    proof: {
      source: docUrl,
      hash: sha256(section.content),
      type: 'DOCUMENTATION',
    },
  }));
}
```

**When to use LLM agents**:
- Only if you can't deterministically extract the data
- Examples: Finding sections relevant to "architecture", classifying concept types
- Must have tests verifying output quality

**When NOT to use LLM agents**:
- Parsing standard formats (JSON, YAML, Markdown)
- Extracting known patterns (README sections, API signatures)
- Building indexes

## Code Style

- **TypeScript**: Strict mode enabled
- **Formatting**: Prettier (enforced pre-commit)
- **Linting**: ESLint (enforced pre-commit)

```bash
npm run lint
npm run lint:fix
```

## Commit Message Format

Use conventional commits with a reference to tests:

```
feat(core): Add proof deduplication by hash

- Reduces duplicate proofs when re-ingesting same source
- Uses SHA256 hash for immutable reference
- Tests: ✅ extractProofFromUrl, ✅ deduplicateProofs

Closes #123
```

## PR Review Checklist

Before submitting:
- [ ] 100% test coverage (run `npm run test:coverage`)
- [ ] All tests pass (`npm run test`)
- [ ] Linter passes (`npm run lint`)
- [ ] Updated relevant docs (README, architecture)
- [ ] Commit messages are clear and reference related tests

## Architecture Decisions

When proposing changes to the architecture:

1. **Create an ADR** (Architecture Decision Record) in `docs/adr/`
2. **Explain the problem**: What scenario does this solve?
3. **Consider token efficiency**: How does this impact AI token burn?
4. **Include examples**: Show how the new structure works
5. **Plan versioning**: How will this migrate existing data?

## Reporting Issues

Include:
- Actual behavior
- Expected behavior
- Steps to reproduce
- Environment (Node version, OS, database)
- Relevant test cases that fail

## Questions?

- 📖 Check [docs/](./docs) for detailed guides
- 💬 Start a [Discussion](https://github.com/yourusername/idud/discussions)
- 🐛 Open an [Issue](https://github.com/yourusername/idud/issues) for bugs
