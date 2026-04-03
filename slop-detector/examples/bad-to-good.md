# Examples: Bad → Diagnosis → Better

These examples calibrate what the skill should produce. Read them for tone, granularity, and severity calibration.

---

## Example 1: LinkedIn Post (Score: 78)

### Input
```
Thrilled to announce that I've joined Acme Corp as their new Head of Product! 🎉

After an incredible journey at my previous company, where I had the privilege of working alongside some truly exceptional individuals, I'm excited to embark on this new chapter.

At Acme, I'll be leading the product organization, driving innovation, and delivering value to our customers. I'm passionate about building products that make a real difference.

Three things I'm most excited about:
1. The incredible team already in place
2. The massive opportunity in the market
3. The company's commitment to innovation

I'm deeply grateful for everyone who supported me along the way. Here's to new beginnings! 🚀

What's the best advice you've received when starting a new role? 👇
```

### Diagnosis
```
## Slop Score: 78/100

## Diagnosis
Template LinkedIn announcement. Every element — the "thrilled to announce" opening, the numbered list, the gratitude performance, the engagement-bait closing question — follows the exact pattern that signals AI or coached-by-AI writing. Zero specifics about what Acme does, why this person cares, or what they'll actually do.

## Findings

### Buzzword Inflation
- "driving innovation" — high. Says nothing. What will you actually build?
- "delivering value to our customers" — high. This is the job description of every product person who has ever lived.
- "passionate about building products that make a real difference" — high. Remove and nothing is lost.

### Emotional Inflation
- "Thrilled to announce" — high. The single most AI-detectable LinkedIn opening.
- "incredible journey" — medium. Which part was incredible? Say that instead.
- "truly exceptional individuals" — medium. Name one. Or say what they taught you.
- "deeply grateful" — medium. Performative without specifics.

### Structural Templates
- Numbered list of "things I'm excited about" — high. All three items are generic.
- Closing with a question to readers — medium. Engagement bait.

### Fake Specificity
- "Head of Product" is real, but everything else is filler. No mention of what Acme builds, what the product challenges are, or what drew this person to the role.
```

### Better
```
I just started as Head of Product at Acme Corp. They make developer tools for infrastructure teams — the kind of thing where a 30-second improvement in deploy time saves thousands of engineering hours per year.

I took the job because their current product has strong adoption but a messy onboarding experience, and that's exactly the kind of problem I like solving. At [previous company], I rebuilt the first-run experience and cut time-to-value from 3 days to 4 hours. I want to do something similar here.

First priority: sit with the support team for a week and read every ticket from the last quarter.
```

**Why this is better**: Specific company, specific problem, specific plan, specific past achievement. No performed emotions. The reader learns something about both the person and the company.

---

## Example 2: Cover Letter Opening (Score: 85)

### Input
```
Dear Hiring Manager,

I am writing to express my strong interest in the Senior Software Engineer position at TechCo. With over 8 years of experience in software development and a proven track record of delivering high-quality solutions, I am confident that my skills and experience make me an ideal candidate for this role.

Throughout my career, I have demonstrated expertise in full-stack development, utilizing technologies such as React, Node.js, Python, and AWS. I have successfully led cross-functional teams and delivered projects that have driven significant business impact.
```

### Diagnosis
```
## Slop Score: 85/100

## Diagnosis
This is the default cover letter that every AI and every bad template produces. Every sentence could be copy-pasted into any application for any job at any company. The hiring manager has read this letter 500 times this week.

## Findings

### Buzzword Inflation
- "proven track record" — high. Meaningless without proof.
- "high-quality solutions" — high. What solutions? What quality?
- "driven significant business impact" — high. How significant? What impact?
- "utilizing technologies such as" — medium. Just name them naturally.

### Hedging
- "I am confident that my skills and experience make me an ideal candidate" — high. Never tell someone you're ideal. Show it.

### Structural Templates
- "I am writing to express my interest in" — high. The most template opening possible.
- Technology list without context — medium. React and Node.js doing what?

### Fake Specificity
- "8 years of experience" — real but ungrounded. 8 years doing what, specifically?
- "cross-functional teams" — medium. How many people? Which functions?
```

### Better
```
I've spent the last three years at DataCorp building the real-time analytics pipeline that processes 2M events/second. Before that, I was at StartupX where I took their monolith apart and moved to microservices — painful, but we went from weekly deploys to continuous.

I'm applying because TechCo's observability product has a gap I've spent years thinking about: your trace visualization doesn't handle async fan-out well, and I've built exactly that at DataCorp using [specific approach]. I'd like to bring that experience to your team.
```

**Why this is better**: Leads with what they actually built, not what they are. References something specific about TechCo's product. The reader knows within 10 seconds what this person has done and why they want this specific job.

---

## Example 3: Text That Should Score Low (Score: 12)

### Input
```
We shipped the new checkout flow last Thursday. Conversion went from 2.1% to 3.4% in the first week — mostly from reducing the number of form fields from 11 to 4.

The payment step still has a 15% drop-off. I think the address autocomplete is broken on Safari. Looking into it tomorrow.
```

### Diagnosis
```
## Slop Score: 12/100

## Diagnosis
This is clean, specific, human writing. Direct claims with real numbers. No filler. Admits an unresolved problem. Has a clear next action.

## Findings

### Minor
- "I think" before "the address autocomplete is broken" — low. Minor hedge, but defensible since they haven't confirmed yet.

## Verdict
Leave as is. This is how humans communicate results.
```

---

## Example 4: Subtle Slop (Score: 45)

### Input
```
Our team has been working hard on improving the user experience of our platform. Over the past quarter, we made several key improvements to the onboarding flow, which has led to a notable increase in user retention.

Some of the changes include a streamlined signup process, better error messaging, and an improved dashboard layout. We're continuing to iterate on these features based on user feedback.

Looking ahead, we're planning to focus on mobile optimization and accessibility improvements in Q2.
```

### Diagnosis
```
## Slop Score: 45/100

## Diagnosis
Not terrible — it has structure and says something real. But every specific is wrapped in a buffer of corporate generality. The facts are there; they're just buried.

## Findings

### Buzzword Inflation
- "streamlined signup process" — medium. How was it streamlined? What changed?
- "improving the user experience of our platform" — medium. Vague.

### Hedging
- "notable increase" — medium. How notable? Give the number.
- "several key improvements" — low. Just say what they were.

### Redundancy
- "We're continuing to iterate on these features based on user feedback" — medium. Standard filler sentence. Says nothing that isn't assumed.

## Line Edits
1. "Our team has been working hard on improving the user experience of our platform" → "We redesigned the onboarding flow this quarter."
2. "which has led to a notable increase in user retention" → "7-day retention is up from 34% to 41%." (or whatever the real number is)
3. "a streamlined signup process" → "we cut the signup from 6 steps to 2"
4. Delete "We're continuing to iterate on these features based on user feedback." entirely.
5. "Looking ahead, we're planning to focus on mobile optimization and accessibility improvements in Q2." → "Q2: mobile redesign and WCAG 2.1 AA compliance."
```

**Why this is a 45 and not higher**: The text has real content underneath the padding. It's not empty — it's just wrapped in too much cotton. Line edits fix it; a full rewrite isn't necessary.

---

## Example 5: The Encyclopedia Voice (Score: 55)

### Input
```
Effective communication is a fundamental skill that professionals must develop throughout their careers. It encompasses verbal, written, and nonverbal dimensions, each requiring distinct competencies.

Organizations that prioritize communication excellence tend to exhibit higher employee engagement and superior operational outcomes. Research indicates that miscommunication costs enterprises approximately $12,500 per employee annually.

To enhance communication capabilities, individuals should consider several approaches. Active listening constitutes a critical component, as it enables deeper comprehension of colleagues' perspectives. Additionally, crafting clear and concise messages reduces the likelihood of misunderstanding. Furthermore, soliciting feedback allows for continuous refinement of one's communication methodology.

It is also worth noting that digital communication channels present unique challenges. Email and messaging platforms do not convey tone effectively, which can lead to misinterpretation. Professionals would benefit from being deliberate about channel selection based on message complexity and emotional content.
```

### Diagnosis
```
## Slop Score: 55/100

## Diagnosis
This text demonstrates three AI signals that often co-occur: zero contractions in conversational-register writing, emotionally flat encyclopedic tone, and systematic synonym rotation. It reads like a Wikipedia article about communication — factually structured, carefully neutral, with no human behind it. The $12,500 statistic is the only specific detail, and it's suspiciously round and unsourced.

## Findings

### Contraction Avoidance (Category 11)
- "It is also worth noting" — should be "It's" (or better: delete the whole phrase)
- "do not convey tone" — "don't" is natural here
- "must develop", "should consider", "would benefit" — consistent avoidance of informal forms
- Zero contractions in 150+ words of advice-register writing: high

### Emotional Flatness (Category 5b)
- No first-person opinion anywhere. No "I think", "in my experience", "honestly"
- Every sentence is a balanced declarative statement
- Who is saying this? A textbook. Not a person.
- Severity: high (this is a topic where personal experience would be natural)

### Vocabulary Diversity (Category 12)
- "encompasses" → "requiring" → "constitutes" → "enables" → "soliciting" → "refinement" → "convey" — every verb is different, each slightly more formal than necessary
- "competencies" / "capabilities" / "methodology" — three ways to say "skills" and "methods"
- No word repetition at all. Humans repeat. This text cycles.

### Hollow Transitions
- "Additionally" + "Furthermore" + "It is also worth noting" — three in four paragraphs: medium

### Fake Specificity
- "$12,500 per employee annually" — suspiciously round, no source: medium

## Line Edits
1. Delete entire first paragraph. It says nothing the reader doesn't already know.
2. "Organizations that prioritize communication excellence tend to exhibit higher employee engagement" → "Teams that communicate well perform better." (10 words instead of 13, same meaning, no jargon)
3. "Active listening constitutes a critical component, as it enables deeper comprehension of colleagues' perspectives" → "Listen before you respond. You'll catch things you'd otherwise miss."
4. Delete "It is also worth noting that" entirely. Start with "Digital channels..."
5. "do not convey tone effectively" → "don't carry tone"
```

### Better
```
Most communication advice sounds like a textbook because it is one. Here's what I've actually found works after 10 years of managing remote teams.

Listen more than you talk. Not "active listening" as a framework — just shut up and pay attention. You'll catch problems three emails earlier.

Write shorter messages. If your Slack message needs a scroll, it should've been a call. If your email needs headers, it should've been a doc.

Pick the right channel for the weight of the message. "Great job on the release" works in Slack. "Your role is changing" does not.
```

**Why this is better**: Has a clear voice (slightly blunt, experience-based). Uses contractions naturally. Repeats "you'll" and "your" without cycling synonyms. Has opinion. Breaks its own rules a little (the last paragraph is a parallel structure — but it's earned, not default). The reader knows a specific person wrote this.
