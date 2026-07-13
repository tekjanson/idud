# Training Discovery Module

## Overview

The Training Discovery module discovers and analyzes GitHub repositories suitable for training validation. It identifies public repositories with active communities, recent development activity, and sufficient complexity to serve as meaningful training candidates for the contract ledger system.

## Architecture

### Core Components

1. **Discovery Engine** (`discover_training_repos()`)
   - Queries GitHub's GraphQL API for repository candidates
   - Filters by: stars (50+), recent updates (within 30 days), active issues & PRs
   - Returns structured candidate data for analysis

2. **Issue Analyzer** (`fetch_issue_and_linked_pr()`)
   - Retrieves detailed issue information by ID
   - Extracts linked pull requests via GitHub timeline events
   - Collects modified files from linked PRs
   - Provides rich context for contract discovery

3. **API Integration**
   - Uses public GitHub GraphQL API (no authentication required)
   - Implements rate limit handling and exponential backoff
   - Graceful degradation on API errors

### Selection Criteria

**Repository Must Have:**
- ✅ 50+ stars (community engagement indicator)
- ✅ Recent updates (within 30 days)
- ✅ Active open issues
- ✅ Active open PRs
- ✅ Public visibility

**Preferred Characteristics:**
- Multiple programming languages (diverse signature patterns)
- Established contributor base
- Active maintenance
- Well-structured codebase

## API Endpoints

### Discover Repositories
```
GET /api/training/discover?limit=100
```

**Query Parameters:**
- `limit` (optional, default: 100, max: 1000) - Number of candidates to return

**Response:**
```json
{
  "success": true,
  "candidates": [
    {
      "url": "https://github.com/owner/repo",
      "name": "repo",
      "owner": "owner",
      "stars": 150,
      "language": "Rust",
      "issue_count": 24,
      "pr_count": 8,
      "last_issue_id": "MDU6SXNzdWUxMjM0NTY3",
      "last_pr_id": "MDU6UHVsbFJlcXVlc3Q3ODkwMTIz",
      "updated_at": "2025-01-15T10:30:00Z"
    }
  ],
  "count": 50
}
```

### Fetch Issue with Linked PR
```
GET /api/training/issue/{repo_owner}/{repo_name}/{issue_id}
```

**Path Parameters:**
- `repo_owner` - Repository owner username
- `repo_name` - Repository name
- `issue_id` - Numeric issue ID

**Response:**
```json
{
  "success": true,
  "data": {
    "issue_title": "Add support for async operations",
    "issue_body": "We need to support async/await patterns...",
    "issue_number": 42,
    "pr_title": "Implement async support",
    "pr_number": 55,
    "pr_files": [
      "src/async/mod.rs",
      "src/runtime/executor.rs",
      "tests/async_tests.rs"
    ]
  }
}
```

## Rate Limiting

GitHub's GraphQL API has rate limits:
- **Unauthenticated**: 60 requests per hour
- **Authenticated**: 5,000 requests per hour

The module handles rate limits gracefully:

1. **Detection**: Checks HTTP 403 responses and rate limit headers
2. **Response**: Returns `RateLimited` error when limit exceeded
3. **Backoff**: Clients should implement exponential backoff when rate limited
4. **Monitoring**: Logs remaining quota and reset times

### Rate Limit Headers
```
x-ratelimit-limit: 60
x-ratelimit-remaining: 45
x-ratelimit-reset: 1705326000
```

## Usage Example

### Discovering Candidates

```rust
use idud::discover_training_repos;

#[tokio::main]
async fn main() -> Result<()> {
    let candidates = discover_training_repos(50).await?;
    
    for candidate in candidates {
        println!("🔍 {} ({} stars)", candidate.name, candidate.stars);
        println!("   Issues: {}, PRs: {}", 
                 candidate.issue_count, candidate.pr_count);
    }
    
    Ok(())
}
```

### Fetching Issue Details

```rust
use idud::fetch_issue_and_linked_pr;

#[tokio::main]
async fn main() -> Result<()> {
    let issue = fetch_issue_and_linked_pr(
        "torvalds",
        "linux",
        12345
    ).await?;
    
    println!("Issue: {}", issue.issue_title);
    println!("Linked PR: #{}", issue.pr_number.unwrap_or(0));
    println!("Files changed: {:?}", issue.pr_files);
    
    Ok(())
}
```

## Error Handling

### Error Types

| Error | Status Code | Meaning |
|-------|-------------|---------|
| `HttpError` | 500 | Network or HTTP layer failure |
| `ApiError` | 500 | GitHub API returned error |
| `RateLimited` | 429 | Rate limit exceeded |
| `RepoNotFound` | 404 | Repository or issue not found |
| `JsonError` | 500 | Response parsing failed |

### Recovery Strategies

**Rate Limit (429):**
- Wait `reset_at` timestamp + 1 second
- Implement exponential backoff: 2s, 4s, 8s, etc.
- Reduce batch size for next request

**Network Error:**
- Retry with exponential backoff (max 3 attempts)
- Increase timeout for slow networks

**API Error:**
- Log full error details for debugging
- Return to user with clear message
- Do not retry on validation errors

## Performance Characteristics

- **Discovery Query**: ~2-3 seconds per 100 repos (network bound)
- **Issue Fetch**: ~1-2 seconds per issue (includes PR file list)
- **Caching**: Recommendations for 1-hour cache on discovery results
- **Parallel Requests**: Safe to batch multiple issue fetches with tokio

## Design Decisions

### Why GraphQL?

- **Efficiency**: Single query fetches repo + issue + PR data
- **Control**: Request only needed fields
- **Rate Limit**: GraphQL queries count as 1 against rate limit

### Why 50 Stars Minimum?

- Indicates established community
- Reduces noise from hobby projects
- Correlates with code quality
- Provides meaningful training signals

### Why Recent Updates?

- Active projects = active contracts
- Dead projects = stale patterns
- 30-day window balances freshness with candidate pool size

## Future Enhancements

1. **Authentication**: Support GitHub token for higher rate limits
2. **Filtering**: Add language, topic, and license filters
3. **Ranking**: Score candidates by activity, contributor count
4. **Caching**: Local cache of discoveries with TTL
5. **Webhooks**: Listen for repo updates instead of polling
6. **Metrics**: Track discovery patterns and success rates

## Testing

### Unit Tests

```bash
cargo test training::discovery::tests
```

### Integration Tests

Test against public GitHub repositories:
- torvalds/linux (kernel, C)
- rust-lang/rust (compiler, Rust)
- kubernetes/kubernetes (orchestration, Go)

### Rate Limit Testing

Use HTTP mock server to simulate rate limit responses.

## Monitoring

### Logs

```
2025-01-15T10:30:00.123Z INFO GitHub API rate limit: 45/60 remaining, resets at 2025-01-15T11:00:00Z
2025-01-15T10:30:05.456Z WARN Repository discovery returned 0 candidates (query may be too restrictive)
```

### Metrics

- Requests per hour
- Rate limit hit frequency
- Average response time
- Candidate quality score
- Linked PR success rate

## Related Modules

- **Contract Ledger**: Stores discovered contracts from analyzed repos
- **Pipelines**: Ingests selected candidate repos
- **UI**: Visualizes discovery results and training progress

## References

- [GitHub GraphQL API](https://docs.github.com/en/graphql)
- [Rate Limiting](https://docs.github.com/en/graphql/overview/rate-limits-and-node-limits-in-the-graphql-api)
- [REST vs GraphQL](https://docs.github.com/en/graphql/guides/migrating-from-rest-to-graphql)
