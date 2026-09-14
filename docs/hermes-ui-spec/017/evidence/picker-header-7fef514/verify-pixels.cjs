// Read-only comparison of the first terminal row in retained screenshot panes.
// Pinned renderer: 30 terminal rows, 19 pixels per row; no pixels are rewritten.
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const {chromium} = require('playwright');
const bundled = require('@sparticuz/chromium');
(async () => {
  const browser = await chromium.launch({executablePath: await bundled.executablePath(), args: ['--no-sandbox', '--disable-dev-shm-usage'], headless: true});
  const cases = [];
  try {
    const page = await browser.newPage();
    for (const name of ['normal', 'delete']) for (const width of [100, 80]) {
      const id = `picker-${name}-${width}x30`;
      const raw = fs.readFileSync(path.join(__dirname, `${id}-bottom.png`));
      const result = await page.evaluate(async data => {
        const image = new Image();
        image.src = `data:image/png;base64,${data}`;
        await image.decode();
        const canvas = document.createElement('canvas');
        canvas.width = image.naturalWidth; canvas.height = image.naturalHeight;
        const context = canvas.getContext('2d'); context.drawImage(image, 0, 0);
        const gutter = 24; // Unchanged renderer #pair gap, not terminal columns.
        const pane = (canvas.width - gutter) / 2;
        if (!Number.isInteger(pane)) throw new Error('Asymmetric panes');
        const height = 19, y = canvas.height - 30 * height;
        if (y !== 31) throw new Error('Unexpected pinned renderer title/terminal geometry');
        const left = context.getImageData(0, y, pane, height).data;
        const right = context.getImageData(pane + gutter, y, pane, height).data;
        return {image_width: canvas.width, image_height: canvas.height, pane_width: pane,
          comparison_y: y, comparison_height: height, equal: left.every((v, i) => v === right[i])};
      }, raw.toString('base64'));
      if (!result.equal) throw new Error(`Header pixel difference: ${id}`);
      cases.push({id, png_sha256: crypto.createHash('sha256').update(raw).digest('hex'), ...result});
    }
  } finally { await browser.close(); }
  const result = {normalization: 'none', comparison: 'unaltered first terminal row, excluding renderer titles and 24px gutter', cases};
  const output = path.join(__dirname, 'header-pixels.json');
  const text = JSON.stringify(result, null, 2) + '\n';
  if (fs.existsSync(output)) {
    if (fs.readFileSync(output, 'utf8') !== text) throw new Error('Existing pixel audit disagrees');
  } else fs.writeFileSync(output, text);
  console.log(text);
})().catch(error => {console.error(error); process.exitCode = 1;});
