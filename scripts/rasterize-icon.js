#!/usr/bin/env node

const sharp = require('sharp');
const fs = require('fs');
const path = require('path');

const svgPath = path.join(__dirname, '../src-tauri/icons/az-plotter-icon.svg');
const outputDir = path.join(__dirname, '../src-tauri/icons');

const sizes = [16, 32, 64, 128, 256];

(async () => {
  const svgBuffer = fs.readFileSync(svgPath);

  for (const size of sizes) {
    const outputPath = path.join(outputDir, `icon-${size}.png`);
    console.log(`Rasterizing to ${size}x${size}...`);

    await sharp(svgBuffer, { density: 300 })
      .png({ adaptiveFiltering: true })
      .resize(size, size, { fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } })
      .toFile(outputPath);

    console.log(`✓ Created ${outputPath}`);
  }

  console.log('\nAll sizes rasterized successfully.');
})().catch(err => {
  console.error('Error:', err);
  process.exit(1);
});
