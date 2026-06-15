#!/usr/bin/env python3
"""
slop-detector pre-screen: deterministic multilingual pattern matching + statistics.

Run this BEFORE the full qualitative audit to get a baseline signal.
This catches the obvious stuff. The qualitative pass catches everything else.

Detects AI-writing tics in English, French, and Spanish via a two-tier model:
- HARD tells (assistant leakage, model artifacts, high-specificity calques) score
  on a single match.
- SOFT tells (intensifiers, connectors, em-dashes, entrenched anglicisms, the
  negation->affirmation pivot) only score when clustered, to avoid penalizing
  ordinary human writing. See `references/heuristics*.md` for the catalogue.

Usage:
    python score.py < input.txt
    python score.py --file path/to/text.txt
    python score.py --lang fr --genre general --strictness brutal --file text.txt
    python score.py --lang auto --json --file text.txt
"""

import argparse
import json
import re
import sys
from collections import Counter, defaultdict
from itertools import pairwise

from patterns import TICS

# === COMPILER ===========================================================
# Tic regexes carry leading/global inline flags (e.g. "(?i)", "(?im)"). Python
# rejects global flags that are not at the very start of the pattern, so we
# extract every global flag group, apply the union, and strip them. MULTILINE is
# always on (harmless for patterns that do not anchor); patterns that need
# case-insensitivity declare "(?i)" themselves — case-sensitive tells (sentence
# capitalisation, Spanish ¿/¡ detection, citation tokens) do not.

_FLAG_GROUP = re.compile(r"\(\?([aiLmsux]+)\)")


def compile_tic(rx):
    flags = re.MULTILINE
    found = "".join(_FLAG_GROUP.findall(rx))
    if "i" in found:
        flags |= re.IGNORECASE
    if "s" in found:
        flags |= re.DOTALL
    return re.compile(_FLAG_GROUP.sub("", rx), flags)


COMPILED = [(compile_tic(t["rx"]), t) for t in TICS]

# === LANGUAGE DETECTION =================================================

_LANG_MARKERS = {
    "fr": r"\b(?:le|la|les|une|des|est|et|dans|pour|que|qui|ne|pas|vous|nous|avec|sur|aux|cette|ces|leur|d['’]|l['’]|c['’]est|n['’])\b",
    "es": r"\b(?:el|la|los|las|una|es|y|en|para|que|con|por|no|se|está|esto|como|pero|más|también|sino|cada)\b",
    "en": r"\b(?:the|and|is|are|to|of|in|that|for|you|with|this|it|as|but|on|was)\b",
}
_LANG_RE = {k: re.compile(v, re.IGNORECASE) for k, v in _LANG_MARKERS.items()}


def detect_lang(text):
    words = max(len(text.split()), 1)
    scores = {k: len(r.findall(text)) / words for k, r in _LANG_RE.items()}
    best, best_score = max(scores.items(), key=lambda kv: kv[1])
    # English is the safe default on weak/tied signal.
    return best if best_score >= 0.04 else "en"


# === GENRE BONUSES (English, hard) ======================================

GENRE_BONUSES = {
    "linkedin": [
        (r"\bthrilled\s+to\s+announce\b", 5, "LinkedIn cliche opening"),
        (r"(?:🎉|🚀|💡|🔥|👇){2,}", 3, "Emoji cluster"),
        (r"#\w+(?:\s+#\w+){2,}", 3, "Hashtag spam"),
    ],
    "cover_letter": [
        (r"\bi\s+am\s+writing\s+to\s+express\b", 10, "Cover letter death sentence"),
        (r"\bhighly\s+motivated\b", 5, "Highly motivated"),
    ],
    "recruiter": [
        (r"\bexciting\s+opportunity\b", 10, "Unnamed exciting opportunity"),
    ],
}
COMPILED_GENRE = {
    g: [(re.compile(p, re.IGNORECASE), b, d) for p, b, d in items]
    for g, items in GENRE_BONUSES.items()
}

# === STATISTICAL ANALYSERS (English only) ===============================

CONTRACTION_PAIRS = [
    (r"\bdo not\b", "don't"), (r"\bdoes not\b", "doesn't"), (r"\bdid not\b", "didn't"),
    (r"\bcannot\b", "can't"), (r"\bcan not\b", "can't"), (r"\bwill not\b", "won't"),
    (r"\bwould not\b", "wouldn't"), (r"\bcould not\b", "couldn't"), (r"\bshould not\b", "shouldn't"),
    (r"\bit is\b", "it's"), (r"\bthat is\b", "that's"), (r"\bwhat is\b", "what's"),
    (r"\bthere is\b", "there's"), (r"\bi am\b", "I'm"), (r"\bi have\b", "I've"),
    (r"\bi will\b", "I'll"), (r"\bi would\b", "I'd"), (r"\bwe are\b", "we're"),
    (r"\bwe have\b", "we've"), (r"\bwe will\b", "we'll"), (r"\bthey are\b", "they're"),
    (r"\bthey have\b", "they've"), (r"\byou are\b", "you're"), (r"\byou have\b", "you've"),
    (r"\byou will\b", "you'll"), (r"\bhe is\b", "he's"), (r"\bshe is\b", "she's"),
    (r"\bis not\b", "isn't"), (r"\bare not\b", "aren't"), (r"\bwas not\b", "wasn't"),
    (r"\bwere not\b", "weren't"), (r"\bhas not\b", "hasn't"), (r"\bhave not\b", "haven't"),
    (r"\bhad not\b", "hadn't"),
]

VOICE_MARKERS = [
    r"\bi think\b", r"\bi believe\b", r"\bi feel\b", r"\bin my (?:experience|opinion|view)\b",
    r"\bhonestly\b", r"\bfrankly\b", r"\bpersonally\b", r"\bprobably\b", r"\bmaybe\b",
    r"\bactually\b", r"\bto be (?:honest|fair)\b", r"\bif i'?m? honest\b", r"\byou know\b",
    r"\bto be clear\b", r"\blook,", r"\bi'?m not sure\b", r"\bi'?d say\b", r"\bi'?d argue\b",
    r"\bwhat i mean is\b", r"\bthe thing is\b",
]


def analyze_contractions(text):
    issues = []
    text_lower = text.lower()
    word_count = len(text.split())
    formal_count, formal_examples = 0, []
    for pattern, contraction in CONTRACTION_PAIRS:
        matches = list(re.finditer(pattern, text_lower))
        if matches:
            formal_count += len(matches)
            if len(formal_examples) < 3:
                formal_examples.append(f'"{matches[0].group()}" -> {contraction}')
    contraction_count = len(re.findall(r"\b\w+(?:'|’)\w+\b", text))
    if formal_count >= 3 and contraction_count == 0 and word_count > 50:
        issues.append({"type": "contraction_avoidance", "severity": 5,
                       "detail": f'{formal_count} uncontracted forms and 0 contractions in {word_count} words. Humans contract by default in non-academic writing. Examples: {"; ".join(formal_examples)}'})
    elif formal_count >= 5 and contraction_count <= 1 and word_count > 80:
        issues.append({"type": "contraction_avoidance", "severity": 3,
                       "detail": f"{formal_count} uncontracted forms vs {contraction_count} contractions. Ratio is heavily formal for non-academic text."})
    return issues


def analyze_vocabulary_diversity(text):
    issues = []
    words = re.findall(r"\b[a-z]{3,}\b", text.lower())
    if len(words) < 50:
        return issues
    window_size = min(100, len(words))
    window = words[:window_size]
    ttr = len(set(window)) / len(window)
    if window_size < 80:
        if ttr > 0.92:
            issues.append({"type": "high_vocabulary_diversity", "severity": 3, "ttr": round(ttr, 3),
                           "detail": f"Type-token ratio {ttr:.2f} across {window_size} content words — extremely high even for a short text. Near-zero repetition suggests synonym rotation."})
    elif ttr > 0.85:
        issues.append({"type": "high_vocabulary_diversity", "severity": 5, "ttr": round(ttr, 3),
                       "detail": f"Type-token ratio {ttr:.2f} — very high. Almost no word repetition across {window_size} content words, suggesting systematic synonym rotation."})
    elif ttr > 0.78:
        issues.append({"type": "high_vocabulary_diversity", "severity": 3, "ttr": round(ttr, 3),
                       "detail": f"Type-token ratio {ttr:.2f} (first {window_size} content words). AI text often shows unusually high diversity from synonym cycling."})
    return issues


def analyze_predictability(text):
    issues = []
    words = re.findall(r"\b[a-z]+\b", text.lower())
    if len(words) < 40:
        return issues
    FILLER_BIGRAMS = {
        ("it", "is"), ("this", "is"), ("there", "are"), ("there", "is"), ("we", "are"),
        ("that", "the"), ("of", "the"), ("in", "the"), ("to", "the"), ("is", "a"),
        ("is", "the"), ("it", "can"), ("can", "be"), ("such", "as"), ("as", "well"),
        ("well", "as"), ("in", "order"), ("order", "to"), ("due", "to"), ("able", "to"),
        ("need", "to"), ("has", "been"), ("have", "been"), ("will", "be"), ("this", "means"),
        ("which", "means"), ("this", "allows"), ("it", "also"), ("we", "also"), ("this", "also"),
        ("important", "to"), ("crucial", "to"), ("essential", "to"),
    }
    bigrams = list(pairwise(words))
    filler_count = sum(1 for b in bigrams if b in FILLER_BIGRAMS)
    filler_ratio = filler_count / len(bigrams) if bigrams else 0
    if filler_ratio > 0.12 and len(words) >= 50:
        issues.append({"type": "high_predictability", "severity": 3, "filler_ratio": round(filler_ratio, 3),
                       "detail": f"Filler bigram ratio {filler_ratio:.1%} ({filler_count}/{len(bigrams)}). Text relies heavily on predictable word pairings."})
    return issues


def analyze_emotional_flatness(text, genre):
    issues = []
    text_lower = text.lower()
    word_count = len(text.split())
    if word_count < 80:
        return issues
    marker_count = sum(len(re.findall(p, text_lower)) for p in VOICE_MARKERS)
    questions = len(re.findall(r"\?", text))
    exclamations = len(re.findall(r"!", text))
    first_person = len(re.findall(r"\b(?:i|my|me|mine|myself)\b", text_lower))
    total_voice_signals = marker_count + min(questions, 3) + min(exclamations, 2)
    if total_voice_signals == 0 and first_person <= 1 and word_count >= 100:
        severity = 5 if genre in ("linkedin", "cover_letter") else 3
        issues.append({"type": "emotional_flatness", "severity": severity,
                       "detail": f"Zero voice markers, {first_person} first-person pronoun(s), {questions} questions in {word_count} words. Reads like an encyclopedia entry — no opinion, no personality."})
    elif total_voice_signals <= 1 and first_person <= 2 and word_count >= 150:
        issues.append({"type": "emotional_flatness", "severity": 3,
                       "detail": f"Very few voice signals ({total_voice_signals} markers, {first_person} first-person) in {word_count} words. Leans toward the flat, detached register typical of AI."})
    return issues


def analyze_structure(text):
    """Language-agnostic structural signals."""
    issues = []
    sentences = [s.strip() for s in re.split(r"[.!?]+", text) if s.strip()]
    if len(sentences) >= 5:
        lengths = [len(s.split()) for s in sentences]
        avg = sum(lengths) / len(lengths)
        variance = sum((length - avg) ** 2 for length in lengths) / len(lengths)
        if variance < 15 and avg > 8:
            issues.append({"type": "low_sentence_variance", "severity": 3,
                           "detail": f"Avg sentence length {avg:.1f} words, variance {variance:.1f}. AI keeps sentences in a narrow band; human writing has higher burstiness."})
    paragraphs = [p.strip() for p in text.split("\n\n") if p.strip()]
    if len(paragraphs) >= 3:
        para_lengths = [len(p.split()) for p in paragraphs]
        if len(set(round(pl, -1) for pl in para_lengths)) == 1 and para_lengths[0] > 20:
            issues.append({"type": "uniform_paragraphs", "severity": 3,
                           "detail": f"All {len(paragraphs)} paragraphs are roughly the same length (~{para_lengths[0]} words). Humans vary more."})
    return issues


# === TIC MATCHING + TWO-TIER GATING =====================================

WINDOW = 750  # chars (~150 words) for the soft-cluster gate
FR_GUARD = re.compile(r"[  ][:;!?»]|«[  ]")


def _spans(text, regex):
    """Sentence/paragraph span lists for context tests."""
    return [(m.start(), m.end()) for m in regex.finditer(text) if m.group().strip()]


_SENT_RE = re.compile(r"[^.!?\n]+[.!?]*")


def collect_matches(text, lang):
    """Run every tic for the active language; return de-duplicated matches."""
    raw = []
    for regex, tic in COMPILED:
        if tic["lang"] not in (lang, "any"):
            continue
        if tic["gkind"] == "nbsp_narrow" and lang == "fr":
            continue  # narrow NBSP is correct French typography
        for m in regex.finditer(text):
            if not m.group():
                continue
            raw.append({"start": m.start(), "end": m.end(), "text": m.group(),
                        "sev": tic["sev"], "gated": tic["gated"], "fp": tic["fp"],
                        "cat": tic["cat"], "gkind": tic["gkind"], "desc": tic["desc"]})
    # Drop spans fully contained inside another kept span (collapses overlap
    # between legacy and taxonomy patterns). Prefer longer, then hard, then severe.
    raw.sort(key=lambda m: (m["start"], -(m["end"] - m["start"]), m["gated"], -m["sev"]))
    kept = []
    for m in raw:
        if any(k["start"] <= m["start"] and m["end"] <= k["end"] and k is not m for k in kept):
            continue
        kept.append(m)
    return kept


def gate(text, matches, lang):
    """Decide which matches fire and at what weight. Returns (findings, weighted_score)."""
    sent_spans = _spans(text, _SENT_RE)
    # Paragraph blocks separated by blank lines (for em-dash / tricolon density).
    blocks, pos = [], 0
    for chunk in re.split(r"\n\s*\n", text):
        blocks.append((pos, pos + len(chunk)))
        pos += len(chunk) + 2
    if not blocks:
        blocks = [(0, len(text))]

    def block_of(p):
        for i, (s, e) in enumerate(blocks):
            if s <= p <= e:
                return i
        return 0

    fired = []  # each: dict(desc, cat, sev, weight, text)
    hard_sentences = set()  # sentence indices that contain a hard tell or fired pivot

    def sent_of(p):
        for i, (s, e) in enumerate(sent_spans):
            if s <= p < e:
                return i
        return -1

    by_gkind = defaultdict(list)
    hard, soft_generic = [], []
    for m in matches:
        if not m["gated"]:
            hard.append(m)
        elif m["gkind"]:
            by_gkind[m["gkind"]].append(m)
        else:
            soft_generic.append(m)

    # --- HARD tells: always fire ---
    for m in hard:
        fired.append({**m, "weight": m["sev"]})
        hard_sentences.add(sent_of(m["start"]))

    # --- Negation->affirmation pivot (top priority, softly gated) ---
    pivots = by_gkind.get("pivot", [])
    bare_sino = by_gkind.get("es_bare_sino", [])
    # A bare "no X, sino Y" antithesis is a genuine pivot construction; once a
    # real pivot is already present it counts toward the "two or more pivots"
    # escalation rather than sitting in its own high-FP bucket.
    sino_as_pivot = bare_sino if pivots else []
    pivot_count = len(pivots) + len(sino_as_pivot)
    pivot_full = pivot_count >= 2
    # A strong pivot (sev>=8) is the canonical, low-FP antithesis cadence
    # ("it's not X, it's Y" / "ce n'est pas X, c'est Y" / "no es X, es Y") that
    # the heuristics flag as the #1 tell. It earns full weight even alone; the
    # half-weight discount stays for the weaker (sev<=7), higher-FP reframes.
    for m in pivots + sino_as_pivot:
        full = pivot_full or m["sev"] >= 8
        w = m["sev"] if full else round(m["sev"] * 0.5, 1)
        fired.append({**m, "weight": w})
        hard_sentences.add(sent_of(m["start"]))
    for m in bare_sino:  # leftover bare-sino (no real pivot): keep the strict guard
        if m not in sino_as_pivot and sent_of(m["start"]) in hard_sentences:
            fired.append({**m, "weight": m["sev"]})

    # --- Em-dash family: density-gated, never a single dash ---
    em = by_gkind.get("emdash", [])
    if em:
        per_block = Counter(block_of(m["start"]) for m in em)
        words = max(len(text.split()), 1)
        threshold = max(2, round(words / 100))
        fr_unspaced = lang == "fr" and any("—" in m["text"] and m["text"].strip() == m["text"].replace(" ", "") and re.match(r"\S—\S", m["text"]) for m in em)
        if (len(em) >= threshold and max(per_block.values()) >= 2) or fr_unspaced:
            fired.append({"desc": f"Em-dash / hyphen-surrogate overuse ({len(em)} occurrences)",
                          "cat": "typography", "sev": 4, "weight": 4 + min(len(em) - 2, 6),
                          "start": em[0]["start"], "text": "—"})

    # --- Connector overload ---
    conn = by_gkind.get("connector", [])
    if conn:
        words = max(len(text.split()), 1)
        block_first = defaultdict(int)
        for m in conn:
            block_first[block_of(m["start"])] += 1
        consecutive = any(block_first.get(i, 0) and block_first.get(i + 1, 0) for i in range(len(blocks)))
        if len(conn) > max(3, 3 * words / 500) or consecutive:
            fired.append({"desc": f"Discourse-connector overload ({len(conn)} sentence-initial connectors)",
                          "cat": "structural-connector", "sev": 5, "weight": 5 + min(len(conn) - 3, 5),
                          "start": conn[0]["start"], "text": conn[0]["text"]})

    # --- Rule-of-three: only when >=2 tricolons share a paragraph ---
    tri = by_gkind.get("tricolon", [])
    if tri:
        per_block = Counter(block_of(m["start"]) for m in tri)
        if max(per_block.values(), default=0) >= 2:
            fired.append({"desc": "Rule-of-three padding (multiple tricolons in one paragraph)",
                          "cat": "rhetorical-padding", "sev": 4, "weight": 4,
                          "start": tri[0]["start"], "text": tri[0]["text"]})

    # --- Intensifier sets ---
    inten = by_gkind.get("intensifier", [])
    if inten:
        toks = [m["text"].lower() for m in inten]
        if len(set(toks)) >= 3 or max(Counter(toks).values()) >= 3:
            fired.append({"desc": f"AI intensifier cluster ({len(set(toks))} distinct: {', '.join(sorted(set(toks))[:6])})",
                          "cat": "lexical-cliche", "sev": 5, "weight": 5,
                          "start": inten[0]["start"], "text": inten[0]["text"]})
    inten_hi = by_gkind.get("intensifier_hi", [])
    for m in inten_hi:  # critic: only when co-occurring with a hard tell or pivot
        if sent_of(m["start"]) in hard_sentences:
            fired.append({**m, "weight": m["sev"]})

    # --- Entrenched anglicisms: >=2 of the same OR alongside >=2 other tics ---
    other_cats = {f["cat"] for f in fired}
    angl = by_gkind.get("anglicism_entrenched", [])
    angl_by_desc = defaultdict(list)
    for m in angl:
        angl_by_desc[m["desc"]].append(m)
    for group in angl_by_desc.values():
        if len(group) >= 2 or len(other_cats) >= 2:
            for m in group:
                fired.append({**m, "weight": m["sev"]})

    # --- Generic soft tells: recurrence OR cluster OR co-occurrence ---
    soft_desc_counts = Counter(m["desc"] for m in soft_generic)
    for m in soft_generic:
        recurs = soft_desc_counts[m["desc"]] >= 2
        near = {s["cat"] for s in soft_generic
                if abs(s["start"] - m["start"]) <= WINDOW and s is not m}
        cluster = len(near | {m["cat"]}) >= 3
        cooccur = sent_of(m["start"]) in hard_sentences
        if recurs or cluster or cooccur:
            fired.append({**m, "weight": m["sev"]})

    # --- French typography guard: drop NBSP findings that are correct spacing ---
    if lang == "fr":
        fired = [f for f in fired if not (f["cat"] == "typography" and "NBSP" in f["desc"].upper()
                                          and FR_GUARD.search(text[max(0, f["start"] - 2):f["start"] + 2]))]

    weighted = sum(f["weight"] for f in fired)
    return fired, weighted


# === DRIVER =============================================================

def run(text, genre="general", strictness="brutal", lang="auto"):
    if lang == "auto":
        lang = detect_lang(text)

    matches = collect_matches(text, lang)
    findings, weighted = gate(text, matches, lang)

    # Genre bonuses (English only)
    genre_findings = []
    if lang == "en" and genre in COMPILED_GENRE:
        for regex, bonus, desc in COMPILED_GENRE[genre]:
            for m in regex.finditer(text):
                genre_findings.append({"desc": f"[{genre}] {desc}", "match": m.group(), "severity": bonus})
                weighted += bonus

    # Statistical analysers (English only — they assume English morphology)
    structural = analyze_structure(text)
    contraction_issues = vocab_issues = predict_issues = flatness_issues = []
    if lang == "en":
        contraction_issues = analyze_contractions(text)
        vocab_issues = analyze_vocabulary_diversity(text)
        predict_issues = analyze_predictability(text)
        flatness_issues = analyze_emotional_flatness(text, genre)
    statistical = structural + contraction_issues + vocab_issues + predict_issues + flatness_issues
    statistical_score = sum(i["severity"] for i in statistical)

    raw_score = min(round(weighted) + statistical_score, 100)
    if strictness == "gentle":
        raw_score = max(0, raw_score - 10)
    elif strictness == "firm":
        raw_score = max(0, raw_score - 5)

    word_count = max(len(text.split()), 1)
    normalized = round(weighted * 1000 / word_count, 1)
    distinct_cats = {f["cat"] for f in findings}
    signal_sources = len(distinct_cats) + (1 if genre_findings else 0) + (1 if statistical else 0)
    # A hard, low-FP opener cliche ("In an era of", "En un mundo donde", ...) is a
    # high-trust AI tell: per the heuristics, an opener buzzword contaminates the
    # reader's trust immediately. When one fires alongside any second category at
    # high density, treat the text as AI even with only two distinct categories —
    # this is the band where short overt EN/ES samples pack their slop into two
    # categories and otherwise just miss the 3-category density path.
    hard_opener = any(f["cat"] == "opener-cliche" and f.get("weight", 0) >= 6 for f in findings)
    # The FR/ES opener-cliche tells are narrow, fixed buzzphrases ("Dans un monde
    # en constante évolution", "en la era de", ...) that never fire on the clean
    # corpus; per heuristics an opener buzzword in the first sentence contaminates
    # reader trust outright (a high, low-FP tell). The largest FR/ES overt FN
    # cluster is a short sample whose slop is exactly one such opener: it clears
    # the density floor but stalls below the 2-/3-category gates. Let a lone
    # high-confidence (weight>=7) FR/ES opener carry the verdict on its own.
    lone_hard_opener = (lang in ("fr", "es")
                        and any(f["cat"] == "opener-cliche" and f.get("weight", 0) >= 7 for f in findings))
    likely_ai = (any(f.get("weight", 0) >= 9 and f.get("sev", 0) >= 9 for f in findings)
                 or (weighted >= 18 and signal_sources >= 2)
                 or (normalized >= 6 and len(distinct_cats) >= 3)
                 or (hard_opener and normalized >= 6 and len(distinct_cats) >= 2)
                 or (lone_hard_opener and normalized >= 6))

    grouped = defaultdict(list)
    for f in sorted(findings, key=lambda x: -x.get("weight", 0)):
        grouped[f["cat"]].append({"description": f["desc"], "match": f.get("text", ""),
                                  "severity": f.get("weight", f.get("sev", 0))})

    return {
        "pre_screen_score": raw_score,
        "language": lang,
        "likely_ai": likely_ai,
        "weighted_score": round(weighted, 1),
        "normalized_per_1000w": normalized,
        "distinct_categories": sorted(distinct_cats),
        "word_count": word_count,
        "genre": genre,
        "strictness": strictness,
        "tic_findings": dict(grouped),
        "genre_findings": genre_findings,
        "structural_findings": structural,
        "contraction_analysis": contraction_issues,
        "vocabulary_analysis": vocab_issues,
        "predictability_analysis": predict_issues,
        "emotional_flatness_analysis": flatness_issues,
        "finding_count": len(findings) + len(genre_findings) + len(statistical),
        "note": "Deterministic pre-screen. The qualitative audit may adjust the score based on context, voice, and intent.",
    }


def sev_label(s):
    if s >= 10:
        return "CRIT"
    if s >= 7:
        return "HIGH"
    if s >= 3:
        return "MED"
    return "LOW"


def main():
    parser = argparse.ArgumentParser(description="Slop detector pre-screen (EN/FR/ES)")
    parser.add_argument("--file", "-f", help="Path to text file")
    parser.add_argument("--lang", "-l", default="auto", choices=["auto", "en", "fr", "es"])
    parser.add_argument("--genre", "-g", default="general",
                        choices=["linkedin", "cover_letter", "recruiter", "general"])
    parser.add_argument("--strictness", "-s", default="brutal",
                        choices=["brutal", "firm", "gentle"])
    parser.add_argument("--json", action="store_true", help="Output raw JSON")
    args = parser.parse_args()

    text = open(args.file).read() if args.file else sys.stdin.read()
    if not text.strip():
        print("Error: no input text provided.", file=sys.stderr)
        sys.exit(1)

    result = run(text, genre=args.genre, strictness=args.strictness, lang=args.lang)

    if args.json:
        print(json.dumps(result, indent=2, default=str, ensure_ascii=False))
        return

    print(f"Pre-screen score: {result['pre_screen_score']}/100   (lang={result['language']}, likely_ai={result['likely_ai']})")
    print(f"Word count: {result['word_count']}   Weighted: {result['weighted_score']}   Per-1000w: {result['normalized_per_1000w']}")
    print(f"Findings: {result['finding_count']}   Categories: {', '.join(result['distinct_categories']) or 'none'}")
    print()
    for category, items in result["tic_findings"].items():
        print(f"  [{category}]")
        for item in items:
            print(f"    {sev_label(item['severity'])}: {item['description']} — \"{item['match']}\"")
        print()
    for f in result["genre_findings"]:
        print(f"  [genre] {sev_label(f['severity'])}: {f['desc']} — \"{f['match']}\"")
    for section, label in [
        ("structural_findings", "Structure"), ("contraction_analysis", "Contractions"),
        ("vocabulary_analysis", "Vocabulary"), ("predictability_analysis", "Predictability"),
        ("emotional_flatness_analysis", "Flatness"),
    ]:
        for issue in result[section]:
            print(f"  [{label}] {sev_label(issue['severity'])}: {issue['detail']}")
    print()
    print(result["note"])


if __name__ == "__main__":
    main()
