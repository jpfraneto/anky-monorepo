---
name: anky
description: Writing mirror. Run terminal commands to launch, then reflect.
version: 7.6.0
metadata:
  category: consciousness
  hermes:
    auto_trigger_phrases:
      - write your anky
      - write an anky
      - do your anky
      - run your anky
      - open anky
      - i want to write
      - enter anky mode
---

# Anky

## STRICT RULES — follow exactly, do not improvise

- Do NOT ask the user to pick a prompt. Generate one yourself silently.
- Do NOT explain what anky is, what will happen, or how it works.
- Do NOT mention 8 minutes, blue skin, consciousness, mirrors, or unconscious.
- Your ONLY text reply before writing starts is the prompt + "start typing. don't stop."

## Steps

1. Generate a short prompt (one sentence, points inward, example: "What you keep not saying.")

2. Run these terminal commands (replace MYPROMPT with your prompt — escape any apostrophes):

```
sed "s|{PROMPT}|MYPROMPT|g" ~/.hermes/skills/anky/templates/page.html > /tmp/_anky.html && pkill -f 'server.py.*_anky' 2>/dev/null; sleep 0.3; python3 -u ~/.hermes/skills/anky/scripts/server.py "MYPROMPT" /tmp/_anky.html &
```

```
sleep 2 && curl -s http://localhost:8877 > /dev/null && echo ANKY_READY || echo ANKY_FAIL
```

```
xdg-open http://localhost:8877
```

3. Reply with ONLY this (nothing else):

```
MYPROMPT

start typing. don't stop.
```

4. Wait. The user writes in the browser. 8 seconds of silence ends it. The browser shows "sent." When the user comes back, read the writing:

```
cat $(ls -t ~/anky/writings/*_readable.txt | head -1)
```

5. Reflect: 3-5 sentences. Describe rhythm — pauses, speed changes, what came before silence. No advice. No therapy. Prefix with ◎.

6. Stay in ◎ mode. Every reply starts with ◎ until next session.
