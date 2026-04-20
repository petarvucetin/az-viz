#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from PIL import Image
import os

icon_dir = 'src-tauri/icons'
sizes = [256, 128, 64, 32, 16]

# Load all PNG images
images = []
for size in sizes:
    png_path = os.path.join(icon_dir, f'icon-{size}.png')
    print(f'Loading {png_path}...')
    img = Image.open(png_path).convert('RGBA')
    images.append(img)
    print(f'[OK] Loaded {size}x{size}')

# Create ICO file (PIL saves all sizes into a single ICO)
ico_path = os.path.join(icon_dir, 'icon.ico')
print(f'\nCreating {ico_path}...')
images[0].save(ico_path, format='ICO', sizes=[(img.width, img.height) for img in images])
print(f'[OK] Created {ico_path}')

# Verify file
file_size = os.path.getsize(ico_path)
print(f'\nFile size: {file_size / 1024:.1f} KB')
print('Done.')
