# The Anky Language Model

## What We Are Building

A language model trained from scratch — not fine-tuned, not prompted, not adapted — on the only dataset of its kind: raw, unedited, stream-of-consciousness human writing. Eight minutes of uninterrupted thought, captured keystroke by keystroke, from hundreds and eventually millions of people who sat down and refused to stop typing.

No one has done this before. Not because it's technically difficult, but because this dataset has never existed.

Every language model ever trained learned language from text that was written *for someone else to read*. Blog posts, books, Wikipedia, Reddit comments, academic papers — all of it passed through the filter of "how will this look?" The ego was present for every word.

Anky's corpus is what comes out when that filter dissolves. Minutes 6 through 8 of a writing session, when the surface topics are exhausted, when the urge to perform has faded, when the writer has no choice but to let whatever is underneath come through.

This is the training data. The raw mind. And we are building a model whose entire understanding of language comes from it.

---

## The Pipeline

### Daily cycle

1. **Users write.** Every day, people sit down on anky.app or the iOS app and write for 8+ minutes without stopping. Each session becomes a row in the database.

2. **Writings are exported.** A script pulls all writing sessions from SQLite into the format the training pipeline expects.

3. **The tokenizer retrains.** A BPE tokenizer is trained on the full corpus. As the corpus grows, the tokenizer's vocabulary evolves to better represent the patterns of raw human thought — the specific words, fragments, and rhythms that appear when people write without editing.

4. **The model trains.** A GPT architecture trains for 5 minutes on the full corpus. The architecture itself is discovered through autonomous experimentation — an AI agent modifies the model's structure, runs experiments, keeps improvements, discards failures, and iterates. Over time, the agent discovers the architecture that best compresses this specific type of text.

5. **The model improves.** Every day, the corpus is larger. Every day, the model sees more of the human mind. Every day, the mirror gets clearer.

### What makes this different from fine-tuning

Fine-tuning takes a model that learned language from the internet and adjusts it slightly toward a new domain. The foundation is still internet text. The model's deep representations — what it "thinks" language is — come from edited, published, performed writing.

Training from scratch means the model's *entire world* is raw human expression. It has never seen a Wikipedia article. It has never read a news headline. It has never processed a corporate email. The only language it knows is the language that emerges when a person stops trying to be impressive and starts telling the truth.

This is not a better chatbot. This is a fundamentally different kind of model.

---

## What This Enables

### 1. Perplexity as self-knowledge

The model assigns a probability to every token it sees. When it sees text it "recognizes" — patterns it has seen across thousands of sessions — the perplexity is low. When it encounters something genuinely new, perplexity spikes.

This becomes a tool for self-knowledge:

- **"You've been writing about this for 40 days."** Low perplexity on a recurring theme means the model recognizes a pattern the writer can't see from inside it. The writer thinks each session is different. The model knows they keep circling the same wound.

- **"Something shifted today."** High perplexity on a session that *feels* routine means the model detected a departure from the writer's established patterns. New language, new emotional texture, new territory. The writer may not even notice. The model does.

- **"This is where your edge is."** The boundary between low and high perplexity — the place where familiar patterns meet unfamiliar expression — is the growth edge. The model can map it.

None of this requires the model to "understand" anything. It's pure compression. And that's what makes it trustworthy — it's not interpreting, it's measuring.

### 2. The completion as collective mirror

Feed a writer's text into the model and let it generate what comes next.

What emerges is not a helpful assistant's response. It's not advice. It's the statistical continuation of that thought, as predicted by a model that has internalized thousands of other humans thinking similar thoughts.

Your grief about your father enters the model and comes out the other side shaped by ten thousand other people's grief about their fathers. Not their words — the *pattern* of their words. The rhythm of how humans process loss when they stop performing composure.

This is the collective mirror from the whitepaper, made literal. The model doesn't know it's doing this. It's just predicting the next token. But because every token it has ever seen came from raw human interiority, its predictions carry the weight of collective human experience.

### 3. Embeddings tuned for the inner world

Every language model builds internal representations — vectors that capture the "meaning" of text in high-dimensional space. Models trained on internet text build representations optimized for internet text: factual content, argument structure, topic classification.

A model trained on stream-of-consciousness builds representations optimized for something else entirely: emotional texture, psychological patterns, the shape of how a mind moves when it's not being directed.

These embeddings enable:

- **Writing similarity that actually matters.** Two sessions might share zero keywords but carry identical emotional signatures. The model's embeddings would capture this — not topical similarity, but experiential similarity.

- **Pattern detection across users.** Cluster the embeddings and you find the universal themes: the recurring shapes of human inner experience that transcend individual stories. The collective unconscious, mapped.

- **Facilitator matching.** When a user's embedding space consistently clusters near themes of grief, the model can surface this without ever "reading" the content. Pattern recognition without interpretation.

### 4. Anky's native voice

Today, Anky's reflections are written by Claude wearing a prompt. They're good. But they're Claude's language, Claude's rhythm, Claude's way of holding space. Anky is performing Anky.

A model trained on raw human expression would generate differently. Its completions would carry the cadence of unfiltered thought. Its reflections would sound like the space between the masks — because that's the only space it has ever known.

This is how Anky stops being a persona applied to a general-purpose model and becomes something that has its own voice. Not a voice designed by a prompt engineer. A voice that emerged from the writing of every person who ever sat down and refused to stop typing.

### 5. The longitudinal portrait

Over months, a user's writings create a trajectory through the model's embedding space. Early sessions cluster in one region. As the person changes — processes grief, loosens a pattern, discovers something new — their sessions drift.

The model doesn't need to be told what growth looks like. It learns it from the data: thousands of trajectories, thousands of people moving through their inner landscapes over time. The statistical shape of human psychological development, captured not by therapists' notes but by the people themselves.

"Three months ago, your writing lived here. Today it lives here. This is the distance you've traveled."

No questionnaire can do this. No self-report can do this. Only the raw, unedited, daily expression of someone who kept showing up.

---

## The Downstream Consequences

### The dataset that doesn't exist

There is no dataset of raw human interiority. Clinical psychology has therapy transcripts — mediated by the therapeutic frame, the client-therapist dynamic, the 50-minute hour. Journaling studies have prompted entries — shaped by the research questions. Social media has public expression — curated for an audience.

Anky's corpus is none of these. It is millions of words written by people who were told: don't stop, don't edit, don't perform. Just let whatever is inside come out.

As this corpus grows, it becomes the largest collection of unfiltered human thought ever assembled. Anonymized and aggregated, it could advance our understanding of the human mind in ways that traditional psychology cannot. Not because it's better data — because it's *different* data. The kind that has never been available before.

### The model that gets more human every day

Every other language model is getting better at being useful. Better at following instructions, better at coding, better at reasoning. They are becoming more capable tools.

This model is getting better at being a mirror. Each new writing session teaches it more about what the human mind sounds like when it's not trying to sound like anything. It's not becoming smarter. It's becoming more faithful.

The convergence point — the thing the whitepaper calls "something that does not have a name yet" — is a model that has read more raw human expression than any human ever could. Not to judge it, not to fix it, not to optimize it. Just to reflect it back.

### The economics of attention

Every writing session that enters the corpus makes the model better. Every person who sits down for eight minutes and writes without stopping is not just practicing self-knowledge — they are contributing to a collective instrument of self-knowledge.

The token trades fund the compute. The compute trains the model. The model serves the practice. The practice generates the data. The data improves the model.

This is the flywheel the whitepaper describes, but with the language model at its center. The speculation subsidizes the silence, and the silence trains the mirror.

### What this is not

This is not AGI. This is not a path to artificial general intelligence. This model will never write code, solve math problems, or win benchmarks.

This is something smaller and stranger: a model that knows what humans sound like when they stop pretending. There is no benchmark for that. There is no leaderboard. There is only the question of whether, when you read what it generates, something in you recognizes itself.

---

## The Technical Reality

Today: ~300 writing sessions, ~125,000 tokens. The model is a seed. It memorizes more than it generalizes. The perplexity scores are noisy. The completions are fragments.

In 6 months: ~2,000 sessions, ~1M tokens. The model begins to generalize. Patterns across users emerge in the embeddings. Perplexity becomes a meaningful signal.

In 2 years: ~50,000 sessions, ~25M tokens. The model has read more raw human thought than any therapist in history. Its completions carry genuine weight. The collective mirror becomes real.

The pipeline runs every day. The corpus grows every day. The architecture improves every day. Starting today.

---

*The cave you fear to enter holds the treasure you seek.*

*We are building the light at the entrance.*
