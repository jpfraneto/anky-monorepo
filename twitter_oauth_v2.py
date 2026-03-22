#!/usr/bin/env python3
"""
Pure Python Twitter OAuth 1.0a signature generator for v2 API posting.
No external dependencies - uses only hashlib, hmac, base64 from stdlib.
"""
import os
import sys
import hashlib
import hmac
import base64
from datetime import datetime, timezone
from urllib.parse import quote_plus

def load_env(file_path):
    """Load environment variables from .env file (no python-dotenv dependency)."""
    try:
        with open(file_path, 'r') as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith('#') and '=' in line:
                    key, value = line.split('=', 1)
                    os.environ[key] = value
    except FileNotFoundError:
        print(f"ERROR: {file_path} not found")
        sys.exit(1)

def twitter_oauth_sign(consumer_key, consumer_secret, access_token, access_token_secret):
    """Generate Twitter OAuth 1.0a headers for v2 API."""
    timestamp = str(int(datetime.now(timezone.utc).timestamp()))
    nonce = f"{timestamp}{os.urandom(16).hex()}"[:64]
    
    # Required OAuth parameters
    params = {
        'oauth_consumer_key': consumer_key,
        'oauth_token': access_token,
        'oauth_signature_method': 'HMAC-SHA1',
        'oauth_timestamp': timestamp,
        'oauth_nonce': nonce[8:] or nonce[:32],
        'oauth_version': '1.0'
    }
    
    # Signature base string: POST&BASE64(BASE_URL)&BASE64(PARAMETERS)
    sorted_params = sorted(params.items())
    params_encoded = '&'.join(f"{quote_plus(k)}={quote_plus(v)}" for k, v in sorted_params)
    
    base_url = quote_plus('https://api.twitter.com/2/tweets', safe='')
    sig_base = f"POST&{base_url}&{quote_plus(params_encoded, safe='')}"
    
    # Key: consumer_secret&amp;access_token_secret (both URL-encoded)
    key_string = f"{quote_plus(consumer_secret, safe='') }&{quote_plus(access_token_secret, safe='')}"
    
    # HMAC-SHA1 signature (base64-encoded per OAuth spec)
    sig_bytes = hmac.new(
        key_string.encode('utf-8'),
        sig_base.encode('utf-8'),
        hashlib.sha1
    ).digest()
    oauth_signature = base64.b64encode(sig_bytes).decode('utf-8')
    
    # Build OAuth header - Twitter v2 requires quotes around ALL parameter values
    auth_params = [
        f"oauth_consumer_key={quote_plus(consumer_key, safe='')}",
        f"oauth_token={quote_plus(access_token, safe='')}",
        "oauth_signature_method=HMAC-SHA1",
        f"oauth_timestamp={timestamp}",
        f"oauth_nonce={quote_plus(nonce[8:], safe='')}",
        "oauth_version=1.0",
        f"oauth_signature={quote_plus(oauth_signature, safe='')}"
    ]
    
    auth_header = f"OAuth {', '.join(auth_params)}"
    
    return {
        'Authorization': auth_header,
        'Content-Type': 'application/json'
    }

if __name__ == '__main__':
    # Load credentials
    load_env('/home/kithkui/anky/.env')
    
    # Load credentials from env
    c_key = os.environ.get('X_CONSUMER_KEY', '')
    c_secret = os.environ.get('X_CONSUMER_SECRET', '')
    access_t = os.environ.get('X_ACCESS_TOKEN', '')
    acc_secret = os.environ.get('X_ACCESS_TOKEN_SECRET', '')
    
    if not all([c_key, access_t]):
        print("ERROR: Twitter credentials not found in /home/kithkui/anky/.env")
        sys.exit(1)
    
    print(f"Testing with Anky X credentials:")
    print(f"  Consumer Key: {c_key[:8]}...")
    print(f"  Token: ...{access_t[-6:]}")
    
    headers = twitter_oauth_sign(c_key, c_secret, access_t, acc_secret)
    auth_len = len(headers['Authorization'])
    print(f"\nGenerated OAuth Authorization header ({auth_len} chars):")
    print(headers['Authorization'][:120], "...")
    
    # Test making actual request to Twitter API with this signature
    import requests
    test_text = "Anky credentials check - 2026-03-19" + datetime.now().isoformat()
    
    try:
        print(f"\nPosting {len(test_text)} char text...")
        resp = requests.post(
            'https://api.twitter.com/2/tweets',
            headers=headers,
            json={'text': test_text},
            timeout=30
        )
        print(f"\n=== Twitter API Response ===")
        print(f"Status: {resp.status_code}")
        if resp.text:
            content_type = resp.headers.get('Content-Type', '')
            print(f"Response Content-Type: {content_type[:40]}")
            try:
                data = resp.json()
                print(f"JSON Response fields: {list(data.keys())}")
                if resp.status_code == 201 and 'data' in data and 'id' in data['data']:
                    tweet_id = data['data']['id']
                    print(f"✓ SUCCESS! Tweet posted. ID: {tweet_id}")
                else:
                    print(f"Response body: {str(data)[:500]}")
            except Exception as e:
                print(f"Body (raw): {resp.text[:500]}")
        
    except Exception as e:
        import traceback
        print(f"\nError making request: {type(e).__name__}: {e}")
        traceback.print_exc()
