const fs = require('fs');
const path = require('path');
const matter = require('gray-matter');
const { marked } = require('marked');

// ---------------------------------------------------------------------------
// Navigation structure — defines sidebar order and section labels
// ---------------------------------------------------------------------------
const NAV = [
  {
    section: 'Introduction',
    pages: [
      { file: 'introduction/overview.mdx', label: 'Overview' },
      { file: 'introduction/philosophy.mdx', label: 'Philosophy' },
      { file: 'introduction/quickstart.mdx', label: 'Quickstart' },
    ],
  },
  {
    section: 'The Ankyverse',
    pages: [
      { file: 'concepts/ankyverse.mdx', label: 'Ankyverse' },
      { file: 'concepts/chakras.mdx', label: 'Chakras' },
      { file: 'concepts/sojourns.mdx', label: 'Sojourns' },
      { file: 'concepts/kingdoms.mdx', label: 'Kingdoms' },
    ],
  },
  {
    section: 'Core Mechanics',
    pages: [
      { file: 'concepts/writing-practice.mdx', label: 'Writing Practice' },
      { file: 'concepts/everything-is-an-excuse.mdx', label: 'Everything is an Excuse' },
      { file: 'concepts/stories.mdx', label: 'Stories' },
      { file: 'concepts/ankycoin.mdx', label: 'Ankycoin' },
    ],
  },
  {
    section: 'Architecture',
    pages: [
      { file: 'architecture/overview.mdx', label: 'Overview' },
      { file: 'architecture/pipelines.mdx', label: 'Pipelines' },
      { file: 'architecture/seed-identity.mdx', label: 'Seed Identity' },
    ],
  },
  {
    section: 'Agents',
    pages: [
      { file: 'agents/overview.mdx', label: 'Overview' },
      { file: 'agents/dumb.mdx', label: 'Dumb Agent' },
      { file: 'agents/smart.mdx', label: 'Smart Agent' },
      { file: 'agents/agi.mdx', label: 'AGI Agent' },
    ],
  },
  {
    section: 'API Reference',
    pages: [
      { file: 'api-reference/introduction.mdx', label: 'Introduction' },
      { file: 'api-reference/authentication.mdx', label: 'Authentication' },
    ],
  },
  {
    section: 'Writing',
    pages: [
      { file: 'api-reference/endpoints/write-v1.mdx', label: 'POST /swift/v1/write' },
      { file: 'api-reference/endpoints/write-v2.mdx', label: 'POST /swift/v2/write' },
      { file: 'api-reference/endpoints/writings.mdx', label: 'GET writings' },
    ],
  },
  {
    section: 'Children & Stories',
    pages: [
      { file: 'api-reference/endpoints/children.mdx', label: 'Children' },
      { file: 'api-reference/endpoints/cuentacuentos.mdx', label: 'Cuentacuentos' },
    ],
  },
  {
    section: 'Facilitators',
    pages: [
      { file: 'api-reference/endpoints/facilitators.mdx', label: 'Facilitators' },
    ],
  },
  {
    section: 'Auth',
    pages: [
      { file: 'api-reference/endpoints/auth.mdx', label: 'Auth Endpoints' },
    ],
  },
  {
    section: 'Self-Hosting',
    pages: [
      { file: 'self-hosting/requirements.mdx', label: 'Requirements' },
      { file: 'self-hosting/configuration.mdx', label: 'Configuration' },
      { file: 'self-hosting/deployment.mdx', label: 'Deployment' },
    ],
  },
];

// ---------------------------------------------------------------------------
// Build a flat ordered list of pages for prev/next links
// ---------------------------------------------------------------------------
const ALL_PAGES = NAV.flatMap((s) => s.pages);

function mdxToHtmlPath(file) {
  return file.replace(/\.mdx$/, '.html');
}

// Depth from an html path (e.g. "api-reference/endpoints/auth.html" => 2)
function depth(htmlPath) {
  return htmlPath.split('/').length - 1;
}

function relativeRoot(htmlPath) {
  const d = depth(htmlPath);
  return d === 0 ? '.' : Array(d).fill('..').join('/');
}

// ---------------------------------------------------------------------------
// Marked configuration
// ---------------------------------------------------------------------------
marked.setOptions({
  gfm: true,
  breaks: false,
});

// Custom renderer for tables (add wrapper div) and code blocks (add copy button)
const renderer = new marked.Renderer();

renderer.table = function (header, body) {
  // marked v12 passes a token object; handle both shapes
  if (typeof header === 'object' && header.header !== undefined) {
    const rows = header.rows || [];
    const hdrs = header.header || [];
    const aligns = header.align || [];

    const thCells = hdrs.map((cell, i) => {
      const align = aligns[i] ? ` style="text-align:${aligns[i]}"` : '';
      return `<th${align}>${cell.tokens ? this.parser.parseInline(cell.tokens) : cell.text || ''}</th>`;
    }).join('');

    const bodyRows = rows.map((row) => {
      const cells = row.map((cell, i) => {
        const align = aligns[i] ? ` style="text-align:${aligns[i]}"` : '';
        return `<td${align}>${cell.tokens ? this.parser.parseInline(cell.tokens) : cell.text || ''}</td>`;
      }).join('');
      return `<tr>${cells}</tr>`;
    }).join('\n');

    return `<div class="table-wrap"><table><thead><tr>${thCells}</tr></thead><tbody>${bodyRows}</tbody></table></div>`;
  }
  // Fallback for older marked signature
  return `<div class="table-wrap"><table><thead>${header}</thead><tbody>${body}</tbody></table></div>`;
};

renderer.code = function (code, lang) {
  // marked v12 may pass a token object {text, lang, escaped}
  if (typeof code === 'object') {
    lang = code.lang || '';
    code = code.text || '';
  }
  const langClass = lang ? ` class="language-${lang}"` : '';
  const langLabel = lang ? `<span class="code-lang">${lang}</span>` : '';
  return `<div class="code-block">${langLabel}<button class="copy-btn" title="Copy code" onclick="copyCode(this)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg></button><pre><code${langClass}>${escapeHtml(code)}</code></pre></div>`;
};

function escapeHtml(str) {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

marked.use({ renderer });

// ---------------------------------------------------------------------------
// HTML template
// ---------------------------------------------------------------------------
function buildSidebar(currentHtmlPath, root) {
  let html = '';
  for (const section of NAV) {
    html += `<div class="nav-section">`;
    html += `<div class="nav-section-title">${section.section}</div>`;
    html += `<ul class="nav-list">`;
    for (const page of section.pages) {
      const href = `${root}/${mdxToHtmlPath(page.file)}`;
      const active = mdxToHtmlPath(page.file) === currentHtmlPath ? ' class="active"' : '';
      html += `<li><a href="${href}"${active}>${page.label}</a></li>`;
    }
    html += `</ul></div>`;
  }
  return html;
}

function buildPrevNext(currentFile, root) {
  const idx = ALL_PAGES.findIndex((p) => p.file === currentFile);
  const prev = idx > 0 ? ALL_PAGES[idx - 1] : null;
  const next = idx < ALL_PAGES.length - 1 ? ALL_PAGES[idx + 1] : null;
  let html = '<div class="prev-next">';
  if (prev) {
    html += `<a class="prev-next-link prev" href="${root}/${mdxToHtmlPath(prev.file)}"><span class="prev-next-dir">Previous</span><span class="prev-next-title">${prev.label}</span></a>`;
  } else {
    html += '<span></span>';
  }
  if (next) {
    html += `<a class="prev-next-link next" href="${root}/${mdxToHtmlPath(next.file)}"><span class="prev-next-dir">Next</span><span class="prev-next-title">${next.label}</span></a>`;
  } else {
    html += '<span></span>';
  }
  html += '</div>';
  return html;
}

function template({ title, description, bodyHtml, sidebar, prevNext, root }) {
  return `<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>${title} - Anky Docs</title>
<meta name="description" content="${description || ''}"/>
<link rel="icon" href="${root}/logo/favicon.svg" type="image/svg+xml"/>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github-dark.min.css" id="hljs-dark"/>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github.min.css" id="hljs-light" disabled/>
<style>${CSS}</style>
</head>
<body>
<header class="top-bar">
  <div class="top-bar-left">
    <button class="hamburger" id="hamburger" aria-label="Toggle menu" onclick="toggleSidebar()">
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="18" x2="21" y2="18"/></svg>
    </button>
    <a href="${root}/introduction/overview.html" class="logo-link">
      <img src="${root}/logo/anky-dark.svg" alt="Anky" class="logo logo-dark" width="100" height="28"/>
      <img src="${root}/logo/anky-light.svg" alt="Anky" class="logo logo-light" width="100" height="28"/>
      <span class="logo-suffix">docs</span>
    </a>
  </div>
  <div class="top-bar-right">
    <a href="https://anky.app" class="top-link">anky.app</a>
    <button class="theme-toggle" id="themeToggle" aria-label="Toggle theme" onclick="toggleTheme()">
      <svg class="icon-sun" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"/><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/></svg>
      <svg class="icon-moon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z"/></svg>
    </button>
  </div>
</header>
<div class="layout">
  <aside class="sidebar" id="sidebar">${sidebar}</aside>
  <div class="sidebar-overlay" id="sidebarOverlay" onclick="toggleSidebar()"></div>
  <main class="content">
    <article class="prose">${bodyHtml}</article>
    ${prevNext}
  </main>
</div>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
<script>
hljs.highlightAll();

function toggleSidebar(){
  document.getElementById('sidebar').classList.toggle('open');
  document.getElementById('sidebarOverlay').classList.toggle('open');
}

function toggleTheme(){
  const html=document.documentElement;
  const next=html.getAttribute('data-theme')==='dark'?'light':'dark';
  html.setAttribute('data-theme',next);
  localStorage.setItem('anky-docs-theme',next);
  applyThemeSheets(next);
}
function applyThemeSheets(t){
  document.getElementById('hljs-dark').disabled=(t==='light');
  document.getElementById('hljs-light').disabled=(t==='dark');
}
(function(){
  const saved=localStorage.getItem('anky-docs-theme');
  if(saved){document.documentElement.setAttribute('data-theme',saved);applyThemeSheets(saved);}
})();

function copyCode(btn){
  const code=btn.parentElement.querySelector('code');
  navigator.clipboard.writeText(code.textContent).then(()=>{
    btn.classList.add('copied');
    btn.title='Copied!';
    setTimeout(()=>{btn.classList.remove('copied');btn.title='Copy code';},1500);
  });
}

// Close sidebar on nav link click (mobile)
document.querySelectorAll('.sidebar a').forEach(a=>{
  a.addEventListener('click',()=>{
    document.getElementById('sidebar').classList.remove('open');
    document.getElementById('sidebarOverlay').classList.remove('open');
  });
});
</script>
</body>
</html>`;
}

// ---------------------------------------------------------------------------
// CSS
// ---------------------------------------------------------------------------
const CSS = `
:root{
  --orange:#FF6B35;
  --orange-dim:#cc5529;
  --dark:#0D0D0D;
  --dark-surface:#161616;
  --dark-surface2:#1e1e1e;
  --dark-border:#2a2a2a;
  --dark-text:#e0e0e0;
  --dark-muted:#888;
  --light-bg:#fafafa;
  --light-surface:#fff;
  --light-border:#e0e0e0;
  --light-text:#1a1a1a;
  --light-muted:#666;
  --sidebar-width:260px;
  --topbar-height:52px;
  --content-max:780px;
}

*,*::before,*::after{box-sizing:border-box;margin:0;padding:0;}

/* Dark theme (default) */
[data-theme="dark"]{
  --bg:var(--dark);
  --surface:var(--dark-surface);
  --surface2:var(--dark-surface2);
  --border:var(--dark-border);
  --text:var(--dark-text);
  --muted:var(--dark-muted);
  --code-bg:#1a1a2e;
  color-scheme:dark;
}
[data-theme="dark"] .logo-light{display:none!important;}
[data-theme="dark"] .icon-sun{display:none;}
[data-theme="dark"] .icon-moon{display:block;}

[data-theme="light"]{
  --bg:var(--light-bg);
  --surface:var(--light-surface);
  --surface2:#f0f0f0;
  --border:var(--light-border);
  --text:var(--light-text);
  --muted:var(--light-muted);
  --code-bg:#f5f5f5;
  color-scheme:light;
}
[data-theme="light"] .logo-dark{display:none!important;}
[data-theme="light"] .icon-moon{display:none;}
[data-theme="light"] .icon-sun{display:block;}

html{font-size:16px;}
body{
  font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Oxygen,Ubuntu,Cantarell,sans-serif;
  background:var(--bg);
  color:var(--text);
  line-height:1.65;
  -webkit-font-smoothing:antialiased;
}

/* Top bar */
.top-bar{
  position:fixed;top:0;left:0;right:0;height:var(--topbar-height);
  background:var(--surface);border-bottom:1px solid var(--border);
  display:flex;align-items:center;justify-content:space-between;
  padding:0 16px;z-index:100;
}
.top-bar-left{display:flex;align-items:center;gap:12px;}
.top-bar-right{display:flex;align-items:center;gap:12px;}
.logo-link{display:flex;align-items:center;gap:6px;text-decoration:none;}
.logo{height:24px;width:auto;}
.logo-suffix{font-size:13px;font-weight:600;color:var(--muted);letter-spacing:0.5px;text-transform:uppercase;}
.top-link{font-size:13px;color:var(--muted);text-decoration:none;transition:color .15s;}
.top-link:hover{color:var(--orange);}
.theme-toggle,.hamburger{
  background:none;border:none;color:var(--muted);cursor:pointer;
  padding:4px;display:flex;align-items:center;justify-content:center;
  border-radius:6px;transition:color .15s,background .15s;
}
.theme-toggle:hover,.hamburger:hover{color:var(--text);background:var(--surface2);}
.hamburger{display:none;}

/* Layout */
.layout{
  display:flex;
  padding-top:var(--topbar-height);
  min-height:100vh;
}

/* Sidebar */
.sidebar{
  position:fixed;top:var(--topbar-height);left:0;bottom:0;
  width:var(--sidebar-width);
  overflow-y:auto;
  padding:20px 0 40px;
  background:var(--surface);
  border-right:1px solid var(--border);
  z-index:90;
}
.sidebar::-webkit-scrollbar{width:4px;}
.sidebar::-webkit-scrollbar-thumb{background:var(--border);border-radius:4px;}
.nav-section{margin-bottom:8px;}
.nav-section-title{
  font-size:11px;font-weight:700;text-transform:uppercase;letter-spacing:0.8px;
  color:var(--muted);padding:10px 20px 4px;
}
.nav-list{list-style:none;}
.nav-list li a{
  display:block;padding:5px 20px 5px 28px;
  font-size:14px;color:var(--text);text-decoration:none;
  border-left:2px solid transparent;
  transition:all .12s;
}
.nav-list li a:hover{color:var(--orange);background:var(--surface2);}
.nav-list li a.active{
  color:var(--orange);border-left-color:var(--orange);
  font-weight:600;background:var(--surface2);
}

/* Content */
.content{
  flex:1;
  margin-left:var(--sidebar-width);
  padding:40px 48px 80px;
  max-width:calc(var(--content-max) + var(--sidebar-width) + 96px);
}

/* Prose */
.prose{max-width:var(--content-max);}
.prose h1{font-size:2rem;font-weight:800;margin-bottom:8px;letter-spacing:-0.02em;color:var(--text);}
.prose h2{font-size:1.4rem;font-weight:700;margin-top:2.2em;margin-bottom:0.6em;padding-bottom:0.3em;border-bottom:1px solid var(--border);color:var(--text);}
.prose h3{font-size:1.15rem;font-weight:600;margin-top:1.8em;margin-bottom:0.5em;color:var(--text);}
.prose h4{font-size:1rem;font-weight:600;margin-top:1.4em;margin-bottom:0.4em;color:var(--text);}
.prose p{margin-bottom:1em;}
.prose a{color:var(--orange);text-decoration:underline;text-underline-offset:2px;}
.prose a:hover{color:var(--orange-dim);}
.prose strong{font-weight:600;color:var(--text);}
.prose ul,.prose ol{margin-bottom:1em;padding-left:1.5em;}
.prose li{margin-bottom:0.35em;}
.prose li>p{margin-bottom:0.35em;}
.prose blockquote{
  border-left:3px solid var(--orange);margin:1.2em 0;padding:0.6em 1em;
  background:var(--surface2);border-radius:0 6px 6px 0;
  color:var(--muted);font-style:italic;
}
.prose hr{border:none;border-top:1px solid var(--border);margin:2em 0;}
.prose img{max-width:100%;border-radius:8px;margin:1em 0;}

/* Inline code */
.prose code{
  font-family:"SFMono-Regular",Consolas,"Liberation Mono",Menlo,monospace;
  font-size:0.875em;background:var(--surface2);padding:0.15em 0.4em;border-radius:4px;
}
.prose pre code{background:none;padding:0;font-size:0.85rem;}

/* Code blocks */
.code-block{
  position:relative;margin:1.2em 0;border-radius:8px;overflow:hidden;
  background:var(--code-bg);border:1px solid var(--border);
}
.code-block pre{
  margin:0;padding:16px 20px;overflow-x:auto;
  background:transparent!important;
}
.code-lang{
  position:absolute;top:6px;left:14px;font-size:10px;
  text-transform:uppercase;letter-spacing:0.5px;color:var(--muted);
  font-weight:600;
}
.copy-btn{
  position:absolute;top:8px;right:8px;background:var(--surface2);
  border:1px solid var(--border);border-radius:6px;padding:4px 6px;
  cursor:pointer;color:var(--muted);display:flex;align-items:center;
  transition:all .15s;opacity:0;
}
.code-block:hover .copy-btn{opacity:1;}
.copy-btn:hover{color:var(--text);background:var(--border);}
.copy-btn.copied{color:var(--orange);}

/* Tables */
.table-wrap{overflow-x:auto;margin:1.2em 0;}
.table-wrap table{
  width:100%;border-collapse:collapse;font-size:0.9rem;
}
.table-wrap th,.table-wrap td{
  padding:8px 12px;text-align:left;border-bottom:1px solid var(--border);
}
.table-wrap th{
  font-weight:600;font-size:0.8rem;text-transform:uppercase;letter-spacing:0.3px;
  color:var(--muted);background:var(--surface2);
}
.table-wrap tr:last-child td{border-bottom:none;}

/* Prev/Next */
.prev-next{
  display:flex;justify-content:space-between;gap:16px;
  margin-top:48px;padding-top:24px;border-top:1px solid var(--border);
}
.prev-next-link{
  display:flex;flex-direction:column;gap:2px;
  text-decoration:none;padding:12px 16px;border-radius:8px;
  border:1px solid var(--border);transition:all .15s;flex:1;max-width:48%;
}
.prev-next-link:hover{border-color:var(--orange);background:var(--surface);}
.prev-next-link.next{text-align:right;margin-left:auto;}
.prev-next-dir{font-size:12px;color:var(--muted);text-transform:uppercase;letter-spacing:0.5px;font-weight:600;}
.prev-next-title{font-size:15px;color:var(--orange);font-weight:500;}

/* Sidebar overlay for mobile */
.sidebar-overlay{
  display:none;position:fixed;inset:0;background:rgba(0,0,0,0.5);z-index:85;
}
.sidebar-overlay.open{display:block;}

/* Mobile */
@media(max-width:768px){
  .hamburger{display:flex;}
  .sidebar{
    transform:translateX(-100%);transition:transform .2s ease;
    z-index:95;top:var(--topbar-height);
  }
  .sidebar.open{transform:translateX(0);}
  .content{margin-left:0;padding:24px 20px 60px;}
  .prev-next{flex-direction:column;}
  .prev-next-link{max-width:100%;}
  .prev-next-link.next{text-align:left;}
}
`;

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------
const DOCS_DIR = __dirname;
const DIST_DIR = path.join(DOCS_DIR, 'dist');

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function build() {
  console.log('Building Anky docs...');

  // Clean
  if (fs.existsSync(DIST_DIR)) {
    fs.rmSync(DIST_DIR, { recursive: true });
  }
  ensureDir(DIST_DIR);

  // Copy logo files
  const logoSrc = path.join(DOCS_DIR, 'logo');
  const logoDst = path.join(DIST_DIR, 'logo');
  ensureDir(logoDst);
  for (const f of fs.readdirSync(logoSrc)) {
    fs.copyFileSync(path.join(logoSrc, f), path.join(logoDst, f));
  }
  console.log('  Copied logos');

  // Process each page
  let built = 0;
  for (const page of ALL_PAGES) {
    const srcPath = path.join(DOCS_DIR, page.file);
    if (!fs.existsSync(srcPath)) {
      console.warn(`  WARN: missing ${page.file}, skipping`);
      continue;
    }

    const raw = fs.readFileSync(srcPath, 'utf-8');
    const { data: fm, content } = matter(raw);

    // Strip JSX-style components (e.g. <Card>, <CardGroup>, etc.) — just pass through as markdown
    const cleaned = content
      .replace(/<\/?Card[^>]*>/g, '')
      .replace(/<\/?CardGroup[^>]*>/g, '')
      .replace(/<\/?Note[^>]*>/g, '')
      .replace(/<\/?Warning[^>]*>/g, '')
      .replace(/<\/?Info[^>]*>/g, '')
      .replace(/<\/?Tip[^>]*>/g, '')
      .replace(/<\/?Tabs[^>]*>/g, '')
      .replace(/<\/?Tab[^>]*>/g, '')
      .replace(/<\/?Steps[^>]*>/g, '')
      .replace(/<\/?Step[^>]*>/g, '')
      .replace(/<\/?Accordion[^>]*>/g, '')
      .replace(/<\/?AccordionGroup[^>]*>/g, '')
      .replace(/<\/?ResponseField[^>]*>/g, '')
      .replace(/<\/?ParamField[^>]*>/g, '')
      .replace(/<\/?Expandable[^>]*>/g, '');

    const bodyHtml = marked(cleaned);
    const htmlPath = mdxToHtmlPath(page.file);
    const root = relativeRoot(htmlPath);
    const sidebar = buildSidebar(htmlPath, root);
    const prevNext = buildPrevNext(page.file, root);

    const html = template({
      title: fm.title || page.label,
      description: fm.description || '',
      bodyHtml,
      sidebar,
      prevNext,
      root,
    });

    const outPath = path.join(DIST_DIR, htmlPath);
    ensureDir(path.dirname(outPath));
    fs.writeFileSync(outPath, html);
    built++;
  }

  // Index redirect
  const indexHtml = `<!DOCTYPE html>
<html><head>
<meta http-equiv="refresh" content="0;url=introduction/overview.html"/>
<link rel="canonical" href="introduction/overview.html"/>
</head><body><p>Redirecting to <a href="introduction/overview.html">documentation</a>...</p></body></html>`;
  fs.writeFileSync(path.join(DIST_DIR, 'index.html'), indexHtml);

  console.log(`  Built ${built} pages + index.html`);
  console.log('Done. Output in docs/dist/');
}

build();
