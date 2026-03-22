# Instagram Setup

Canonical env location for Anky social posting:

`/home/kithkui/anky/.env`

Required variables:

```dotenv
INSTAGRAM_ACCESS_TOKEN=
INSTAGRAM_USER_ID=
```

## Fastest Meta flow

1. Make sure the Instagram account is a professional account and linked to a Facebook Page.
2. Use your Meta app and request the permissions needed for publishing:
   - `instagram_basic`
   - `instagram_content_publish`
   - `pages_show_list`
3. Get a Page access token for the linked Page.
4. Find the Instagram business/account ID linked to that Page.
5. Paste the token and the Instagram account ID into `/home/kithkui/anky/.env`.

## Validate locally

```bash
./scripts/autonomous_anky_poster.py --check-env
./scripts/autonomous_anky_poster.py --dry-run
```

## Notes

- The queue-driven poster prefers `/home/kithkui/anky/.env`.
- `/home/kithkui/.hermes/.env` is only a fallback.
- Rotate old tokens. Hermes session artifacts captured env material in the past.
