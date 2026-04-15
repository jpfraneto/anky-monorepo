#!/usr/bin/env python3
"""
Get a long-lived Instagram access token.

Usage:
    python3 ~/anky/scripts/get_instagram_token.py

You'll need:
- Facebook App ID
- Facebook App Secret
- A short-lived user token (from https://developers.facebook.com/tools/explorer/)
"""

import requests
import webbrowser
from urllib.parse import urlencode

# ============================================================
# Step 1: Get App Credentials
# ============================================================

print("="*60)
print("Instagram Token Generator")
print("="*60)

print("\n1. Get your Facebook App credentials:")
print("   https://developers.facebook.com/apps/")
print("   → Select your app → Settings → Basic")

app_id = input("\nEnter your Facebook App ID: ").strip()
app_secret = input("Enter your Facebook App Secret: ").strip()

if not app_id or not app_secret:
    print("Error: App ID and App Secret required")
    exit(1)

# ============================================================
# Step 2: Get User Token via OAuth
# ============================================================

print("\n2. Opening authorization URL in browser...")

redirect_uri = "https://localhost:8889/callback"
scope = ["instagram_basic", "instagram_content_publish"]
state = "anky_token_request"

auth_url = (
    f"https://api.instagram.com/oauth/authorize?"
    f"{urlencode({\n        'client_id': app_id,\n        'redirect_uri': redirect_uri,\n        'scope': ','.join(scope),\n        'response_type': 'code'\n    })}"
)

print(f"\nAuthorize URL: {auth_url}")
print("\nOpening in browser...")
webbrowser.open(auth_url)

print("\n3. After authorizing, you'll be redirected to:")
print(f"   {redirect_uri}?code=XXXXXXXXXX")
print("\nCopy the 'code' parameter and paste it below:")

auth_code = input("Authorization code: ").strip()

if not auth_code:
    print("Error: Authorization code required")
    exit(1)

# ============================================================
# Step 3: Exchange Code for Short-Lived Token
# ============================================================

print("\n4. Exchanging code for short-lived token...")

token_url = "https://api.instagram.com/oauth/access_token"

data = {
    'client_id': app_id,
    'client_secret': app_secret,
    'grant_type': 'authorization_code',
    'redirect_uri': redirect_uri,
    'code': auth_code
}

response = requests.get(token_url, params=data)

if response.status_code != 200:
    print(f"Error getting token: {response.text}")
    exit(1)

# Parse the token from response
token_data = response.text
short_lived_token = None

for part in token_data.split('&'):
    if part.startswith('access_token='):
        short_lived_token = part.split('=')[1]
        break

if not short_lived_token:
    print("Error: Could not extract access token")
    exit(1)

print(f"✅ Short-lived token obtained (valid 1 hour)")
print(f"   {short_lived_token[:40]}...")

# ============================================================
# Step 4: Exchange for Long-Lived Token (60 days)
# ============================================================

print("\n5. Exchanging for long-lived token (60 days)...")

fb_token_url = "https://graph.facebook.com/oauth/access_token"

fb_data = {
    'grant_type': 'fb_exchange_token',
    'client_id': app_id,
    'client_secret': app_secret,
    'fb_exchange_token': short_lived_token
}

fb_response = requests.get(fb_token_url, params=fb_data)

if fb_response.status_code != 200:
    print(f"Error exchanging token: {fb_response.text}")
    exit(1)

long_lived_token = fb_response.json().get('access_token')

if not long_lived_token:
    print("Error: Could not get long-lived token")
    exit(1)

print(f"\n✅ LONG-LIVED TOKEN OBTAINED (valid 60 days)")
print(f"\n{long_lived_token}")
print("="*60)

# ============================================================
# Step 5: Save to .env
# ============================================================

print("\n6. Adding to ~/anky/.env...")

# Read current .env
try:
    with open('~/anky/.env', 'r') as f:
        env_content = f.read()
except FileNotFoundError:
    env_content = ""

# Update or add Instagram token
import re

if 'INSTAGRAM_ACCESS_TOKEN=' in env_content:
    # Replace existing
    env_content = re.sub(
        r'INSTAGRAM_ACCESS_TOKEN=[^\n]*',
        f'INSTAGRAM_ACCESS_TOKEN={long_lived_token}',
        env_content
    )
else:
    # Add new
    env_content += f'\nINSTAGRAM_ACCESS_TOKEN={long_lived_token}\n'

# Write back
with open('~/anky/.env', 'w') as f:
    f.write(env_content)

print("✅ Token saved to ~/anky/.env")
print("\n" + "="*60)
print("TEST THE TOKEN:")
print("="*60)
print(f"\npython3 ~/anky/scripts/anky_instagram_carousel.py")
print("\nToken expires in ~60 days. Save it somewhere safe!")
print("="*60)
