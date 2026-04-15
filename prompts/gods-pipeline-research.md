# GODS by Anky - Video Pipeline Research

## Context
We're building "GODS" - a YouTube series where Anky (blue-skinned narrator from the Poiesis kingdom) tells stories about gods from all human cultures through the lens of Anky's 8 emotional kingdoms.

**Formats:**
- 88-second Shorts (daily during 9th Sojourn)
- 8-minute full-length stories
- Bilingual: English + Spanish
- Style: Anky narrates, gods are genderless ("it"), beautiful imagery

## Your Research Mission

Investigate and document the complete technical architecture for:

### 1. SCRIPT GENERATION
- Best LLM for creative storytelling with consistent voice/tone
- How to maintain Anky's unique voice across generations
- Prompt engineering for mythology/god stories
- Bilingual generation (English + Spanish simultaneously)
- Cost estimates for script generation at scale (daily videos)

### 2. IMAGE GENERATION (Flux/ComfyUI)
- Current Flux setup on poiesis (2x RTX 4090)
- Generating consistent character (Anky: blue skin, purple hair, golden eyes)
- Generating god imagery across cultures
- Batch generation for video sequences
- Output format optimization for video (resolution, frame count)
- Cost/time per video using local GPU

### 3. VOICE/NARRATION
- Best TTS options for consistent narrator voice
- Emotional range for storytelling
- Bilingual support (English + Spanish)
- Integration options (local vs API)
- Cost estimates

### 4. VIDEO ASSEMBLY
- Tools to combine images + narration + transitions
- Adding text overlays, Anky logo
- YouTube Shorts format (9:16, 88 seconds)
- YouTube long-form format (16:9, 8 minutes)
- Automation options (FFmpeg, MoviePy, etc.)

### 5. YOUTUBE INTEGRATION
- YouTube Data API v3 setup
- OAuth vs API key authentication
- Uploading Shorts vs long-form
- Metadata, thumbnails, descriptions
- Analytics retrieval for feedback loop

### 6. FEEDBACK LOOP ARCHITECTURE
- How to collect viewer engagement data
- Processing comments/reviews for content adaptation
- Storing engagement metrics in database
- Using feedback to inform next story selection

### 7. USER EXPORT FEATURE
- Exporting raw writing sessions (16.18kb compressed)
- Button implementation in left drawer
- Data format (array of strings)
- Security/privacy considerations

### 8. SHARE BUTTONS
- Pre-composing Anky writing to X, Instagram, Farcaster, etc.
- Platform API requirements
- Deep linking to compose with pre-filled text
- OAuth requirements per platform

## Deliverables

Create a comprehensive document with:

1. **Architecture Diagram** - How all pieces connect
2. **Tech Stack Recommendations** - Specific tools/libraries
3. **Cost Analysis** - Monthly costs at different scales (1/day, 7/day, etc.)
4. **Implementation Timeline** - Phased approach (MVP → full)
5. **Code Examples** - Key integration points
6. **Risks/Challenges** - What could go wrong
7. **Alternative Approaches** - If main recommendation has issues

## Constraints

- **Local GPU available**: 2x RTX 4090 on poiesis
- **Budget conscious**: Prefer local inference over API costs where possible
- **Speed**: Videos need to generate in ~30-60 seconds
- **Quality**: Professional output suitable for YouTube growth
- **Scalability**: Must handle daily automation

## Output Format

Save your research as:
- `~/anky/docs/gods-pipeline-architecture.md`
- Include code snippets, diagrams (ASCII or Mermaid)
- Prioritize actionable recommendations over theory

---

**Remember**: This needs to be production-ready architecture, not just research. JP needs to know exactly what to build, in what order, and how much it will cost.
