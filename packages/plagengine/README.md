# plagengine

This Cargo workspace was imported from `/home/shujanforge/Projects/plagenine`.
It contains both original crates and their algorithms, tests, examples, and
locked dependencies. Run the Cargo commands below from `packages/plagengine`,
or use `pnpm engine --help` and `pnpm demo:engine` from the repository root.
The React app does not call this engine yet.

A plagiarism detection engine in Rust whose JSON output mirrors Turnitin's
**Enhanced Similarity Report**: similarity %, a per-collection breakdown,
ranked sources, every match with exact character ranges on both sides,
**Match Groups** (Not Cited or Quoted / Missing Quotations / Missing Citation
/ Cited and Quoted) and integrity **Flags** (hidden text, replaced
characters, other manipulation).

- Pure Rust. Every crate has `#![forbid(unsafe_code)]`. No AI/ML.
- Embedded on-disk index ([redb](https://github.com/cberner/redb), one file).
  No database server, and submissions persist between runs.
- Inputs: `.txt`, `.md`, `.pdf`, `.docx`. For `.docx`, run formatting is
  resolved (styles, colour, size, `w:vanish`) so hidden text can be flagged.
- Parallel with rayon (ingest, candidate alignment, evaluation).

See [PLAN.md](PLAN.md) for the design notes written before implementation.

## Quick start

```sh
cargo build --release
B=target/release/plagengine

$B ingest ./corpus/web   --collection internet
$B ingest ./corpus/pubs  --collection publications
$B check essay.docx --store > report.json          # --store adds it to `submissions`
$B report <check_id> --exclude-quotes --exclude-bibliography --min-words 8
```

`examples/demo/run.sh` runs this whole flow on a small bundled corpus.

| command | purpose |
|---|---|
| `ingest PATH --collection internet\|publications\|submissions` | add a file or directory, recursively. An optional `<file>.meta.json` sidecar (`{"title":…, "url":…}`) sets the source title/URL |
| `check FILE [--store] [filters] [--embed-raw] [-o out.json]` | detect, print the report, and save the raw check under its `check_id` |
| `report CHECK_ID\|report.json [filters]` | re-apply filters to a saved check without re-checking (a report file needs `--embed-raw`) |
| `sources [--collection C]`, `stats` | inspect the index |
| `extract FILE` | debug: extracted text, hidden spans, block quotes, footnote markers |
| `eval pan …`, `eval adversarial …` | evaluations (see below) |

The index defaults to `./plagengine.db` (override with `--db` or `PLAGENGINE_DB`).

### Filters

All filters run on the persisted raw check, so they are re-runnable:

| flag | effect |
|---|---|
| `--exclude-quotes` | text inside quotation marks or block quotes is excluded (and leaves the denominator) |
| `--exclude-bibliography` | the References / Bibliography / Works Cited section is excluded (and leaves the denominator) |
| `--min-words N` | drop matches shorter than N words |
| `--min-percent N` | drop sources contributing less than N % |
| `--exclude-source ID` (repeatable) | drop one source; its characters can go to the next best source |
| `--exclude-collection NAME` (repeatable) | drop a whole collection |

## Report format

Trimmed output for `examples/demo` (`essay.md`):

```jsonc
{
  "engine": "plagengine 0.1.0",
  "check_id": "5cd5b472c698f3ac",
  "submission": { "title": "Why coral reefs matter", "format": "md", "characters": 1409,
                  "non_space_chars": 1176, "countable_chars": 1176, "words": 228 },
  "filters": { "exclude_quotes": false, "exclude_bibliography": false, "min_words": 0, ... },
  "similarity": { "percent": 45, "exact": 45.07, "matched_chars": 530, "total_chars": 1176 },
  "by_collection": [
    { "collection": "internet",     "percent": 23.04, "attributed_percent": 23.04, "sources": 1, "matches": 2 },
    { "collection": "publications", "percent": 22.02, "attributed_percent": 22.02, "sources": 1, "matches": 2 },
    { "collection": "submissions",  "percent": 0.0,   "attributed_percent": 0.0,   "sources": 0, "matches": 0 }
  ],
  "match_groups": [
    { "group": "Not Cited or Quoted", "match_count": 3, "chars": 408, "percent": 34.69 },
    { "group": "Missing Quotations",  "match_count": 0, "chars": 0,   "percent": 0.0 },
    { "group": "Missing Citation",    "match_count": 0, "chars": 0,   "percent": 0.0 },
    { "group": "Cited and Quoted",    "match_count": 1, "chars": 122, "percent": 10.37 }
  ],
  "sources": [
    { "rank": 1, "source_id": 1, "title": "Coral reef - Encyclopedia entry",
      "url": "https://example.org/wiki/Coral_reef", "path": "corpus/internet/coral-reefs.txt",
      "collection": "internet", "percent": 23.04, "matched_chars": 271, "match_count": 2,
      "match_groups": { "Not Cited or Quoted": 2, "Missing Quotations": 0, "Missing Citation": 0, "Cited and Quoted": 0 } },
    ...
  ],
  "matches": [
    { "id": 3, "source_id": 2, "source_rank": 2, "collection": "publications",
      "text": "mass coral bleaching events are triggered when sea surface temperatures exceed ...",
      "submission_range": { "start": 722, "end": 865 }, "source_range": { "start": 43, "end": 186 },
      "words": 22, "chars": 122, "group": "Cited and Quoted", "quoted": true, "cited": true,
      "citation": "(Hughes et al., 2018)" },
    { "id": 4, ..., "text": "Repeated bleaching leaves too little time for recovery between events, ...",
      "group": "Not Cited or Quoted", "quoted": false, "cited": false }
  ],
  "flags": {
    "eligible": true, "non_space_chars": 1176,
    "hidden_text":         { "count": 0, "locations": [] },
    "replaced_characters": { "count": 1, "locations": [
        { "range": { "start": 577, "end": 584 }, "count": 1, "text": "divеrse", "detail": "U+0435 'е' → 'e'" } ] },
    "other_manipulation":  { "count": 0, "locations": [] }
  },
  "excluded": { "quoted_ranges": 0, "excluded_chars": 0, "excluded_sources": [], "excluded_collections": [] }
}
```

How each number maps to Turnitin:

- **Character ranges** are half-open `[start, end)` Unicode-scalar indices
  into the extracted text. For `.txt`/`.md` this is the file itself, minus
  a leading BOM.
- **Similarity** = matched countable chars ÷ countable chars after filters.
  Countable means non-whitespace, visible (not hidden, not zero-width).
  `percent` is rounded to an integer, as Turnitin displays it.
- **One character, one source.** Sources are ranked by greedy set cover:
  the source explaining the most still-unclaimed characters ranks first,
  and so on. Per-source percentages therefore sum to the overall score.
- **`by_collection.percent`** is the similarity against that collection
  alone, which is Turnitin's "Top Sources Overview" figure, so the three can
  sum to more than the total. `attributed_percent` is each collection's share
  of the overall score, and those do sum to it.
- **Matches** are split wherever the quote status changes, where the source
  side jumps, or where more than 6 unmatched words intervene. Each match is
  therefore one highlight that maps to one source region.
- **Match Groups.** A match is *quoted* when ≥ 80% of it lies inside
  quotation marks (“…”, "…", ‘…’, '…', «…», „…“, 「…」), a markdown `>` block,
  or a docx `Quote` style. It is *cited* when an in-text citation sits inside
  the match's sentence(s) or immediately follows it. Recognised citation
  forms: author–date `(Smith & Lee, 2019, p. 4)`, narrative
  `Jones et al. (2020)`, numeric `[3, 5–7]`, MLA `(Brown 42)`, docx
  footnote/endnote references, and superscript numbers.
- **Flags** apply only to documents with ≥ 500 non-space characters.
  `count` is the number of suspect characters, and each location gives its
  range, the affected text and the reason:
  - *hidden text*: `.docx` runs with `w:vanish`, white or near-white text
    (including `themeColor=background1`) not on a dark highlight, text the
    same colour as its shading or highlight, or font ≤ 2 pt.
  - *replaced characters*: Cyrillic, Greek, Armenian and Cherokee look-alikes,
    Latin small capitals, and fullwidth or mathematical letters, when they sit
    inside Latin words or form whole confusable words in Latin-dominant text.
    Real Russian or Greek text, accented letters and ligatures are not flagged.
  - *other manipulation*: zero-width characters, bidi controls, soft hyphens,
    tag characters, variation selectors and Hangul fillers adjacent to letters.

## How detection works

```
extract → normalize → fingerprint → candidates → align → attribute → report
```

1. **extract** (`crates/core/src/extract`): txt and md are kept verbatim
   (`>` lines become block quotes); pdf uses `pdf-extract`, run under
   `catch_unwind`. For docx, a quick-xml parser resolves docDefaults →
   paragraph style chain → character style chain → direct `w:rPr` for every
   run and records hidden spans. It also records footnote references (as
   citation evidence) and Quote styles, appends footnotes and endnotes, and
   skips deleted text and field codes.
2. **normalize** (`normalize.rs`): each char is folded independently (NFKD,
   drop combining marks, map homoglyphs to Latin, lowercase). Invisible
   chars are dropped without splitting the word they sit in, so
   `pla​giarism` is one token. Hidden spans act as separators, which defeats
   the "white letters instead of spaces" trick. Tokens are alphanumeric runs
   (CJK ideographs are one token each) and keep their original char range.
3. **fingerprint** (`fingerprint.rs`): token hash xxh3 → order-sensitive
   k-gram hash → winnowing. Stored sources use k = 4, w = 3, so any shared
   run of ≥ 6 tokens is guaranteed to be found. Queries use every k-gram.
4. **candidates** (`candidates.rs`): inverted index `(hash, doc)` in redb.
   Hashes found in more than 1000 documents are skipped as boilerplate.
   Documents are ranked by their densest window of hits, so a book sharing a
   few scattered stock phrases doesn't outrank a real source.
5. **align** (`align.rs`), one rayon task per candidate:
   - Seeds are shared 3-grams. A query-side index is built once per check.
   - Seeds on a diagonal are extended into maximal exact fragments.
   - A *fine tier* chains fragments with gaps ≤ 5 tokens on both sides,
     which tolerates substitutions, insertions and deletions.
   - A *coarse tier* chains along a diagonal band (bounded gap and drift,
     with a density floor) for heavily edited text. A passage must contain a
     fine-tier core of ≥ 6 tokens, so scraps of stock phrases never stitch
     together.
   - A *final grouping* joins nearby same-source passages, in either source
     order, because obfuscation reorders sentences.
   - Each passage keeps its aligned token pairs, so a clipped piece still
     maps to an exact source range.
6. **attribute** (`attribute.rs`): greedy set cover over submission chars
   (see above). The min-words and min-% filters are applied inside the
   greedy loop, so filtered-out sources free their characters for others.
7. **report** (`report.rs`): a pure function `build_report(&CheckResult,
   &Filters)`. Quotes, citations, bibliography and flags are recomputed from
   the persisted extracted document.

Store layout (`store.rs`): `meta` (index config, fixed at creation), `docs`
(metadata), `doc_text` (extracted doc), `content` (content hash → id,
dedupe), `postings` (`(u64 hash, u32 doc)` keys), `checks` (raw check JSON).
Large ingests buffer postings and flush them sorted in a few big
transactions. Postings keys are uniformly spread, so flushing per batch
would rewrite most of the copy-on-write B-tree on every commit.

## Evaluation

Both evaluations are reproducible with fixed seeds.

```sh
# PAN-PC-11 (https://zenodo.org/records/3250095), external-detection-corpus
plagengine eval pan --corpus …/external-detection-corpus --limit 0 --distractors -1   # full corpus
plagengine eval pan --corpus …/external-detection-corpus --limit 100 --distractors 300 # quick sample
# Adversarial set generated from any directory of .txt documents
plagengine eval adversarial --corpus …/external-detection-corpus/source-document --sources 200 --cases-per-attack 50
```

Metrics are the PAN character-level macro precision, recall, granularity
and plagdet = F1 / log2(1 + granularity) (Potthast et al., COLING 2010),
reported overall, without the cross-language cases ("monolingual"), and per
obfuscation type. `eval pan` uses a PAN profile: `pan_profile()` (15-word
minimum, wider coarse tier, final grouping at 300 tokens, 50 candidates)
plus a 200-character floor on detections, because 99% of PAN cases are
≥ 213 characters. `--dump` writes the ground truth and detections, and
`eval rescore DUMP` re-scores them without re-running detection.
Every knob can be overridden from the command line (`--min-passage-words`,
`--coarse-gap`, `--final-gap`, `--max-candidates`, …).

### Results (8-core laptop, release build)

**PAN-PC-11, full source index**: all 11,093 source documents indexed
(12.9 GB index file for 2.4 GB of text). 2,000 suspicious documents were
sampled at random (seed 2011), covering 8,692 cases.

| | precision | recall | granularity | plagdet |
|---|---|---|---|---|
| **overall** | 0.682 | 0.411 | 1.198 | **0.451** |
| monolingual (no translation cases) | 0.681 | 0.472 | 1.199 | **0.491** |
| artificial, no obfuscation | 0.996 | 0.991 | 1.000 | 0.993 |
| artificial, low obfuscation | 0.994 | 0.800 | 1.229 | 0.767 |
| artificial, high obfuscation | 0.994 | 0.175 | 1.198 | 0.262 |
| simulated (manual paraphrase) | 0.995 | 0.302 | 1.000 | 0.464 |
| translation (es/de → en) | – | 0.001 | – | 0.003 |

Per-type precision only counts detections that hit a case of that type
(PAN convention), so read recall and granularity in those rows.

Most of the remaining false positives are *real but unannotated* reuse:
Bible verses, Longfellow, epigrams from quotation books. These occur both in
a suspicious document's "original" text and in some source. A Turnitin-style
engine should report them; PAN scores them as errors. Scored without the
200-char detection floor (`eval rescore <dump> --min-detection-chars 0`),
precision is 0.498 and plagdet 0.404. Throughput on the full index was about
1 suspicious document/s; PAN suspicious documents are often book-length.

For context, the PAN 2011 external-detection winner scored plagdet 0.556
(P 0.94, R 0.40, G 1.00), and the runner-up scored 0.42.

**PAN-PC-11, quick sample** (`--limit 100 --distractors 300`: 100 suspicious
docs against their ~150 sources plus 300 distractors, 46 s):

| | precision | recall | granularity | plagdet |
|---|---|---|---|---|
| overall | 0.972 | 0.429 | 1.199 | 0.524 |
| monolingual | 0.972 | 0.510 | 1.199 | 0.588 |

**Adversarial set** (`--sources 200 --cases-per-attack 50`): 300 documents.
Each is 3–6k characters of the generator's own text, taken from documents
that are never indexed, around one 400–1,500-character passage copied from
one of 200 indexed PAN source books.

- *raw* scores every aligned passage against every candidate source.
- *report* scores the matches a user sees after attribution.
- *flags* counts documents where every expected flag fired.

| attack | raw P / R / plagdet | report P / R / plagdet | flags | unexpected flags |
|---|---|---|---|---|
| clean copy (control) | 0.77 / 1.00 / 0.87 | 0.79 / 1.00 / 0.86 | – | 0 / 50 |
| homoglyphs (30% of letters → Cyrillic) | 0.69 / 1.00 / 0.82 | 0.77 / 1.00 / 0.87 | 50 / 50 replaced chars | 0 |
| invisible chars (ZW* / WJ / BOM in 60% of words) | 0.52 / 1.00 / 0.69 | 0.73 / 1.00 / 0.83 | 50 / 50 other manipulation | 0 |
| hidden text (.docx white letters for spaces + vanish/tiny filler) | 0.73 / 1.00 / 0.84 | 0.77 / 1.00 / 0.87 | 50 / 50 hidden text | 0 |
| light rewording (synonyms, deletions, swaps) | 0.71 / 0.995 / 0.83 | 0.76 / 0.99 / 0.79 | – | 0 |
| combined (reword + homoglyph + invisible) | 0.70 / 0.996 / 0.82 | 0.71 / 0.996 / 0.83 | 50 / 50 both | 0 |

Recall is ≈ 1.0 under every attack, and the integrity flags fire on 100% of
attacked documents with zero false flags on clean copies. Precision below
1.0 here comes from common 8+-word phrases in the generator's own text that
really do occur in some indexed book ("it makes all the difference in the
world"). These show up as tiny secondary sources, as in Turnitin, and
`--min-words` removes them.


## Limitations

- **Cross-language plagiarism** (PAN `translation` cases) is out of scope
  for v1: no MT and no ML.
- **Heavy machine obfuscation** (PAN `artificial-high`) is only partly
  recovered. Purely lexical seeds don't survive synonym substitution of most
  content words.
- **PDF**: text only. Hidden-text flags are `.docx`-only, and hyphenated line
  breaks are not re-joined.
- **One writer at a time.** redb takes a file lock, so concurrent `check`
  processes on the same index must be serialised (every check saves its
  result).
- **Short stock phrases.** By default passages need ≥ 8 matched words, so
  8+-word stock phrases still appear as small (<1%) sources, as in Turnitin.
  Use `--min-words` to hide them.

## Development

```sh
cargo test               # unit tests per stage + CLI end-to-end tests
cargo clippy --all-targets
cargo run --release -p plagengine-core --example debug_align -- susp.txt src.txt   # inspect alignment
cargo run --release -p plagengine-core --example profile_check -- index.db file.txt # timing breakdown
```
