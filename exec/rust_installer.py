#!/usr/bin/env python3
"""
🦀 RUST INSTALLER FOR GOLEM BOUNTY
==================================
Automatikus Rust fejlesztői környezet setup
"""

import subprocess
import os
import sys
from pathlib import Path

def install_rust():
    """Rust telepítés Windows-ra"""
    
    print("🦀 INSTALLING RUST FOR GOLEM BOUNTY...")
    print("=" * 45)
    
    # Check if Rust already installed
    try:
        result = subprocess.run(['cargo', '--version'], capture_output=True, text=True)
        if result.returncode == 0:
            print(f"✅ Rust already installed: {result.stdout.strip()}")
            return True
    except FileNotFoundError:
        pass
    
    print("📦 Downloading Rust installer...")
    
    # Download rustup-init.exe
    import urllib.request
    
    try:
        rustup_url = "https://win.rustup.rs/x86_64"
        rustup_path = "rustup-init.exe"
        
        print(f"⬇️ Downloading from: {rustup_url}")
        urllib.request.urlretrieve(rustup_url, rustup_path)
        print(f"✅ Downloaded: {rustup_path}")
        
        # Run installer
        print("🚀 Running Rust installer...")
        result = subprocess.run([rustup_path, "-y"], check=True)
        
        print("✅ Rust installation completed!")
        
        # Clean up
        os.remove(rustup_path)
        
        # Update PATH for current session
        cargo_path = Path.home() / ".cargo" / "bin"
        if cargo_path.exists():
            os.environ['PATH'] = str(cargo_path) + os.pathsep + os.environ['PATH']
            print(f"📝 Added to PATH: {cargo_path}")
        
        return True
        
    except Exception as e:
        print(f"❌ Rust installation failed: {e}")
        return False

def install_cargo_component():
    """cargo-component telepítése"""
    
    print("\n🔧 INSTALLING CARGO-COMPONENT...")
    print("=" * 35)
    
    try:
        # Install cargo-component
        result = subprocess.run([
            'cargo', 'install', 'cargo-component'
        ], check=True, capture_output=True, text=True)
        
        print("✅ cargo-component installed successfully!")
        return True
        
    except subprocess.CalledProcessError as e:
        print(f"❌ cargo-component installation failed: {e}")
        print(f"stdout: {e.stdout}")
        print(f"stderr: {e.stderr}")
        return False
    except FileNotFoundError:
        print("❌ cargo command not found. Rust installation may have failed.")
        return False

def install_wasm_tools():
    """WebAssembly tools telepítése"""
    
    print("\n🛠️ INSTALLING WASM TOOLS...")
    print("=" * 30)
    
    tools = [
        'wasm-tools',
        'wit-bindgen-cli'
    ]
    
    for tool in tools:
        try:
            print(f"📦 Installing {tool}...")
            result = subprocess.run([
                'cargo', 'install', tool
            ], check=True, capture_output=True, text=True)
            print(f"✅ {tool} installed!")
            
        except subprocess.CalledProcessError as e:
            print(f"⚠️ {tool} installation failed, but continuing...")

def verify_installation():
    """Telepítés ellenőrzése"""
    
    print("\n🔍 VERIFYING INSTALLATION...")
    print("=" * 30)
    
    checks = [
        ('cargo', ['cargo', '--version']),
        ('rustc', ['rustc', '--version']),
        ('cargo-component', ['cargo', 'component', '--version']),
    ]
    
    all_good = True
    
    for name, cmd in checks:
        try:
            result = subprocess.run(cmd, capture_output=True, text=True, check=True)
            print(f"✅ {name}: {result.stdout.strip()}")
        except (subprocess.CalledProcessError, FileNotFoundError):
            print(f"❌ {name}: Not available")
            all_good = False
    
    return all_good

def create_project_update_script():
    """Script a tracker frissítéséhez"""
    
    script_content = '''
import sys
sys.path.append('..')
from bounty_tracker import BountyTracker

tracker = BountyTracker()
tracker.quick_update('Rust toolchain installed', 'completed')
tracker.quick_update('cargo-component installed', 'completed')
tracker.log_milestone('Development environment ready', 'Rust + WebAssembly tools installed')
print('✅ TRACKER UPDATED: Development environment ready!')
'''
    
    with open('update_tracker.py', 'w') as f:
        f.write(script_content)

if __name__ == "__main__":
    print("🎯 GOLEM BOUNTY - RUST SETUP")
    print("=" * 30)
    
    success = True
    
    # Install Rust
    if not install_rust():
        success = False
    
    # Install cargo-component
    if success and not install_cargo_component():
        success = False
    
    # Install additional tools
    if success:
        install_wasm_tools()
    
    # Verify everything
    if success:
        success = verify_installation()
    
    if success:
        print("\n🎉 RUST SETUP COMPLETE!")
        print("✅ Ready for Golem bounty implementation!")
        
        create_project_update_script()
        
        print("\n🚀 NEXT STEPS:")
        print("1. cargo component build")
        print("2. Start JavaScript executor implementation")
        print("3. Continue with Python executor")
        
    else:
        print("\n❌ SETUP FAILED!")
        print("Please install Rust manually from: https://rustup.rs/")
    
    input("\nPress Enter to continue...")
