#!/usr/bin/env python3
import os
import sys
import json
from pathlib import Path

# Set up environment variables programmatically
os.environ['ANKY_AGENT_API_KEY'] = 'anky_4a4c317f5d892b6401436a0f8f7584b0'
sys.path.insert(0, str(Path(__file__).parent.parent / 'src'))

# Now run the main script logic
from anky.autonomous import main as autonomous_main

if __name__ == '__main__':
    autonomous_main()
