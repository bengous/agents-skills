#!/usr/bin/env python3
"""
slop-detector pre-screen: deterministic pattern matching + statistical analysis.

Run this BEFORE the full qualitative audit to get a baseline signal.
This catches the obvious stuff. The qualitative pass catches everything else.

Usage:
    python score.py < input.txt
    python score.py --file path/to/text.txt
    python score.py --genre linkedin --strictness brutal --file text.txt
"""

import argparse
import json
import re
import sys
from collections import defaultdict
from itertools import pairwise

# === PATTERN DEFINITIONS ===

# Each pattern: (regex, category, severity_points, description)
# severity: 5=high, 3=medium, 1=low

PATTERNS = [
    # Buzzword Inflation
    (r'\b(?:leverage[ds]?|utiliz(?:e[ds]?|ing)|facilitate[ds]?|streamline[ds]?|optimize[ds]?|empower(?:s|ed|ing)?)\b', 'buzzword_inflation', 3, 'Corporate verb'),
    (r'\b(?:delve[ds]?|delving|navigat(?:e[ds]?|ing)|synthesiz(?:e[ds]?|ing)|unveil(?:s|ed|ing)?|harness(?:es|ed|ing)?|spearhead(?:s|ed|ing)?)\b', 'buzzword_inflation', 3, 'AI-favorite verb'),
    (r'\b(?:innovative|cutting[\s-]edge|state[\s-]of[\s-]the[\s-]art|next[\s-]generation|world[\s-]class)\b', 'buzzword_inflation', 3, 'Hype adjective'),
    (r'\b(?:holistic|synergy|ecosystem|paradigm|disruptive)\b', 'buzzword_inflation', 3, 'Consultant speak'),
    (r'\b(?:robust|scalable|seamless|end[\s-]to[\s-]end)\b', 'buzzword_inflation', 1, 'Tech buzzword'),
    (r'\b(?:drive|driving)\s+(?:impact|results|value|growth|change|innovation)\b', 'buzzword_inflation', 3, '"Drive X" pattern'),
    (r'\bgame[\s-]?changer\b', 'buzzword_inflation', 5, 'Game-changer'),
    (r'\btransformative\b', 'buzzword_inflation', 3, 'Transformative'),
    (r"in today'?s?\s+(?:\w+\s+)?(?:world|landscape|environment|era)", 'buzzword_inflation', 5, '"In today\'s X world"'),
    (r'\baims?\s+to\s+explore\b', 'buzzword_inflation', 3, 'Aims to explore'),
    (r'\bnavigating\s+(?:the\s+)?(?:landscape|complexities|challenges)\b', 'buzzword_inflation', 5, 'Navigating the landscape'),
    (r'\bat\s+its\s+core\b', 'buzzword_inflation', 1, 'At its core'),

    # Emotional Inflation
    (r'\bthrilled\s+to\s+(?:announce|share|join|be)\b', 'emotional_inflation', 5, 'Thrilled to X'),
    (r'\b(?:honored|humbled)\s+to\b', 'emotional_inflation', 5, 'Honored/humbled to'),
    (r"\bcouldn'?t\s+be\s+more\s+(?:excited|grateful|proud)\b", 'emotional_inflation', 5, "Couldn't be more X"),
    (r'\b(?:deeply|incredibly)\s+(?:grateful|moved|honored|passionate)\b', 'emotional_inflation', 5, 'Deeply/incredibly + emotion'),
    (r'\bpassionate\s+about\b', 'emotional_inflation', 3, 'Passionate about'),
    (r'\bincredible\s+(?:journey|team|opportunity|experience)\b', 'emotional_inflation', 3, 'Incredible + noun'),
    (r'\bnothing\s+short\s+of\b', 'emotional_inflation', 3, 'Nothing short of'),

    # Hollow Transitions
    (r'\b(?:moreover|furthermore|additionally)\b', 'hollow_transitions', 1, 'Filler transition'),
    (r"\bthat\s+(?:being|having\s+been)\s+said\b", 'hollow_transitions', 3, 'That being said'),
    (r"\blet'?s?\s+(?:dive\s+in|explore|unpack)\b", 'hollow_transitions', 5, "Let's dive in/explore"),
    (r'\bit\'?s?\s+worth\s+noting\s+that\b', 'hollow_transitions', 3, "It's worth noting"),
    (r'\bat\s+the\s+end\s+of\s+the\s+day\b', 'hollow_transitions', 3, 'At the end of the day'),
    (r'\bneedless\s+to\s+say\b', 'hollow_transitions', 3, 'Needless to say'),
    (r'\bthis\s+is\s+where\s+\w+\s+comes?\s+in\b', 'hollow_transitions', 3, 'This is where X comes in'),
    (r'\b(?:consequently|therefore|thus|hence)\b', 'hollow_transitions', 1, 'Formal causal connector'),

    # Hedging
    (r"\bit(?:'?s|\s+is)\s+important\s+to\s+note\s+that\b", 'hedging', 3, "It's important to note"),
    (r'\b(?:arguably|essentially|fundamentally)\b', 'hedging', 1, 'Filler qualifier'),
    (r'\bin\s+many\s+ways\b', 'hedging', 1, 'In many ways'),
    (r'\bcould\s+potentially\b', 'hedging', 3, 'Double hedge'),
    (r'\bmight\s+perhaps\b', 'hedging', 3, 'Double hedge'),
    (r"\bit(?:'?s|\s+is)\s+not\s+without\s+its\s+challenges\b", 'hedging', 3, 'Not without challenges'),

    # Cover Letter Specific
    (r'\bi\s+am\s+writing\s+to\s+express\s+my\s+(?:strong\s+)?interest\b', 'template_language', 5, 'Template cover letter opening'),
    (r'\bproven\s+track\s+record\b', 'template_language', 5, 'Proven track record'),
    (r'\bideal\s+candidate\b', 'template_language', 3, 'Ideal candidate'),
    (r'\bwelcome\s+the\s+opportunity\s+to\s+discuss\b', 'template_language', 3, 'Template cover letter closing'),
    (r'\bhighly\s+motivated\b', 'template_language', 3, 'Highly motivated'),

    # Recruiter Specific
    (r'\bi\s+came\s+across\s+your\s+profile\b', 'template_language', 5, 'Template recruiter opening'),
    (r'\bexciting\s+opportunity\b', 'template_language', 5, 'Exciting opportunity'),
    (r'\byou\'?d?\s+be\s+a\s+great\s+fit\b', 'template_language', 3, 'Great fit'),

    # Structural Templates
    (r'(?:here\'?s?\s+what\s+I\s+learned|here\s+are\s+(?:my|the)\s+(?:\d+\s+)?(?:key\s+)?(?:takeaways?|lessons?))', 'structural_template', 5, 'Takeaway/lessons format'),
    (r'(?:what\s+do\s+you\s+think\s*\?|agree\s*\?|thoughts\s*\?)\s*$', 'structural_template', 3, 'Engagement bait closing'),

    # Redundancy Signals
    (r'\bin\s+other\s+words\b', 'redundancy', 1, 'In other words'),
    (r'\bas\s+(?:I\s+)?(?:mentioned|said|noted)\s+(?:earlier|before|above)\b', 'redundancy', 1, 'As mentioned earlier'),

    # Rhetorical Crutches
    (r"(?:^|\.\s+)(?:the\s+)?(?:truth|reality)\s+is\b", 'rhetorical_crutch', 3, 'The truth/reality is'),
    (r"(?:^|\.\s+)(?:and\s+)?here'?s?\s+the\s+thing\b", 'rhetorical_crutch', 3, "Here's the thing"),
]

# Genre-specific bonus penalties
GENRE_BONUSES = {
    'linkedin': [
        (r'\bthrilled\s+to\s+announce\b', 5, 'LinkedIn cliche opening'),
        (r'(?:🎉|🚀|💡|🔥|👇){2,}', 3, 'Emoji cluster'),
        (r'#\w+(?:\s+#\w+){2,}', 3, 'Hashtag spam'),
    ],
    'cover_letter': [
        (r'\bi\s+am\s+writing\s+to\s+express\b', 10, 'Cover letter death sentence'),
        (r'\bhighly\s+motivated\b', 5, 'Highly motivated'),
    ],
    'recruiter': [
        (r'\bexciting\s+opportunity\b', 10, 'Unnamed exciting opportunity'),
    ],
}

# Contraction pairs: (formal form regex, what the contraction would be)
CONTRACTION_PAIRS = [
    (r'\bdo not\b', "don't"),
    (r'\bdoes not\b', "doesn't"),
    (r'\bdid not\b', "didn't"),
    (r'\bcannot\b', "can't"),
    (r'\bcan not\b', "can't"),
    (r'\bwill not\b', "won't"),
    (r'\bwould not\b', "wouldn't"),
    (r'\bcould not\b', "couldn't"),
    (r'\bshould not\b', "shouldn't"),
    (r'\bit is\b', "it's"),
    (r'\bthat is\b', "that's"),
    (r'\bwhat is\b', "what's"),
    (r'\bthere is\b', "there's"),
    (r'\bi am\b', "I'm"),
    (r'\bi have\b', "I've"),
    (r'\bi will\b', "I'll"),
    (r'\bi would\b', "I'd"),
    (r'\bwe are\b', "we're"),
    (r'\bwe have\b', "we've"),
    (r'\bwe will\b', "we'll"),
    (r'\bthey are\b', "they're"),
    (r'\bthey have\b', "they've"),
    (r'\byou are\b', "you're"),
    (r'\byou have\b', "you've"),
    (r'\byou will\b', "you'll"),
    (r'\bhe is\b', "he's"),
    (r'\bshe is\b', "she's"),
    (r'\bis not\b', "isn't"),
    (r'\bare not\b', "aren't"),
    (r'\bwas not\b', "wasn't"),
    (r'\bwere not\b', "weren't"),
    (r'\bhas not\b', "hasn't"),
    (r'\bhave not\b', "haven't"),
    (r'\bhad not\b', "hadn't"),
]

# Human voice markers — absence of these in longer texts signals emotional flatness
VOICE_MARKERS = [
    r'\bi think\b', r'\bi believe\b', r'\bi feel\b', r'\bin my (?:experience|opinion|view)\b',
    r'\bhonestly\b', r'\bfrankly\b', r'\bpersonally\b',
    r'\bprobably\b', r'\bmaybe\b', r'\bactually\b',
    r'\bto be (?:honest|fair)\b', r'\bif i\'?m? honest\b',
    r'\byou know\b', r'\bto be clear\b', r'\blook,',
    r'\bi\'?m not sure\b', r'\bi\'?d say\b', r'\bi\'?d argue\b',
    r'\bwhat i mean is\b', r'\bthe thing is\b',
]


def find_matches(text, genre='general'):
    """Run all patterns against text. Return findings grouped by category."""
    findings = defaultdict(list)
    total_score = 0

    text_lower = text.lower()

    for pattern, category, severity, description in PATTERNS:
        for match in re.finditer(pattern, text_lower, re.IGNORECASE | re.MULTILINE):
            findings[category].append({
                'match': match.group(),
                'position': match.start(),
                'severity': severity,
                'description': description,
            })
            total_score += severity

    # Genre bonuses
    if genre in GENRE_BONUSES:
        for pattern, bonus, description in GENRE_BONUSES[genre]:
            for match in re.finditer(pattern, text_lower, re.IGNORECASE):
                findings['genre_bonus'].append({
                    'match': match.group(),
                    'position': match.start(),
                    'severity': bonus,
                    'description': f'[{genre}] {description}',
                })
                total_score += bonus

    return findings, min(total_score, 100)


def analyze_contractions(text, genre='general'):
    """Detect absence of contractions in genres where they're expected."""
    issues = []
    text_lower = text.lower()
    word_count = len(text.split())

    # Count formal forms that could be contracted
    formal_count = 0
    formal_examples = []
    for pattern, contraction in CONTRACTION_PAIRS:
        matches = list(re.finditer(pattern, text_lower))
        if matches:
            formal_count += len(matches)
            if len(formal_examples) < 3:
                formal_examples.append(f'"{matches[0].group()}" -> {contraction}')

    # Count actual contractions present
    contraction_count = len(re.findall(r"\b\w+(?:'|')\w+\b", text))

    if formal_count >= 3 and contraction_count == 0 and word_count > 50:
        issues.append({
            'type': 'contraction_avoidance',
            'detail': f'Found {formal_count} uncontracted forms and 0 contractions in {word_count} words. Humans contract by default in non-academic writing. Examples: {"; ".join(formal_examples)}',
            'severity': 5,
            'formal_count': formal_count,
            'contraction_count': contraction_count,
        })
    elif formal_count >= 5 and contraction_count <= 1 and word_count > 80:
        issues.append({
            'type': 'contraction_avoidance',
            'detail': f'Found {formal_count} uncontracted forms vs {contraction_count} contractions. Ratio is heavily formal for non-academic text.',
            'severity': 3,
            'formal_count': formal_count,
            'contraction_count': contraction_count,
        })

    return issues


def analyze_vocabulary_diversity(text):
    """Detect suspiciously high vocabulary diversity (synonym cycling)."""
    issues = []
    words = re.findall(r'\b[a-z]{3,}\b', text.lower())

    if len(words) < 50:
        return issues

    # Type-token ratio on a fixed window to avoid length bias
    window_size = min(100, len(words))
    window = words[:window_size]
    ttr = len(set(window)) / len(window)

    # Short texts (50-79 words) naturally have high TTR — only flag extremes
    if window_size < 80:
        if ttr > 0.92:
            issues.append({
                'type': 'high_vocabulary_diversity',
                'detail': f'Type-token ratio: {ttr:.2f} across {window_size} content words — extremely high even for a short text. Near-zero word repetition suggests synonym rotation.',
                'severity': 3,
                'ttr': round(ttr, 3),
            })
    else:
        # Longer texts: standard thresholds
        if ttr > 0.85:
            issues.append({
                'type': 'high_vocabulary_diversity',
                'detail': f'Type-token ratio: {ttr:.2f} — very high. Almost no word repetition across {window_size} content words, suggesting systematic synonym rotation.',
                'severity': 5,
                'ttr': round(ttr, 3),
            })
        elif ttr > 0.78:
            issues.append({
                'type': 'high_vocabulary_diversity',
                'detail': f'Type-token ratio: {ttr:.2f} (first {window_size} content words). AI text often shows unusually high diversity from synonym cycling. Humans repeat favorite words more.',
                'severity': 3,
                'ttr': round(ttr, 3),
            })

    return issues


def analyze_predictability(text):
    """Rough bigram predictability proxy. High frequency of common filler bigrams = low surprise."""
    issues = []
    words = re.findall(r'\b[a-z]+\b', text.lower())

    if len(words) < 40:
        return issues

    # Common filler bigrams that AI overproduces
    FILLER_BIGRAMS = {
        ('it', 'is'), ('this', 'is'), ('there', 'are'), ('there', 'is'),
        ('we', 'are'), ('that', 'the'), ('of', 'the'), ('in', 'the'),
        ('to', 'the'), ('is', 'a'), ('is', 'the'), ('it', 'can'),
        ('can', 'be'), ('such', 'as'), ('as', 'well'), ('well', 'as'),
        ('in', 'order'), ('order', 'to'), ('due', 'to'), ('able', 'to'),
        ('need', 'to'), ('has', 'been'), ('have', 'been'), ('will', 'be'),
        ('this', 'means'), ('which', 'means'), ('this', 'allows'),
        ('it', 'also'), ('we', 'also'), ('this', 'also'),
        ('important', 'to'), ('crucial', 'to'), ('essential', 'to'),
    }

    bigrams = list(pairwise(words))
    filler_count = sum(1 for b in bigrams if b in FILLER_BIGRAMS)
    filler_ratio = filler_count / len(bigrams) if bigrams else 0

    if filler_ratio > 0.12 and len(words) >= 50:
        issues.append({
            'type': 'high_predictability',
            'detail': f'Filler bigram ratio: {filler_ratio:.1%} ({filler_count}/{len(bigrams)}). Text relies heavily on predictable word pairings.',
            'severity': 3,
            'filler_ratio': round(filler_ratio, 3),
        })

    return issues


def analyze_emotional_flatness(text, genre='general'):
    """Detect absence of human voice markers (opinion, hedges, first-person reactions)."""
    issues = []
    text_lower = text.lower()
    word_count = len(text.split())

    if word_count < 80:
        return issues

    marker_count = 0
    for pattern in VOICE_MARKERS:
        marker_count += len(re.findall(pattern, text_lower))

    questions = len(re.findall(r'\?', text))
    exclamations = len(re.findall(r'!', text))
    first_person = len(re.findall(r'\b(?:i|my|me|mine|myself)\b', text_lower))

    total_voice_signals = marker_count + min(questions, 3) + min(exclamations, 2)

    if total_voice_signals == 0 and first_person <= 1 and word_count >= 100:
        severity = 5 if genre in ('linkedin', 'cover_letter') else 3
        issues.append({
            'type': 'emotional_flatness',
            'detail': f'Zero voice markers, {first_person} first-person pronoun(s), {questions} questions in {word_count} words. Text reads like an encyclopedia entry — no opinion, no personality, no human reaction.',
            'severity': severity,
            'voice_markers': marker_count,
            'first_person': first_person,
        })
    elif total_voice_signals <= 1 and first_person <= 2 and word_count >= 150:
        issues.append({
            'type': 'emotional_flatness',
            'detail': f'Very few voice signals ({total_voice_signals} markers, {first_person} first-person) in {word_count} words. Text leans toward the flat, detached register typical of AI.',
            'severity': 3,
            'voice_markers': marker_count,
            'first_person': first_person,
        })

    return issues


def analyze_structure(text):
    """Check for structural AI signals that regex can't catch well."""
    issues = []
    sentences = re.split(r'[.!?]+', text)
    sentences = [s.strip() for s in sentences if s.strip()]

    if len(sentences) >= 5:
        lengths = [len(s.split()) for s in sentences]
        avg = sum(lengths) / len(lengths)
        variance = sum((length - avg) ** 2 for length in lengths) / len(lengths)
        if variance < 15 and avg > 8:
            issues.append({
                'type': 'low_sentence_variance',
                'detail': f'Avg sentence length: {avg:.1f} words, variance: {variance:.1f}. AI keeps sentences in a narrow band. Human writing has higher "burstiness."',
                'severity': 3,
            })

    paragraphs = [p.strip() for p in text.split('\n\n') if p.strip()]
    if len(paragraphs) >= 3:
        para_lengths = [len(p.split()) for p in paragraphs]
        if len(set(round(pl, -1) for pl in para_lengths)) == 1 and para_lengths[0] > 20:
            issues.append({
                'type': 'uniform_paragraphs',
                'detail': f'All {len(paragraphs)} paragraphs are roughly the same length (~{para_lengths[0]} words). Humans vary more.',
                'severity': 3,
            })

    numbered = re.findall(r'^\s*\d+[\.\)]\s', text, re.MULTILINE)
    if len(numbered) == 3:
        issues.append({
            'type': 'rule_of_three',
            'detail': 'Exactly 3 numbered points. The "3 takeaways" template is a strong AI signal.',
            'severity': 3,
        })

    return issues


def run(text, genre='general', strictness='brutal'):
    """Full pre-screen. Returns structured results."""
    findings, pattern_score = find_matches(text, genre)

    structural = analyze_structure(text)
    contraction_issues = analyze_contractions(text, genre)
    vocab_issues = analyze_vocabulary_diversity(text)
    predict_issues = analyze_predictability(text)
    flatness_issues = analyze_emotional_flatness(text, genre)

    all_statistical = structural + contraction_issues + vocab_issues + predict_issues + flatness_issues
    statistical_score = sum(i['severity'] for i in all_statistical)
    raw_score = min(pattern_score + statistical_score, 100)

    if strictness == 'gentle':
        raw_score = max(0, raw_score - 10)
    elif strictness == 'firm':
        raw_score = max(0, raw_score - 5)

    word_count = len(text.split())

    result = {
        'pre_screen_score': raw_score,
        'word_count': word_count,
        'genre': genre,
        'strictness': strictness,
        'pattern_findings': {k: v for k, v in findings.items()},
        'structural_findings': structural,
        'contraction_analysis': contraction_issues,
        'vocabulary_analysis': vocab_issues,
        'predictability_analysis': predict_issues,
        'emotional_flatness_analysis': flatness_issues,
        'finding_count': sum(len(v) for v in findings.values()) + len(all_statistical),
        'note': 'This is a deterministic pre-screen. The full qualitative audit may adjust the score based on context, voice, and intent.',
    }

    return result


def main():
    parser = argparse.ArgumentParser(description='Slop detector pre-screen')
    parser.add_argument('--file', '-f', help='Path to text file')
    parser.add_argument('--genre', '-g', default='general',
                        choices=['linkedin', 'cover_letter', 'recruiter', 'general'])
    parser.add_argument('--strictness', '-s', default='brutal',
                        choices=['brutal', 'firm', 'gentle'])
    parser.add_argument('--json', action='store_true', help='Output raw JSON')
    args = parser.parse_args()

    if args.file:
        with open(args.file) as f:
            text = f.read()
    else:
        text = sys.stdin.read()

    if not text.strip():
        print('Error: no input text provided.', file=sys.stderr)
        sys.exit(1)

    result = run(text, genre=args.genre, strictness=args.strictness)

    if args.json:
        print(json.dumps(result, indent=2, default=str))
    else:
        print(f"Pre-screen score: {result['pre_screen_score']}/100")
        print(f"Word count: {result['word_count']}")
        print(f"Findings: {result['finding_count']}")
        print()

        def sev_label(s):
            if s >= 10: return 'CRIT'
            if s >= 5: return 'HIGH'
            if s >= 3: return 'MED'
            return 'LOW'

        for category, items in result['pattern_findings'].items():
            if items:
                label = category.replace('_', ' ').title()
                print(f"  [{label}]")
                for item in items:
                    print(f"    {sev_label(item['severity'])}: {item['description']} — \"{item['match']}\"")
                print()

        for section, label in [
            ('structural_findings', 'Structure'),
            ('contraction_analysis', 'Contractions'),
            ('vocabulary_analysis', 'Vocabulary'),
            ('predictability_analysis', 'Predictability'),
            ('emotional_flatness_analysis', 'Flatness'),
        ]:
            for issue in result[section]:
                print(f"  [{label}] {sev_label(issue['severity'])}: {issue['detail']}")

        print()
        print(result['note'])


if __name__ == '__main__':
    main()
