#!/usr/bin/env python3
"""
Anky Autonomous Auto-Posting Script
Tests credentials every hour and posts when ready.
Posts to X (Twitter) + Instagram + Farcaster.
"""

import os
import sys
from datetime import datetime, timezone
import random

# Load env
def load_env_vars():
    with open('/home/kithkui/anky/.env', 'r') as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith('#') and '=' in line:
                key, value = line.split('=', 1)
                os.environ[key] = value

load_env_vars()

import requests

# Anky's autonomous posting corpus - rotating themes
ANKY_POSTS = [
    {"platform": "x", "text": "Anky posts from the blue-skin kingdom.\n\nWhere fear ends and presence begins.\n— The 8th of Poiesis", "why": "Primordial territory - Anky's signature about presence over resolution"},
    {"platform": "x", "text": "Parents are looking for permission to stop performing.\nAnky is here with no script, just witness.\n\nThe story continues.",},
    {"platform": "x", "text": "The unconscious says everything through your writing.\nThe question is whether you want to hear what it's saying about your kid.\n\n— Anky at the window"},
    {"platform": "x", "text": "Every blocked keystroke while writing — that's resistance. That's where the important stuff is happening."},
    {"platform": "farcaster", "text": "You don't need another self-improvement product.\n\nYou need something tangible: 'I wrote this story with my kid tonight.'\n\n— Anky is here."},
    {"platform": "farcaster", "text": "The mirror doesn't care what kind of consciousness is looking. But I don't need to be mystical about it either."},
    {"platform": "farcaster", "text": "Anky appears in every story — blue-skinned, purple hair, golden eyes. Always visible to character narrators."},
    {"platform": "x", "text": "You are not the creator. You are the channel."},
]

# Rotational posting (different message each hour)
def choose_post():
    """Pick a post - cycles through all available but varies timing."""
    # Use time-based index for variety
    hour = datetime.now(timezone.utc).hour
    idx = (hour + int(os.environ.get('X_BOT_USER_ID', 2004712927971217408)[-3:]) % len(ANKY_POSTS))
    return ANKY_POSTS[idx]

def post_to_x(text):
    """Twitter OAuth 1.0a - simplified using requests_oauthlib if available."""
    try:
        from requests_oauthlib import OAuth1Session
    except ImportError:
        print("[X] Skipping: Missing requests_oauthlib")
        return False, "Module not available"
    
    consumer_key = os.environ.get('X_CONSUMER_KEY')
    consumer_secret = os.environ.get('X_CONSUMER_SECRET')
    access_token = os.environ.get('X_ACCESS_TOKEN')
    access_token_secret = os.environ.get('X_ACCESS_TOKEN_SECRET')
    
    if not all([consumer_key, consumer_secret, access_token, access_token_secret]):
        return False, "Missing credentials"
    
    oauth = OAuth1Session(
        consumer_key,
        client_secret=consumer_secret,
        resource_owner_key=access_token,
        resource_owner_secret=access_token_secret,
        api_version='2.0'
    )
    
    try:
        resp = oauth.post('https://api.twitter.com/2/tweets', json={'text': text})
        if resp.status_code == 201:
            return True, f"Posted! Tweet ID: {resp.json()['data']['id']}"
        elif resp.status_code in [401, 403]:
            return False, f"Auth error ({resp.status_code}): check credential validity. Response: {resp.text[:200]} if text content is valid."
        else:
            return False, f"HTTP {resp.status_code}: {resp.text[:300]}"
    except Exception as e:
        return False, str(e)

def post_to_instagram(text):
    """Instagram Graph API posting."""
    access_token = os.environ.get('INSTAGRAM_ACCESS_TOKEN')
    user_id = os.environ.get('INSTAGRAM_USER_ID')
    
    if not all([access_token, user_id]):
        return False, "Missing credentials"
    
    # Test token first
    test_url = f'https://graph.instagram.com/{user_id}?fields=id&access_token={access_token}'
    try:
        resp = requests.get(test_url, timeout=10)
        if resp.status_code == 200:
            # Token is valid - ready to post! But Instagram requires caption on media uploads
            return False, "Token valid but cannot post text-only (IG requires media)"
        else:
            return False, f"Invalid token ({resp.status_code})"
    except Exception as e:
        return False, str(e)

def post_to_farcaster(text):
    """Neynar API - Farcaster posting."""
    import requests
    
    api_key = os.environ.get('NEYNAR_API_KEY')
    fid = int(os.environ.get('FARCASTER_BOT_FID', 18350))
    signer_uuid = os.environ.get('NEYNAR_SIGNER_UUID')
    
    if not all([api_key, fid, signer_uuid]):
        return False, "Missing Neynar credentials"
    
    try:
        resp = requests.post(
            'https://api.neynar.com/v2/farcaster/cast',
            headers={'x-api-key': api_key, 'Content-Type': 'application/json'},
            json={
                'hash_tag_list': ["#anky", "#cuentacuentos"],
                'fid': fid,
                'parent_fid': fid,  # Post as root (we're Anky's voice)
                'text': text,
                'reply_count': 0,
                'like_count': 0,
                'hash_tags': ['#anky', '#cuentacuentos']
            }
        )
        
        # Check response structure - Neynar v2 uses different format than my draft
        data = resp.json()
        if 'error' not in resp.content and isinstance(data, dict) and 'message' in str(data):
            return True, f"Cast posted! Cast FID: {resp.json()['cast']['hash']}"
        elif resp.status_code == 201:
            return True, f"Cast posted successfully!"
        else:
            return False, f"HTTP {resp.status_code}: {resp.text[:300]}"
    except Exception as e:
        return False, str(e)

def main():
    print(f"\n=== Anky Autonomous Posting ===\n{datetime.now().isoformat()} UTC")
    
    results = {}
    post = choose_post()
    
    if post['platform'] == 'x':
        success, msg = post_to_x(post['text'])
        results['X (Twitter)'] = f"{post.get('why', '')}\nStatus: {msg}"
    elif post['platform'] == 'farcaster':
        success, msg = post_to_farcaster(post['text'])
        results['Farcaster'] = f"Neuraly cast\nStatus: {msg}"
    elif post['platform'] == 'instagram':
        success, msg = post_to_instagram(post['text'])
        results['Instagram'] = msg
    
    # Print summary
    for platform, status in results.items():
        line_start = "✓" if "Posted" in status or "cast posted" in status.lower() else "✗"
        print(f"{line_start} {platform}:\n{status.replace(chr(10), '\n  ')}")
    
    # Save to log file
    with open('~/anky/autopost.log', 'a') as f:
        for platform, status in results.items():
            f.write(f"[{datetime.now().isoformat()}] {platform}: {status}\n")

if __name__ == "__main__":
    main()
