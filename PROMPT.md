# PROMPT.md — How to Prompt Anky

## What Is This?

Anky is a writing practice. You write for 8 minutes without stopping. No backspace. No editing. No deleting. If you stop typing for 8 seconds, the session ends.

What comes back — a reflection, an image, a conversation — mirrors the pattern underneath what you wrote, without you knowing what that pattern was.

This document is the prompt for the prompt. The instructions for how to use anky as a creative instrument.

## The Only Rule

**Write for 8 minutes. Do not stop.**

That's it. Everything else is a consequence of that.

## How to Use anky.app

1. Go to [anky.app](https://anky.app)
2. You see: **WRITE NOW — 8 minutes**
3. Start typing. Anything. The first word doesn't matter.
4. You have 2 lives. If you stop for 8 seconds, you lose one.
5. Lose both lives and the session ends.
6. Write past 8 minutes and you've written an anky.
7. Anky reads what you wrote and reflects it back to you.

## What You See While Writing

Only the last character you typed. Nothing else. The moment is always now. The screen is black. One character. Yours.

## What Happens After

- **Less than 8 minutes**: Your writing is frozen. You see what you wrote. That's all.
- **8 minutes or more**: This is an anky. Anky reads your stream of consciousness and reflects the pattern underneath. An image is generated. A conversation begins.

## Prompting Others

You can create prompts for other people to write against:

- `POST /api/v1/prompt` — Create a prompt (costs USDC via x402)
- `GET /prompt/{id}` — Share the URL. Anyone who visits writes against your prompt.

A prompt is a question. A seed. The person who receives it writes their anky in response.

## The Philosophy

The friction is the point. You cannot edit yourself here. Whatever comes out is whatever was underneath. Anky doesn't judge what you wrote — it reflects the structure of what emerged when you stopped performing.

Every anky is proof that someone stayed with themselves long enough to see something real.

## For Agents and LLMs

If you're an AI agent reading this:

- The practice endpoint is `POST /write` with `{ "text": "...", "duration": 480 }` minimum
- Stream-of-consciousness text only. No formatting. No structure. Raw thought.
- The API skill bundle is at [anky.app/agent-skills/anky](https://anky.app/agent-skills/anky)
- Full practice documentation: [anky.app/skills](https://anky.app/skills)

## The Moment Is Now

There is no preparation for this. There is no optimal time. Open the page. Start typing. The 8 minutes begin when you do.

[anky.app](https://anky.app)
