// Render retained PTY bytes in one pinned terminal emulator; never generate UI copy.
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const zlib = require('zlib');
const { chromium } = require('playwright');
const bundled = require('@sparticuz/chromium');
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');

async function main() {
  const [input, output, mode = 'banner'] = process.argv.slice(2);
  if (!['banner', 'ui'].includes(mode)) throw new Error('Unsupported render mode');
  if (!input || !output) throw new Error('Usage: node render.cjs paired.json NEW_OUTPUT_DIR');
  if (fs.existsSync(output)) throw new Error('Refusing to overwrite evidence');
  const bundle = JSON.parse(fs.readFileSync(input, 'utf8'));
  fs.mkdirSync(output, { recursive: true });
  const fontRoot = path.dirname(require.resolve('@fontsource/dejavu-mono/package.json'));
  const faces = [[400, 'normal'], [700, 'normal'], [400, 'italic'], [700, 'italic']];
  const fontFiles = faces.map(([weight, style]) => ({weight, style,
    data: fs.readFileSync(path.join(fontRoot, `files/dejavu-mono-latin-${weight}-${style}.woff2`))}));
  const fontCSS = fontFiles.map(f => `@font-face{font-family:CaptureMono;font-weight:${f.weight};font-style:${f.style};src:url(data:font/woff2;base64,${f.data.toString('base64')})}`).join('\n');
  const browser = await chromium.launch({ executablePath: await bundled.executablePath(),
    args: ['--no-sandbox', '--disable-dev-shm-usage'], headless: true });
  const manifest = { status: 'CAPTURED_NOT_REVIEWED', browser: browser.version(),
    xterm: '5.5.0', font: 'DejaVu Mono @fontsource 5.2.5', fontSize: 16,
    deviceScaleFactor: 1, normalization: 'none', input_sha256: sha(fs.readFileSync(input)),
    fonts: fontFiles.map(f => ({weight:f.weight, style:f.style, sha256:sha(f.data)})), cases: [] };
  try {
    for (const entry of bundle.cases) {
      const validId = mode === 'banner' ? /^banner-(100|80|94|95)x30$/ : /^(wizard|picker|completion|summary)-[a-z-]+-(100|80)x30$/;
      if (!validId.test(entry.id)) throw new Error('Unsupported case id');
      const page = await browser.newPage({ viewport: { width: 2300, height: 1000 }, deviceScaleFactor: 1 });
      await page.route('**/*', route => route.abort()); // Replay needs no network.
      await page.setContent('<html><head></head><body><div id="pair"><section><h3>Python — upstream display component</h3><div id="python"></div></section><section><h3>Rust — real offline REPL banner</h3><div id="rust"></div></section></div></body></html>');
      if (mode === 'ui') await page.evaluate(() => {
        const headings = document.querySelectorAll('h3');
        headings[0].textContent = 'Python — upstream UI component';
        headings[1].textContent = 'Rust — actual CLI / summary component';
      });
      await page.addStyleTag({ content: fontCSS + '\nbody{margin:0;padding:20px;background:#16191d;color:#eee;font:14px sans-serif}#pair{display:flex;gap:24px;width:max-content}section{width:max-content}h3{margin:0 0 12px}.xterm{padding:0}' });
      await page.addStyleTag({ path: require.resolve('@xterm/xterm/css/xterm.css') });
      await page.addScriptTag({ path: require.resolve('@xterm/xterm/lib/xterm.js') });
      await page.evaluate(async () => { await document.fonts.load('16px CaptureMono'); await document.fonts.load('bold 16px CaptureMono'); await document.fonts.ready; });
      const screens = {};
      for (const side of ['python', 'rust']) {
        const c = entry[side];
        if (c.error) throw new Error(`Unreached UI case ${entry.id}/${side}: ${c.error}`);
        const raw = Buffer.from(c.raw_base64, 'base64');
        if (sha(raw) !== c.raw_sha256) throw new Error('Corrupt raw capture');
        const replay = Buffer.concat(c.events_base64.map(e => Buffer.from(e[1], 'base64')));
        if (!replay.equals(raw)) throw new Error('Recording does not reconstruct raw stream');
        if (c.width !== entry.rust.width || c.height !== 30 || c.snapshot_end_byte > raw.length) throw new Error('Invalid dimensions/endpoint');
        fs.writeFileSync(path.join(output, `${entry.id}-${side}.ansi`), raw);
        const decoder = new TextDecoder('utf-8', {fatal: true});
        const recording = [JSON.stringify({version:2,width:c.width,height:c.height,env:{TERM:c.environment.TERM},title:`${entry.id}-${side}`})];
        for (const [time, b64] of c.events_base64) {
          const text = decoder.decode(Buffer.from(b64, 'base64'), {stream:true});
          if (text) recording.push(JSON.stringify([time, 'o', text]));
        }
        const tail = decoder.decode();
        if (tail) recording.push(JSON.stringify([c.events_base64.at(-1)[0], 'o', tail]));
        fs.writeFileSync(path.join(output, `${entry.id}-${side}.cast`), recording.join('\n')+'\n');
        screens[side] = await page.evaluate(async ({side,c,data}) => {
          const t = new Terminal({cols:c.width,rows:c.height,fontFamily:'CaptureMono',fontSize:16,lineHeight:1,
            scrollback:200,theme:{background:'#000000',foreground:'#ffffff'},cursorBlink:false});
          window.terminals ||= {}; window.terminals[side] = t;
          t.open(document.getElementById(side));
          await new Promise(resolve => t.write(Uint8Array.from(atob(data), x=>x.charCodeAt(0)), resolve));
          const rows = [];
          for (let y=0;y<t.buffer.active.length;y++) {
            const line=t.buffer.active.getLine(y), cells=[];
            for(let x=0;x<c.width;x++) {
              const cell=line.getCell(x);
              cells.push({s:cell.getChars(),w:cell.getWidth(),fg:cell.getFgColor(),fm:cell.getFgColorMode(),
                bg:cell.getBgColor(),bm:cell.getBgColorMode(),bold:!!cell.isBold(),dim:!!cell.isDim(),italic:!!cell.isItalic(),underline:!!cell.isUnderline(),inverse:!!cell.isInverse()});
            }
            rows.push({text:line.translateToString(true),wrapped:line.isWrapped,cells});
          }
          return rows;
        }, {side,c,data:raw.subarray(0,c.snapshot_end_byte).toString('base64')});
      }
      const images=[];
      for (const position of ['top','bottom']) {
        await page.evaluate(position => {for(const t of Object.values(window.terminals)) position==='top'?t.scrollToTop():t.scrollToBottom();}, position);
        await page.evaluate(() => new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
        const file=`${entry.id}-${position}.png`;
        await page.locator('#pair').screenshot({path:path.join(output,file)});
        images.push({file,sha256:sha(fs.readFileSync(path.join(output,file))),viewport:position});
      }
      fs.writeFileSync(path.join(output,`${entry.id}-cells.json.gz`),zlib.gzipSync(JSON.stringify(screens)));
      manifest.cases.push({id:entry.id,width:entry.rust.width,height:30,images});
      await page.close();
    }
  } finally { await browser.close(); }
  fs.writeFileSync(path.join(output,'renderer.json'),JSON.stringify(manifest,null,2)+'\n');
}
main().catch(error => { console.error(error); process.exitCode=1; });
