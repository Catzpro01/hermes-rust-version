// Read-only comparison of pinned-renderer panes (left = Python reference,
// right = Rust capture) over explicit terminal coordinates. No pixels are
// rewritten and no colours are normalized; the caller decides the regions.
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const {chromium} = require('playwright');
const bundled = require('@sparticuz/chromium');
(async () => {
  const dir = process.argv[2];
  const specPath = process.argv[3];
  const out = process.argv[4];
  const spec = JSON.parse(fs.readFileSync(specPath, 'utf8'));
  const browser = await chromium.launch({executablePath: await bundled.executablePath(), args: ['--no-sandbox','--disable-dev-shm-usage'], headless: true});
  const results = [];
  try {
    const page = await browser.newPage();
    for (const entry of spec.cases) {
      const raw = fs.readFileSync(path.join(dir, `${entry.id}-bottom.png`));
      const regions = [];
      for (const region of entry.regions) {
        const result = await page.evaluate(async ({data, region}) => {
          const image = new Image();
          image.src = `data:image/png;base64,${data}`;
          await image.decode();
          const canvas = document.createElement('canvas');
          canvas.width = image.naturalWidth; canvas.height = image.naturalHeight;
          const context = canvas.getContext('2d'); context.drawImage(image, 0, 0);
          const gutter = 24;
          const pane = (canvas.width - gutter) / 2;
          if (!Number.isInteger(pane)) throw new Error('Asymmetric panes');
          const rowHeight = 19;
          const top = canvas.height - 30 * rowHeight;
          if (top !== 31) throw new Error('Unexpected pinned renderer title/terminal geometry');
          const cell = pane / region.columns;
          const x0 = Math.round(region.colStart * cell);
          const x1 = Math.round(region.colEnd * cell);
          const y0 = top + (region.row - 1) * rowHeight;
          const width = x1 - x0, height = rowHeight;
          const left = context.getImageData(x0, y0, width, height).data;
          const right = context.getImageData(pane + gutter + x0, y0, width, height).data;
          let differing = 0;
          for (let i = 0; i < left.length; i += 4) if (left[i]!==right[i]||left[i+1]!==right[i+1]||left[i+2]!==right[i+2]||left[i+3]!==right[i+3]) differing++;
          return {x: x0, y: y0, width, height, differing_pixels: differing, equal: differing === 0};
        }, {data: raw.toString('base64'), region: {...region, columns: entry.columns}});
        regions.push({...region, ...result});
      }
      const pngSha = crypto.createHash('sha256').update(raw).digest('hex');
      results.push({id: entry.id, columns: entry.columns, png_sha256: pngSha,
                    differing_total: regions.reduce((n, r) => n + r.differing_pixels, 0), regions});
      console.log(entry.id, JSON.stringify(results[results.length - 1]));
    }
  } finally { await browser.close(); }
  fs.writeFileSync(out, JSON.stringify({spec, results}, null, 2) + '\n');
})().catch(error => { console.error(error); process.exit(1); });
