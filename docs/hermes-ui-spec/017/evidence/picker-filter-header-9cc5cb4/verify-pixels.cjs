// Read-only comparison of the first terminal row in retained screenshot panes.
// Pinned renderer: 30 terminal rows, 19 pixels per row; no pixels are rewritten.
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const {chromium} = require('playwright');
const bundled = require('@sparticuz/chromium');
(async () => {
  const dir = process.argv[2];
  const out = process.argv[3];
  const ids = process.argv.slice(4);
  const browser = await chromium.launch({executablePath: await bundled.executablePath(), args: ['--no-sandbox','--disable-dev-shm-usage'], headless: true});
  const cases = [];
  try {
    const page = await browser.newPage();
    for (const id of ids) {
      const raw = fs.readFileSync(path.join(dir, `${id}-bottom.png`));
      const result = await page.evaluate(async data => {
        const image = new Image();
        image.src = `data:image/png;base64,${data}`;
        await image.decode();
        const canvas = document.createElement('canvas');
        canvas.width = image.naturalWidth; canvas.height = image.naturalHeight;
        const context = canvas.getContext('2d'); context.drawImage(image, 0, 0);
        const gutter = 24;
        const pane = (canvas.width - gutter) / 2;
        if (!Number.isInteger(pane)) throw new Error('Asymmetric panes');
        const height = 19, y = canvas.height - 30 * height;
        if (y !== 31) throw new Error('Unexpected pinned renderer title/terminal geometry');
        const left = context.getImageData(0, y, pane, height).data;
        const right = context.getImageData(pane + gutter, y, pane, height).data;
        let differing = 0;
        for (let i = 0; i < left.length; i += 4) if (left[i]!==right[i]||left[i+1]!==right[i+1]||left[i+2]!==right[i+2]||left[i+3]!==right[i+3]) differing++;
        return {image_width: canvas.width, image_height: canvas.height, pane_width: pane,
          comparison_y: y, comparison_height: height, differing_pixels: differing, equal: differing === 0};
      }, raw.toString('base64'));
      cases.push({id, png_sha256: crypto.createHash('sha256').update(raw).digest('hex'), ...result});
      console.log(id, JSON.stringify(result));
    }
  } finally { await browser.close(); }
  const result = {normalization: 'none', comparison: 'unaltered first terminal row, excluding renderer titles and 24px gutter', cases};
  fs.writeFileSync(out, JSON.stringify(result, null, 2) + '\n');
})().catch(error => {console.error(error); process.exitCode = 1;});
