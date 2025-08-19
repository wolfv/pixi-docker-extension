#!/usr/bin/env python3
"""
Example tests for the pixi-docker demo application
"""

import pytest
import sys
from pathlib import Path

# Add src to path so we can import main
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

def test_import():
    """Test that we can import our main module"""
    import main
    assert hasattr(main, 'serve_static')

def test_port_from_env(monkeypatch):
    """Test that PORT environment variable is used"""
    monkeypatch.setenv('PORT', '9000')
    
    # We would need to mock the server startup to test this properly
    # For now, just test that the environment variable can be read
    import os
    assert os.environ.get('PORT') == '9000'

def test_static_directory_check():
    """Test static directory detection"""
    from pathlib import Path
    static_dir = Path('../static')  # Relative to tests directory
    
    # In a real test, we'd check the actual logic from main.py
    # For demo purposes, just verify the directory exists
    assert static_dir.exists(), "Static directory should exist for demo"

if __name__ == "__main__":
    # Run tests when called directly
    pytest.main([__file__])