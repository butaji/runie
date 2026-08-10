# Persona: The Academic Researcher

**Primary Use Case:** Computational research, reproducible experiments, code documentation for papers, literature review, collaborative research workflows, and thesis/dissertation writing with code components.

---

## 1. Persona Profile

### Background

Dr. Sarah Okonkwo, 38, Associate Professor of Computational Biology at a research university. She leads a lab of 6 graduate students and 2 postdocs. Her research combines machine learning with genomics data analysis, requiring both wet-lab work and substantial computational pipelines. She's published 40+ papers, manages multiple grants, and serves as a reviewer for several journals.

**Current stack:**
- Python, R, and occasionally Julia for analysis
- Snakemake for workflow management
- Git/GitHub for version control
- LaTeX/Overleaf for paper writing
- Jupyter notebooks for exploration and teaching
- SLURM cluster for high-performance computing
- Zotero for reference management
- High-performance workstations + remote HPC access

### Expertise Level

**Advanced expert with deep domain knowledge, moderate tooling sophistication.** Sarah is an expert in her research domain (computational biology) and highly proficient with version control, but she's not a software engineer by training. She values correctness and reproducibility over clever optimizations. Her students look to her for both domain expertise and coding practices.

### Work Style

- **Documentation-first mindset** — If it's not documented, it didn't happen
- **Reproducibility obsession** — Methods sections demand exact replication steps
- **Collaborative by default** — Papers involve multiple co-authors, code sharing with lab members
- **Publication cycles** — Intense writing phases punctuated by focused coding
- **Teaching alongside research** — Trains students in good coding practices
- **Grant-driven milestones** — Specific deliverables tied to funding periods
- **Publication-quality outputs** — Figures, tables, and code that pass peer review

---

## 2. Goals and Motivations

### Primary Goals

1. **Ensure reproducibility** — Every experiment must be replicable by herself, students, and external researchers
2. **Write publication-ready code** — Code that passes peer review scrutiny and can be cited
3. **Manage complex experiments** — Track parameter combinations, data versions, and results
4. **Collaborate effectively** — Share code and methods with co-authors without confusion
5. **Train students** — Instill good coding practices in her lab
6. **Maximize research throughput** — Get to insights faster without sacrificing rigor

### What Motivates Sarah

- **Scientific integrity** — Her reputation depends on reproducible, verifiable results
- **Efficiency gains** — Tools that accelerate research without introducing errors
- **Knowledge transfer** — Teaching others and building on previous work
- **Publication success** — Getting papers accepted with minimal revision cycles
- **Grant productivity** — Demonstrating progress to funding agencies
- **Student development** — Growing the next generation of rigorous researchers

### Secondary Goals

- Reduce technical debt in research code
- Build reusable components across projects
- Stay current with computational methods in her field
- Balance exploration with systematic experimentation
- Maintain work-life boundaries during intense writing periods

---

## 3. Pain Points with Current Tools

### The Reproducibility Crisis

Research reproducibility is a critical concern, and current AI tools often undermine it:

- **AI-generated code has no provenance** — When a reviewer asks "how was this parameter chosen?", Sarah can't trace it back to an AI suggestion
- **Version control gaps** — AI often modifies multiple files without clear commit boundaries
- **Environment specification is an afterthought** — Dependencies captured incompletely
- **Random seed problems** — ML experiments produce different results on rerun

> "I need to be able to say 'this figure was generated with commit X, using parameters Y, on data Z.' AI tools make that impossible."

### Documentation Burden

Scientific code requires extensive documentation that AI tools treat as optional:

- **Docstrings are often missing or wrong** — AI generates plausible but incorrect parameter descriptions
- **Type hints are incomplete** — Mixed types not annotated, edge cases undocumented
- **README files lack essential details** — Installation instructions that assume Linux, missing data requirements
- **Examples don't match API changes** — Stale examples break when code evolves

### Citation and Attribution Challenges

Academic work requires clear attribution:

- **Code provenance unclear** — If AI generated 60% of a function, how should it be cited?
- **License compatibility uncertain** — AI suggestions may inadvertently incorporate copyrighted patterns
- **Collaborator attribution** — Lab members who wrote prompts vs. AI that generated code

### Exploration vs. Exploitation Tension

Research requires balancing exploration with systematic analysis:

- **AI encourages quick fixes** — "Here's a working solution" without exploring alternatives
- **Missing the "why"** — Code works but Sarah doesn't understand the algorithm's assumptions
- **Black-box ML models** — AI-generated models are hard to explain in papers
- **Literature integration lacking** — Can't naturally connect suggestions to related work

### Review and Verification Fatigue

Given the 66% "almost right" frustration rate identified in UX research, Sarah experiences:

- **Debugging AI-generated pipelines** — Takes longer than writing from scratch
- **Parameter validation** — AI suggestions require cross-checking against domain knowledge
- **Edge case discovery** — AI often misses corner cases specific to scientific data
- **Statistical rigor checks** — Generated code may violate statistical assumptions

---

## 4. What Would Delight This User

### Reproducibility Guarantees

Sarah would be thrilled with tools that make reproducibility effortless:

- **Automatic experiment snapshots** — Every AI-assisted change creates a git commit with meaningful message
- **Environment fingerprinting** — Complete dependency specification (Python, system libs, GPU drivers)
- **Parameter provenance tracking** — Know exactly which run produced which result
- **Deterministic output guarantees** — Same inputs always produce same outputs
- **Reproducible container support** — Integration with Docker/Singularity for complete isolation

### Publication-Ready Code Generation

Tools that understand academic requirements:

- **Scientific docstring templates** — NumPy/SciPy style with parameter descriptions, return types, examples
- **Type hints as first-class citizens** — Full type annotation including generics, unions
- **README generation from code** — Auto-generate installation, usage, and examples sections
- **DOI-ready repository metadata** — CITATION.cff generation, license verification
- **Figure caption generation** — Describe what the code does in publication language

### Transparent Reasoning

Following the Unix philosophy of transparency over magic:

- **Show me the alternative** — "I chose algorithm X because Y; alternatives were Z"
- **Cite sources when possible** — "This pattern follows the approach in [paper reference]"
- **Explain statistical choices** — "Using Mann-Whitney U test because [assumption check]"
- **Acknowledge uncertainty** — "I'm 80% confident this will work; here's what could go wrong"

### Collaboration-First Design

Multi-author research demands excellent collaboration:

- **Co-author attribution** — Track who requested what changes
- **Comment-based discussions** — Inline comments like Google Docs for code review
- **Review workflow integration** — GitHub PR-style review for research code
- **Merge conflict transparency** — Clear visualization when combining changes
- **Access control** — Clear permissions for students, collaborators, reviewers

### Tool Integration with Academic Workflow

Tools that fit into established pipelines:

- **Zotero/EndNote integration** — Automatically cite papers referenced in code comments
- **Overleaf/LaTeX compatibility** — Insert code snippets that compile correctly
- **Jupyter notebook first-class** — Native support for notebook format, not just scripts
- **Snakemake/Nextflow awareness** — Understand workflow management systems
- **HPC/slurm awareness** — Code that actually works on remote clusters

---

## 5. Specific UI/UX Recommendations for Runie

### 5.1 Reproducibility Dashboard

**Design a dedicated panel showing experiment provenance:**

```
┌─────────────────────────────────────────────────────────────────┐
│ EXPERIMENT: variant_calling_run_003                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Git Commit:    a3f8c21 "Add variant calling pipeline"          │
│  Timestamp:     2026-07-15 14:32:07 UTC                        │
│  Parameters:    min_af=0.01, qual_cutoff=20                     │
│  Data Version:  dataset_v2.1 (SHA: 8c4e...)                   │
│  AI Sessions:   3 sessions, last: "optimize memory usage"     │
│                                                                 │
│  [View Full Provenance]  [Export to Methods Section]  [Re-run] │
└─────────────────────────────────────────────────────────────────┘
```

**Why this matters:** Cognitive Load research shows that chunking related information reduces mental burden. Sarah can immediately see the complete context of any experiment without hunting through git history.

### 5.2 Documentation-on-Demand Panel

**Provide contextual documentation generation:**

```
┌─────────────────────────────────────────────────────────────────┐
│ DOCUMENTATION ASSISTANT                                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Target: analyze_sequence(filepath: Path, min_quality: int)    │
│                                                                 │
│  [✓] Generate NumPy-style docstring                            │
│  [✓] Add type hints                                            │
│  [✓] Include usage example                                      │
│  [✓] Add "See Also" references                                  │
│  [ ] Generate unit tests                                        │
│                                                                 │
│  Preview:                                                        │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ def analyze_sequence(                                      ││
│  │     filepath: Path,                                        ││
│  │     min_quality: int = 30                                 ││
│  │ ) -> pd.DataFrame:                                        ││
│  │     """                                                    ││
│  │     Analyze sequencing data from FASTA/FASTQ files.       ││
│  │                                                             ││
│  │     Parameters                                            ││
│  │     ----------                                            ││
│  │     filepath : Path                                       ││
│  │         Path to input sequencing file (.fa, .fq, .fastq)  ││
│  │     min_quality : int, default=30                         ││
│  │         Minimum Phred quality score to include             ││
│  │                                                             ││
│  │     Returns                                                ││
│  │     -------                                                ││
│  │     pd.DataFrame                                          ││
│  │         DataFrame with columns: pos, base, quality        ││
│  │     """                                                    ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  [Insert at Cursor]  [Append to Existing]  [Preview in Context] │
└─────────────────────────────────────────────────────────────────┘
```

**Why this matters:** Progressive disclosure from Cognitive Load research — show Sarah the options she needs, hide advanced features until requested.

### 5.3 Scientific Code Review Mode

**Implement a review mode with academic-specific checks:**

```
┌─────────────────────────────────────────────────────────────────┐
│ SCIENTIFIC CODE REVIEW                                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ✓ Statistical assumptions documented                           │
│  ⚠ Missing null hypothesis documentation                       │
│  ✗ Random seed not set (may affect reproducibility)            │
│  ✓ Output format matches publication template                   │
│                                                                 │
│  Statistical Checks:                                            │
│  ──────────────────                                             │
│  [PASS] Normality test documented                               │
│  [FAIL] No correction for multiple comparisons                  │
│  [WARN] Sample size not validated against power analysis        │
│                                                                 │
│  Reproducibility Checks:                                         │
│  ───────────────────────                                        │
│  [PASS] All dependencies pinned                                 │
│  [FAIL] Data paths hardcoded                                    │
│  [WARN] GPU model not specified                                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Why this matters:** Pattern matching research on delightful experiences — clear, actionable feedback that Sarah can address before paper submission.

### 5.4 Experiment Versioning Interface

**Visualize experiment lineage:**

```
┌─────────────────────────────────────────────────────────────────┐
│ EXPERIMENT HISTORY: protein_folding                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  v1.0.0 ─────┬──── v1.1.0 ─────┬──── v2.0.0                   │
│  baseline    │    +GPU opt     │    +ensemble                 │
│              │                 │                              │
│              └─ v1.1.1 ────────┘    (merged)                  │
│              +memory opt                                  │
│                                                             │
│  Current: v2.0.0                                           │
│  Result: accuracy=0.89, time=2.3h                         │
│                                                             │
│  [Compare Versions]  [Restore v1.0.0]  [Tag Release]       │
└─────────────────────────────────────────────────────────────────┘
```

**Why this matters:** Visual representation of experiment lineage supports the "external memory" principle from Cognitive Load research — Sarah doesn't have to remember every change.

### 5.5 Citation Generation Panel

**Auto-generate citations for AI-assisted code:**

```
┌─────────────────────────────────────────────────────────────────┐
│ CITATION & ATTRIBUTION                                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  This function was AI-assisted on 2026-07-15                   │
│  Prompt: "implement Smith et al. 2024 clustering algorithm"    │
│                                                                 │
│  Suggested citation:                                            │
│  ─────────────────────                                          │
│  "This analysis utilized AI-assisted code generation            │
│   tools (Runie, v1.2.0). The implementation of the            │
│   clustering algorithm follows Smith et al., Nature            │
│   Methods, 2024, with modifications as described in           │
│   supplementary section S3."                                    │
│                                                                 │
│  Code license: MIT                                              │
│  Data source: GEO GSE123456 (cite if using this data)          │
│                                                                 │
│  [Copy Citation]  [Add to Methods Template]                   │
└─────────────────────────────────────────────────────────────────┘
```

**Why this matters:** Addresses the unique academic need for attribution and licensing clarity.

---

## 6. Default Behaviors That Would Impress Them

### 6.1 Git-First Workflow

**Default behavior: Every significant AI-assisted change creates a meaningful git commit.**

```
$ runie "optimize the sequence alignment function"
✓ Understood optimization goal
✓ Found: analyze_sequences.py:142-178
✓ Suggested changes:
  - Vectorized loop with NumPy
  - Added Cython fallback
  - Set numba JIT cache
✓ Created commit: "Optimize alignment with vectorization"
✓ Environment snapshot: python=3.11, numpy=1.24.0, numba=0.58.0
```

**Why impressive:** Follows Unix philosophy of transparent, auditable operations. Sarah can always trace exactly what changed and why.

### 6.2 Deterministic Mode

**Default behavior: AI-generated code includes random seed handling.**

```python
# Before AI generation:
# Some ML code with implicit randomness

# After AI generation (with deterministic mode):
import numpy as np
import torch

# Set seeds for reproducibility
np.random.seed(42)
torch.manual_seed(42)
torch.cuda.manual_seed_all(42)
# Additional: make CuDNN deterministic
torch.backends.cudnn.deterministic = True
torch.backends.cudnn.benchmark = False

# TODO: Document this choice in methods section
```

**Why impressive:** Removes a major source of reproducibility headaches in ML-heavy research.

### 6.3 Comprehensive Type Hints

**Default behavior: Generate complete type annotations by default.**

```python
# Instead of:
def process_data(data, threshold=0.5):
    ...

# Generate:
from typing import Literal
import numpy as np
import pandas as pd

def process_data(
    data: pd.DataFrame | np.ndarray,
    threshold: float = 0.5,
) -> pd.DataFrame:
    """Process sequencing data with quality filtering.
    
    Parameters
    ----------
    data : pd.DataFrame or np.ndarray
        Input data with 'quality' column or array values.
    threshold : float, default=0.5
        Minimum quality score threshold. Must be between 0 and 1.
    
    Returns
    -------
    pd.DataFrame
        Filtered data with rows below threshold removed.
    
    Raises
    ------
    ValueError
        If threshold is outside [0, 1] range.
    """
    if not 0 <= threshold <= 1:
        raise ValueError(f"threshold must be in [0, 1], got {threshold}")
    ...
```

**Why impressive:** Matches the docstring standards expected in scientific Python, reducing review friction.

### 6.4 Environment Specification

**Default behavior: Capture complete environment before any AI-assisted changes.**

```
$ runie "implement PCA analysis"
✓ Environment snapshot:
  Python: 3.11.4
  NumPy: 1.24.3
  Pandas: 2.0.2
  Scikit-learn: 1.3.0
  OS: Ubuntu 22.04 LTS
  CPU: x86_64
  GPU: NVIDIA RTX 3090 (CUDA 11.8)

✓ Created: environment.lock
✓ Run: pip-compile environment.lock  # to generate requirements.txt
```

**Why impressive:** Ensures that "it worked on my machine" never happens in collaborative settings.

### 6.5 Parameter Logging

**Default behavior: Log all parameters to a machine-readable file.**

```json
{
  "experiment_id": "pca_analysis_2026_07_15_001",
  "timestamp": "2026-07-15T14:32:07Z",
  "git_commit": "a3f8c21",
  "parameters": {
    "n_components": 10,
    "standardize": true,
    "whiten": false,
    "svd_solver": "auto"
  },
  "input_hash": "sha256:8c4e2f1a...",
  "output_hash": "sha256:3f2b8d9c...",
  "ai_assisted": true,
  "ai_prompt": "implement PCA with configurable components"
}
```

**Why impressive:** Creates an immutable record that can be cited in publications.

---

## 7. Reproducibility and Documentation Requirements

### 7.1 Minimum Viable Reproducibility

Every Runie session should guarantee:

1. **Complete dependency specification**
   - Python package versions (via `pip freeze` equivalent)
   - System dependencies (apt/yum packages if applicable)
   - Hardware requirements (GPU model, memory minimum)
   - External data references (URLs with checksums)

2. **Parameter documentation**
   - Every function parameter documented
   - Default values explained
   - Units specified for numerical parameters
   - Range validation documented

3. **Data provenance**
   - Input data hash/checksum recorded
   - Data source cited (database, URL, generated)
   - Data preprocessing steps documented

4. **Execution context**
   - Random seeds for all stochastic operations
   - Environment variables that affect behavior
   - Runtime: CPU vs GPU, thread count

### 7.2 Publication-Ready Documentation Checklist

When Sarah marks code as "ready for publication":

- [ ] All functions have complete docstrings (NumPy style)
- [ ] Type hints on all public interfaces
- [ ] README with: installation, usage, examples, testing
- [ ] CITATION.cff file with correct metadata
- [ ] LICENSE file (MIT, Apache, or specified)
- [ ] Code passes linting (ruff, mypy)
- [ ] Tests have >80% coverage
- [ ] Example notebook demonstrates key use cases
- [ ] Methods section template generated

### 7.3 Experiment Registry Integration

Support integration with common research tooling:

| Tool | Integration |
|------|-------------|
| **MLflow** | Log parameters, metrics, and artifacts |
| **DVC** | Version control for data alongside code |
| **Snakemake** | Generate rule-based workflows |
| **Nextflow** | DSL2 pipeline generation |
| **Jupyter** | Notebook cell generation with prose |
| **Quarto** | Generate reproducible manuscripts |
| **Renku** | Full research reproducibility platform |

### 7.4 Audit Trail Requirements

For grant compliance and publication:

- **Immutable timestamps** — Verifiable timestamps for all changes
- **Change attribution** — Who made what change (human vs AI)
- **Review history** — Code review comments preserved
- **Access logs** — Who accessed what (for sensitive data)
- **Export capabilities** — Generate reports for ethics boards

---

## 8. Exploration and Experimentation Workflow Needs

### 8.1 Hypothesis-Driven Exploration

Research exploration requires systematic testing of hypotheses:

```
┌─────────────────────────────────────────────────────────────────┐
│ HYPOTHESIS: "Increasing k-mer size improves assembly quality"   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Experiments:                                                   │
│  ────────────                                                  │
│  [1] k=21, N50=45kb, time=2.3h, memory=32GB                    │
│  [2] k=31, N50=52kb, time=3.1h, memory=38GB                    │
│  [3] k=41, N50=48kb, time=4.2h, memory=45GB  ← current best    │
│  [4] k=51, N50=42kb, time=5.8h, memory=52GB                    │
│                                                                 │
│  AI Analysis:                                                   │
│  "k=31 appears optimal. k>41 shows diminishing returns         │
│   due to coverage reduction in low-complexity regions."        │
│                                                                 │
│  [Generate Figure]  [Add to Methods]  [Plan Next Experiment]   │
└─────────────────────────────────────────────────────────────────┘
```

**Why this matters:** Supports the scientific method workflow, not just code generation.

### 8.2 Parallel Experiment Tracking

**Simultaneously run and compare multiple experiments:**

```bash
# Launch parameter sweep
runie sweep --experiment=assembly_optimization \
    --k=21,31,41,51 \
    --min_len=1000,5000,10000 \
    --output=results/sweep_{k}_{min_len}

# View live comparison
runie compare --experiment=assembly_optimization
```

### 8.3 "What If" Exploration Mode

**Support exploratory analysis without commitment:**

- **Temporary changes** — Try variations without polluting git history
- **Branching experiments** — Create named branches for hypothesis testing
- **Easy rollback** — Discard exploratory work without trace
- **Promote to production** — Graduate successful experiments to main branch

### 8.4 Literature-Aware Suggestions

**When AI suggests an algorithm, also show related work:**

```
AI Suggestion: Use DBSCAN for clustering gene expression data

Related Literature:
├── "DBSCAN: Density-based clustering for spatial data" (Ester et al., 1996)
├── "Gene expression clustering with density-based methods" (Kim et al., 2023)
└── "Comparison of clustering algorithms for scRNA-seq" (Duò et al., 2020)

Recommendation: Cite Ester et al. 1996 for the algorithm, 
               Duò et al. 2020 for the application domain.
```

### 8.5 Interactive Visualization Support

**Generate publication-quality figures from exploration:**

```python
# Runie could suggest:
import matplotlib.pyplot as plt
import seaborn as sns

# Publication-ready figure
fig, ax = plt.subplots(figsize=(8, 6))
sns.boxplot(data=df, x='condition', y='expression', ax=ax)
ax.set_xlabel('Experimental Condition', fontsize=12)
ax.set_ylabel('Relative Expression', fontsize=12)
ax.set_title('Gene Expression by Condition', fontsize=14)

# Export with publication settings
fig.savefig('figures/expression_boxplot.pdf', 
            dpi=300, bbox_inches='tight', 
            fontfamily='Times New Roman')
```

---

## 9. How Runie Can Exceed Their Expectations (Wow Factors)

### 9.1 The "Methods Section Generator"

** Automatically generate publication-ready methods sections:**

```
$ runie generate methods-section --experiment=variant_calling

Generated Methods Section:
═══════════════════════════════════════════════════════════════

Sequence alignment was performed using BWA-MEM2 (v2.2.1) with
default parameters unless otherwise specified. Variant calling
was conducted using GATK HaplotypeCaller (v4.3.0.0) following
the recommended best practices workflow (Van der Auwera & O'Connor,
2020). Quality control metrics were calculated using FastQC (v0.12.1).
Statistical analysis was performed in Python (v3.11.4) using SciPy
(v1.11.0) and NumPy (v1.24.3). AI-assisted code generation was
performed using Runie (v1.2.0); custom modifications to the variant
filtering pipeline are available at [GitHub DOI].

Parameters:
- Minimum allele frequency: 0.01
- Quality cutoff: 20 (Phred scale)
- Minimum coverage: 10x

═══════════════════════════════════════════════════════════════

[Copy to Clipboard]  [Open in Overleaf]  [Export as .tex]
```

**Wow factor:** Transforms tedious methods writing into one command.

### 9.2 The Reproducibility Score

**Score the reproducibility of any code artifact:**

```
$ runie score reproducibility --path=./src/analysis/

Reproducibility Score: 72/100

Breakdown:
┌─────────────────────────────────────────────────────────────────┐
│ Environment                    15/20  ████████░░░░░░░          │
│  ✓ Dependencies pinned        +10                           │
│  ⚠ Python version unspecified +5                           │
│  ✗ System deps not documented  -5                           │
│                                                                 │
│ Documentation                 18/20  █████████░░░░░░░          │
│  ✓ Complete docstrings        +10                           │
│  ✓ Type hints present         +5                            │
│  ⚠ Usage examples incomplete  +3                           │
│                                                                 │
│ Data Provenance               12/20  ██████░░░░░░░░░░          │
│  ✓ Input checksums recorded   +8                            │
│  ✗ Output not tracked        -8                            │
│                                                                 │
│ Testing                       15/20  ███████░░░░░░░░          │
│  ✓ Tests present              +8                            │
│  ⚠ Coverage 78%              +4                            │
│  ✗ No integration tests      -3                            │
│                                                                 │
│ Statistical Rigor             12/20  ██████░░░░░░░░░░          │
│  ✓ Random seeds set          +5                            │
│  ⚠ No power analysis         +3                            │
│  ⚠ Multiple comparison corr.  +4                            │
└─────────────────────────────────────────────────────────────────┘

Recommendations:
1. Add Python version requirement to environment.yml
2. Set up DVC for output tracking
3. Include power analysis justification

[Generate Report for PI]  [Open Fix Wizard]
```

**Wow factor:** Quantifiable reproducibility metrics that satisfy reviewers and funding agencies.

### 9.3 The Co-Pilot Review Mode

**AI that reviews research code with scientific rigor:**

```
$ runie review --method=strict --focus=statistics

Reviewing: statistical_analysis.py

⚠ Statistical Concern:
   Line 142: t-test assumes normality, but n=12 samples
   may not be sufficient for CLT. Consider:
   - Shapiro-Wilk normality test (p=0.23)
   - Mann-Whitney U test as non-parametric alternative
   - Bootstrap confidence intervals

⚠ Reproducibility Concern:
   Line 89: np.random.shuffle() uses system entropy.
   Add: np.random.seed(42) before this call.

⚠ Citation Needed:
   Line 234: "Benjamini-Hochberg FDR correction"
   Consider citing: Benjamini & Yekutieli (2001)
   
✓ Good Practice:
   - Effect sizes reported alongside p-values (Cohen's d)
   - Post-hoc power analysis included
   - Figure captions match journal standards

Overall Assessment: APPROVED WITH NOTES
Code is suitable for publication pending minor revisions.
```

**Wow factor:** Expert-level statistical review without requiring a statistics co-author.

### 9.4 The Collaborative Experiment Notebook

**Jupyter notebook integration with built-in reproducibility:**

```python
# In Jupyter, Sarah can run:
# %load_ext runie_magics

# %%runie --experiment=patient_stratification
# Train a classifier to stratify patients by prognosis

# This automatically:
# 1. Snapshots the current code state
# 2. Logs all parameters to MLflow
# 3. Saves output to versioned artifact store
# 4. Generates provenance metadata
# 5. Creates a reproducible notebook snapshot
```

**Wow factor:** Jupyter notebooks become first-class reproducibility documents.

### 9.5 The Grant Progress Dashboard

**For funding agencies and lab meetings:**

```
┌─────────────────────────────────────────────────────────────────┐
│ LAB PROGRESS: NIH R01-GM123456                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Aims:                                                          │
│  ├─ Aim 1: Develop variant calling pipeline         [████████░]│
│  │  └─ Milestone: 1000-genome benchmark              [Complete] │
│  │  └─ Milestone: Publication submitted               [In Review]│
│  │                                                         90%  │
│  │                                                             │
│  ├─ Aim 2: Validate in clinical cohort             [██████░░░░]│
│  │  └─ Milestone: IRB approval                        [Complete]│
│  │  └─ Milestone: Initial validation (n=100)          [Complete]│
│  │  └─ Milestone: Full cohort (n=1000)                [In Progress]│
│  │                                                         65%  │
│  │                                                             │
│  └─ Aim 3: Distribute as open-source tool        [██░░░░░░░░]│
│     └─ Milestone: GitHub release                    [Planned] │
│     └─ Milestone: Documentation site               [Planned] │
│                                                                 15%  │
│                                                                 │
│  Code Metrics:                                                  │
│  ├─ Total reproducibility score: 84/100                       │
│  ├─ Test coverage: 87%                                        │
│  ├─ Open issues: 12 (4 critical)                               │
│  └─ Citation count: 3 (new this month: +1)                   │
│                                                                 │
│  [Export for Progress Report]  [Generate PI Summary]          │
└─────────────────────────────────────────────────────────────────┘
```

**Wow factor:** Transforms scattered code into compelling grant narratives.

---

## Summary

Dr. Sarah Okonkwo represents a persona whose success depends on **rigorous reproducibility, clear documentation, and transparent AI assistance**. Unlike pure software engineers, she operates in a world where every code artifact may be scrutinized by peer reviewers, funding agencies, and future researchers.

### Key Differentiators for This Persona

| Need | Why It Matters | Runie's Opportunity |
|------|----------------|---------------------|
| **Provenance tracking** | Peer review, grant compliance | Automatic git commits, parameter logging |
| **Statistical rigor** | Scientific validity | Built-in statistical review mode |
| **Publication-ready output** | Career advancement | One-command methods generation |
| **Collaboration tools** | Multi-author papers | Co-author attribution, review workflows |
| **Exploration support** | Hypothesis testing | Experiment versioning, comparison tools |

### Core Principle

> "The best AI tool for research is one that makes reproducibility easier than not, documentation more automatic than manual, and collaboration more natural than email."

Runie can exceed expectations by treating documentation, versioning, and reproducibility as **first-class features**, not afterthoughts. When Sarah can say "this result came from commit X with parameters Y using data Z, and here's the citation for the algorithm," she has a competitive advantage that pure productivity gains cannot match.

---

*Document version: 1.0*
*Created: 2026-07-15*
*Research basis: UX best practices, Unix philosophy, Cognitive Load Theory*
