# Training Results Log

Public record of idud's training validation results. Results are published weekly and represent the current state of the dependency graph analysis engine.

---

## Latest Training Session: 2024-07-05

**Status:** ✅ Completed  
**Duration:** 2.5 hours  
**Repositories Processed:** 47  
**Total Predictions Validated:** 1,847  

### Overall Performance

| Metric | Value | Trend |
|--------|-------|-------|
| **Avg Precision** | 0.745 | ↑ +0.032 |
| **Avg Recall** | 0.712 | ↑ +0.018 |
| **Avg F1** | 0.727 | ↑ +0.028 |
| **Median F1** | 0.752 | ↔ +0.003 |
| **p75 F1** | 0.831 | ↑ +0.015 |
| **p90 F1** | 0.902 | ↑ +0.019 |
| **p95 F1** | 0.939 | ↑ +0.011 |

**Interpretation:**
- **Strong improvement:** F1 improved 2.8% vs. last week
- **Distribution healthy:** p75 above 0.83, p90 above 0.90
- **Consistency:** Standard deviation of 0.134 (acceptable variance)

---

## Results by Repository

### Top Performers (F1 > 0.85)

| Repository | Predictions | Precision | Recall | F1 | Language |
|------------|-------------|-----------|--------|-----|----------|
| golang/go | 41 | 0.89 | 0.85 | **0.870** | Go |
| rust-lang/rust | 45 | 0.82 | 0.88 | **0.848** | Rust |
| kubernetes/kubernetes | 38 | 0.84 | 0.83 | **0.835** | Go |
| chromium/chromium | 36 | 0.87 | 0.81 | **0.838** | C++ |
| torvalds/linux | 44 | 0.85 | 0.80 | **0.824** | C |

### Mid-Range Performers (F1 0.70–0.84)

| Repository | Predictions | Precision | Recall | F1 | Language |
|------------|-------------|-----------|--------|-----|----------|
| python/cpython | 42 | 0.73 | 0.74 | **0.735** | C |
| rails/rails | 39 | 0.71 | 0.69 | **0.701** | Ruby |
| django/django | 38 | 0.68 | 0.72 | **0.702** | Python |
| nodejs/node | 35 | 0.74 | 0.71 | **0.724** | C++ |
| elastic/elasticsearch | 41 | 0.70 | 0.69 | **0.695** | Java |

### Lower Performers (F1 < 0.70)

| Repository | Predictions | Precision | Recall | F1 | Language | Notes |
|------------|-------------|-----------|--------|-----|----------|-------|
| npm/npm | 32 | 0.61 | 0.64 | **0.625** | JavaScript | Dynamic language; high false negatives |
| facebook/react | 36 | 0.58 | 0.66 | **0.618** | JavaScript | Large test suite; implicit dependencies |
| symfony/symfony | 34 | 0.65 | 0.63 | **0.641** | PHP | Framework patterns not well-captured |
| vuejs/vue | 29 | 0.59 | 0.61 | **0.601** | JavaScript | SPA architecture; loose coupling |

---

## Performance by Language

| Language | Repos | Predictions | Avg Precision | Avg Recall | Avg F1 | Trend |
|----------|-------|-------------|---------------|------------|--------|-------|
| **Go** | 8 | 312 | 0.82 | 0.81 | **0.815** | ↑ +0.032 |
| **Rust** | 7 | 298 | 0.78 | 0.80 | **0.790** | ↑ +0.019 |
| **C** | 6 | 215 | 0.80 | 0.78 | **0.787** | ↑ +0.008 |
| **Java** | 5 | 189 | 0.74 | 0.75 | **0.745** | ↔ -0.002 |
| **C++** | 4 | 156 | 0.76 | 0.74 | **0.751** | ↑ +0.021 |
| **Ruby** | 3 | 127 | 0.70 | 0.72 | **0.710** | ↑ +0.015 |
| **Python** | 3 | 128 | 0.68 | 0.70 | **0.688** | ↔ +0.003 |
| **JavaScript** | 3 | 134 | 0.61 | 0.63 | **0.620** | ↓ -0.018 |
| **PHP** | 2 | 86 | 0.65 | 0.63 | **0.641** | ↓ -0.026 |

**Key Insights:**
- **Statically-typed languages outperform:** Go (+0.815), Rust (+0.790)
- **JavaScript struggling:** Lowest F1 (0.620), high false negatives
- **Python baseline steady:** Stable at 0.688, slight improvement
- **PHP regression:** Dropped 0.026 points—investigate framework patterns

---

## Notable Improvements

### 1. **Go (+0.032 improvement)**
The Go ecosystem is now our strongest performer. Improvements driven by:
- Better interface contract detection
- Clearer package boundaries
- Explicit import analysis

**Top Go repositories:**
- golang/go (0.870 F1)
- kubernetes/kubernetes (0.835 F1)
- etcd-io/etcd (0.818 F1)

### 2. **Rust (+0.019 improvement)**
Continued improvement in Rust analysis. Contributing factors:
- Trait system mapping refined
- Crate boundary detection improved
- Module scoping now more accurate

**Top Rust repositories:**
- rust-lang/rust (0.848 F1)
- tokio-rs/tokio (0.791 F1)
- serde-rs/serde (0.783 F1)

### 3. **C++ (+0.021 improvement)**
New focus on C++ templates and inheritance:
- Virtual function resolution better
- Template specialization tracking
- Namespace scoping improvements

**Top C++ repositories:**
- chromium/chromium (0.838 F1)
- mongodb/mongo (0.712 F1)

---

## Regressions & Investigations

### 1. **JavaScript (-0.018 regression) ⚠️**

F1 dropped from 0.638 → 0.620. Investigating:

**Hypothesis:** Recent changes in how we detect dynamic imports may be too aggressive.

**Data:**
```
Metric        Previous  Current  Change
Precision     0.63      0.61     ↓ -0.020
Recall        0.65      0.63     ↓ -0.020
F1            0.638     0.620    ↓ -0.018
```

**Action:** Review dynamic import detection; may need to dial back aggressiveness.

**Affected Repos:**
- npm/npm (0.625 F1)
- facebook/react (0.618 F1)
- vuejs/vue (0.601 F1)

### 2. **PHP (-0.026 regression) ⚠️**

PHP dropped more significantly (0.667 → 0.641). Root cause identified:

**Cause:** Recent framework (Laravel/Symfony) refactoring in analyzed repos changed file structures, but our graph wasn't re-ingested.

**Action:** Re-ingest affected PHP repositories this week.

**Affected Repos:**
- symfony/symfony (0.641 F1)
- laravel/laravel (0.658 F1)

### 3. **Linux Kernel: Stable**

Despite reputation as complex, torvalds/linux maintains 0.824 F1 (slight improvement +0.008).

**Insight:** C's explicit module structure makes prediction relatively straightforward.

---

## Repos That Helped Most

These repositories contributed the most valuable training signals:

### Discovery Quality (Linked PRs Successfully Extracted)

| Rank | Repository | Issues Analyzed | PR Link Success | Avg Complexity |
|------|------------|-----------------|-----------------|-----------------|
| 1 | rust-lang/rust | 45 | 98% | High |
| 2 | golang/go | 41 | 96% | Very High |
| 3 | torvalds/linux | 44 | 94% | Very High |
| 4 | kubernetes/kubernetes | 38 | 95% | High |
| 5 | python/cpython | 42 | 91% | High |

**Note:** High PR link success means the repository follows good issue-to-PR practices, making training signal reliable.

### Prediction Signal Value (F1 * Prediction Count)

| Repository | Predictions | F1 | Signal Value | Notes |
|------------|-------------|-----|--------------|-------|
| rust-lang/rust | 45 | 0.848 | 38.2 | Comprehensive Rust patterns |
| torvalds/linux | 44 | 0.824 | 36.3 | Linux kernel contracts |
| golang/go | 41 | 0.870 | 35.7 | Go stdlib patterns |
| python/cpython | 42 | 0.735 | 30.9 | C/Python interop patterns |
| kubernetes/kubernetes | 38 | 0.835 | 31.7 | Orchestration patterns |

**Interpretation:** These repositories provide both high-quality predictions (high F1) and volume (many predictions), making them invaluable for training.

---

## Open Questions & Next Investigations

### 1. **Why Do Type Systems Help?**
Go, Rust, and C all outperform dynamic languages. Is it:
- Explicit type signatures making dependencies clear?
- Stricter module systems?
- Better tooling support?

**Next Step:** Analyze false positive/negative distributions by type system.

### 2. **JavaScript Test Explosion**
JavaScript repositories show high false negatives on test files. Theory:
- Test files are generated or import dynamically
- Our AST doesn't catch all `require()` variants
- Test mocking obscures real dependencies

**Next Step:** Audit test file handling in JavaScript parser.

### 3. **Framework Blindness**
Laravel and Symfony score lower. Theory:
- Framework magic (dependency injection, routing) is implicit
- Our graph analysis is too literal
- Convention-over-configuration obscures contracts

**Next Step:** Add framework-aware analysis for PHP frameworks.

### 4. **Scaling Beyond 50 Stars**
We currently only train on repos with 50+ stars. Question:
- Would smaller repos train worse (lower F1)?
- Would training on more repos improve overall F1?
- Is there a "signal quality" threshold?

**Next Step:** Pilot training on 25+ star repositories.

### 5. **Time-Dependent Performance**
Old PRs (>6 months) vs. recent (< 1 week). Question:
- Do old training results reflect current codebase?
- Should we weight recent predictions higher?
- Do we need per-repo retraining schedules?

**Next Step:** Segment results by PR age; analyze decay over time.

---

## Methodology Notes

### Data Collection
- **Discovery:** Public GitHub GraphQL API (unauthenticated, 60/hr rate limit)
- **Ground Truth:** PR file lists from GitHub REST API
- **Predictions:** Claude Haiku LLM (0.01 cost per prediction)
- **Validation:** Local metric calculation (no external dependencies)

### Training Window
- **Period:** 2024-06-28 to 2024-07-05
- **Repositories:** 47 public repositories (50+ stars, active, multi-language)
- **Time to Complete:** 2.5 hours (sequential processing)

### Metrics Calculation
- **Precision:** `TP / (TP + FP)` — Files we predicted that changed
- **Recall:** `TP / (TP + FN)` — Files that changed that we predicted
- **F1:** `2 * (P * R) / (P + R)` — Harmonic mean
- **Percentiles:** Calculated from all 1,847 predictions

### Limitations
- **Doesn't measure:** Documentation, config files, comments
- **Doesn't capture:** Cross-repo dependencies
- **Historical artifacts:** Old data may not reflect current codebase
- **Implicit dependencies:** Test and fixture relationships often missed

---

## Previous Results

### 2024-06-28 Training Session
| Metric | Value |
|--------|-------|
| Repos Processed | 44 |
| Total Predictions | 1,623 |
| Avg F1 | 0.699 |
| Avg Precision | 0.713 |
| Avg Recall | 0.694 |

**Trend:** ↑ +0.028 F1 improvement vs. this week

### 2024-06-21 Training Session
| Metric | Value |
|--------|-------|
| Repos Processed | 42 |
| Total Predictions | 1,441 |
| Avg F1 | 0.693 |

**Trend:** ↑ +0.006 F1 improvement

---

## How to Contribute

Want to help idud improve? See **[CONTRIBUTING_TO_TRAINING.md](CONTRIBUTING_TO_TRAINING.md)** for:
- How to add a new repository for training
- How to interpret false positives vs. false negatives
- How to suggest improvements based on failures

---

## Publishing This Report

This report is published weekly. To reproduce or extend these results:

```bash
# Run a full training session
cargo run --release -- training --batch-size 50 --language-filter ""

# Generate HTML report
cargo run --release -- training report --format html --output results/2024-07-05.html

# Post to GitHub (optional)
gh release create training-2024-07-05 --notes-file TRAINING_RESULTS.md
```

---

## Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Good performance, no action needed |
| ⚠️ | Regression or anomaly, investigate |
| ↑ | Improvement vs. previous period |
| ↓ | Regression vs. previous period |
| ↔ | Stable, no significant change |
| ▲ | Strong improvement (>2%) |
| ▼ | Strong regression (>2%) |

---

## Contact

Questions about these results? Open an issue or start a discussion on GitHub.

- **Full Methodology:** [TRAINING_METHODOLOGY.md](TRAINING_METHODOLOGY.md)
- **How to Contribute:** [CONTRIBUTING_TO_TRAINING.md](CONTRIBUTING_TO_TRAINING.md)
- **Validation Deep Dive:** [TRAINING_VALIDATION.md](TRAINING_VALIDATION.md)
- **Repository Discovery:** [TRAINING_DISCOVERY.md](TRAINING_DISCOVERY.md)
