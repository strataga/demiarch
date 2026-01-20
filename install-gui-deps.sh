#!/bin/bash

# Install GUI system dependencies for Tauri development
# Updated with correct package names for this system

echo "Installing GUI system dependencies..."

# Update package lists
echo "Updating package lists..."
sudo apt-get update

# Install Tauri GUI dependencies
echo "Installing Tauri GUI dependencies..."
sudo apt-get install -y \
    libwebkit2gtk-4.1-dev \
    libjavascriptcoregtk-4.1-dev \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libglib2.0-dev \
    libgdk-pixbuf2.0-dev \
    libpango1.0-dev \
    libatk1.0-dev \
    libcairo2-dev \
    libsoup2.4-dev \
    pkg-config \
    build-essential

echo "GUI system dependencies installation complete!"
echo "You can now test the GUI compilation with: cargo check -p demiarch-gui"