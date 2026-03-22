#!/usr/bin/env python3
"""
Create a valid Anky agent for autonomous posting.
Generates a properly formatted API key and creates agent record.
"""
import hashlib
import secrets
import subprocess
from pathlib import Path
import datetime

ANKY_DB = Path.home() / "anky" / "data" / "anky.db"
ENV_PATH = Path.home() / "anky" / ".env"

def generate_agent_key():
    """Generate a valid anky_ API key (37 chars total: anky_ + 32 hex)."""
    return f"anky_{secrets.token_hex(16)}"

def create_agent_record(agent_id, name, api_key):
    """Create agent and associated API key in database."""
    # Hash the API key for storage (Rust app stores hash)
    hashed = hashlib.sha256(api_key.encode()).hexdigest()
    
    print(f"[{datetime.datetime.now()}] Creating agent: '{agent_id}'...")
    
    # Step 1: Insert into api_keys table
    print("\n[1/2] Inserting API key record...")
    sqlite_cmd = f"""
        INSERT OR REPLACE INTO api_keys (key, is_active) 
        VALUES ('{hashed}', 1);
        """
    result = subprocess.run(["sqlite3", str(ANKY_DB), sqlite_cmd], capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"Failed to insert API key: {result.stderr}")
    
    # Step 2: Create agent record
    print("[2/2] Creating agent record...")
    sqlite_cmd = f"""
        INSERT OR REPLACE INTO agents (id, name, model, api_key, free_sessions_remaining)
        VALUES ('{agent_id}', '{name}', 'auto', '{api_key}', 5);
        """
    result = subprocess.run(["sqlite3", str(ANKY_DB), sqlite_cmd], capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"Failed to create agent: {result.stderr}")

def update_env(api_key):
    """Update .env file with new agent key."""
    env_content = ENV_PATH.read_text()
    # Remove old lines
    lines = [l for l in env_content.splitlines() 
             if not l.startswith("#") or "ANKY_AGENT_API_KEY" in l]
    new_entry = f"\n# ANKY AGENT API KEY (for session-based chunked submission)\nANKY_AGENT_API_KEY={api_key}\n"
    ENV_PATH.write_text("\n".join(lines + [new_entry]))

if __name__ == "__main__":
    if not ANKY_DB.exists():
        print(f"✗ Anky DB not found at {ANKY_DB}")
        exit(1)
    
    agent_id = "autonomous"
    name = "Anky Autonomous Content System"
    api_key = generate_agent_key()
    
    try:
        create_agent_record(agent_id, name, api_key)
        update_env(api_key)
        
        print(f"\n✓ Agent created successfully!")
        print(f"• ID: {agent_id}")
        print(f"• Name: {name}")
        print(f"• API Key: {api_key}")
        print(f"• Free sessions remaining: 5")
    except Exception as e:
        print(f"✗ Failed: {e}")
        exit(1)
