# plagengine — design plan

A plagiarism detection engine whose JSON output mirrors Turnitin's Enhanced
Similarity Report. Pure-Rust, embedded on-disk index (redb), no ML,
`#![forbid(unsafe_code)]` in every crate.

## Workspace

```
crates/core   plagengine-core  library: every stage, store, report, eval metrics
crates/cli    plagengine       binary: ingest / check / report / sources / eval
```

## Data flow

```
file ──extract──▶ ExtractedDoc ──normalize──▶ NormDoc ──fingerprint──▶ k-gram hashes
                  text + hidden spans         tokens w/ original      │
                  + block quotes              char offsets            ├─ ingest: winnowed → redb postings
                  + citation markers          + replaced/invisible    └─ check : all hashes → candidates
                                                                              │
          Report ◀──filters+attribute── CheckResult (raw, persisted) ◀──align─┘
```

`CheckResult` is the unfiltered output of detection (all aligned passages,
quote spans, bibliography span, flags, word/char accounting). `Report` is a
*pure function* `build_report(&CheckResult, &Filters)`, so filters are
re-runnable without re-checking: every check is persisted in the store under a
`check_id` (`plagengine report <check_id> --exclude-quotes`), and `--embed-raw`
makes a report JSON self-contained (`plagengine report report.json ...`).

## Offsets

All ranges are **Unicode scalar (char) indices**, half-open `[start, end)`,
into the extracted text of the document (for `.txt`/`.md`, the file verbatim —
this is also what PAN annotations use).

## Stages

1. **extract** (`extract/`): txt, md (verbatim; `>` lines = block quotes),
   pdf (`pdf-extract`), docx (zip + quick-xml). docx resolves run formatting
   through docDefaults → paragraph style chain → character style chain →
   direct `w:rPr`, and marks *hidden* runs: `w:vanish`/`w:specVanish`, white
   or near-white color (incl. `themeColor=background1`), color equal to
   shading/highlight, font size ≤ 2pt. Footnote references become citation
   markers; footnotes/endnotes are appended; `w:delText`/`w:instrText` skipped;
   `Quote`/`IntenseQuote` paragraph styles become block quotes.
2. **normalize** (`normalize.rs`): per original char → NFKD, drop combining
   marks, homoglyph fold (Cyrillic/Greek/Armenian/… confusables → Latin),
   lowercase; zero-width & other invisible chars stripped (not token breaks);
   hidden spans become token breaks and are excluded from tokens. Tokens are
   alphanumeric runs (CJK ideographs = 1 token each) carrying original
   `[start,end)`. Also records *replaced characters* (confusables / fullwidth /
   math letters in a Latin-dominant context, ligatures excepted) and
   *invisible characters* (ZW*, bidi controls, tag chars, soft hyphen, …).
3. **fingerprint** (`fingerprint.rs`): token hash = xxh3; k-gram hash = mix of
   k token hashes; winnowing (window w, rightmost-minimum rule). Sources are
   winnowed (index k=4, w=3 → any shared run ≥ 6 tokens is guaranteed to be
   found); the query uses *all* k-grams.
4. **candidates** (`candidates.rs`): inverted index lookup `hash → (doc,pos)`,
   skipping over-frequent hashes (df cap); docs ranked by distinct shared
   hashes, `≥ min_shared`, top `max_candidates`.
5. **align** (`align.rs`): per candidate (rayon), dense seeds from full 3-gram
   matching between the pair → extend to maximal exact token runs → greedy
   chaining of fragments with gap ≤ `max_gap` tokens on both sides (light
   edits) → iterate merging → keep passages with ≥ `min_tokens` matched tokens
   and density ≥ `min_density`. Each passage keeps its aligned token pairs so
   it can later be clipped and still map to an exact source range.
6. **attribute** (`attribute.rs`): greedy set cover over submission chars —
   repeatedly pick the source with the most *unclaimed* countable chars
   (after the min-words filter on the clipped pieces), claim them, stop when
   the best gain falls below the min-% filter. Each char counts toward exactly
   one (highest-ranked) source.

## Report (mirrors Turnitin Enhanced Similarity Report)

- `similarity`: matched countable chars ÷ countable chars after filters
  (countable = non-whitespace, visible; bibliography / quoted text leave the
  denominator when excluded). Integer `percent` like Turnitin + `exact`.
- `by_collection`: for each of internet / publications / submissions, the
  Turnitin "Top Sources Overview" number (similarity against that collection
  alone — these can sum to more than the overall score) and the attributed
  share of the overall score (sums to it).
- `sources`: ranked; id, title, path/url, collection, %, matched chars,
  match count, per-group counts.
- `matches`: text, submission range, source range, source id, rank, words,
  `group`, quoted/cited evidence.
- `match_groups`: Not Cited or Quoted / Missing Quotations / Missing Citation
  / Cited and Quoted — match counts and % of submission.
  quoted = ≥ 80% of the match inside quotation marks or a block quote;
  cited = an in-text citation (author-date, narrative, numeric `[n]`, MLA
  `(Author 12)`, footnote marker, superscript digit) inside the match or in a
  window around it bounded by the paragraph.
- `flags` (only when ≥ 500 non-space chars): hidden text, replaced
  characters, other manipulation (invisible chars) — each with count and
  locations.

## Filters

`--exclude-quotes`, `--exclude-bibliography`, `--min-words N`,
`--min-percent N`, `--exclude-source ID` (repeatable),
`--exclude-collection NAME` (repeatable). Bibliography = last
References/Bibliography/Works Cited/… heading line in the back 70% of the doc
to end (or to an Appendix heading).

## Store (redb, single file, default `./plagengine.db`)

| table        | key                 | value                                   |
|--------------|---------------------|-----------------------------------------|
| `meta`       | &str                | config (k, w, next ids) — fixed at create |
| `docs`       | u32 doc id          | DocMeta (title, path, collection, lens)  |
| `doc_text`   | u32 doc id          | extracted text (+hidden spans)          |
| `content`    | u64 content hash    | u32 doc id (dedupe)                     |
| `postings`   | (u64 hash, u32 doc) | varint-packed token positions           |
| `checks`     | &str check id       | CheckResult (JSON)                      |

Ingest: walk → extract/normalize/fingerprint in parallel (rayon) → sorted
batch writes in one write transaction per batch.

## Evaluation

- `plagengine eval pan --corpus DIR`: PAN-PC-11 external detection corpus.
  Character-level macro precision/recall, granularity and plagdet (Potthast
  et al. 2010), overall and per obfuscation type. `--limit`, `--sources`
  sampling for laptop-scale runs.
- `plagengine eval adversarial --corpus DIR --out DIR`: deterministic
  generator (seeded) builds suspicious docs from corpus passages under
  homoglyph substitution, zero-width insertion, hidden-text injection in
  .docx (white "glue" letters replacing spaces + hidden filler), light
  rewording (synonyms, deletions, insertions, local swaps), and clean copies;
  then measures plagdet per attack plus flag detection rates.

## Build order (each stage lands with unit tests)

1. workspace scaffold, model types
2. normalize  3. fingerprint  4. store + candidates  5. align
6. quotes/citations/bibliography  7. flags  8. attribute + filters + report
9. extractors (txt/md/pdf/docx)  10. CLI  11. eval metrics + PAN + adversarial
12. end-to-end tests, run evals, write README with results
