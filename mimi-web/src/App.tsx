import { useState } from 'react'

const downloadUrl = 'https://github.com/yuxino/mimi/releases/latest'
const repositoryUrl = 'https://github.com/yuxino/mimi'
const apiKeyUrl = 'https://help.aliyun.com/zh/model-studio/get-api-key'
const openAiApiKeyUrl = 'https://platform.openai.com/api-keys'

function ArrowUpRight() {
  return (
    <svg aria-hidden="true" viewBox="0 0 20 20" fill="none">
      <path d="M5 15 15 5M7 5h8v8" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}

function WaveIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 32 20" fill="none">
      <path d="M1 10h3M8 6v8M15 2v16M22 5v10M29 8v4" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" />
    </svg>
  )
}

function CatMark({ dark = false }: { dark?: boolean }) {
  return (
    <span className={`cat-mark${dark ? ' cat-mark--dark' : ''}`} aria-hidden="true">
      <img src="/mimi/mimi-icon-512.jpg" alt="" width={512} height={512} decoding="async" />
    </span>
  )
}

const features = [
  {
    index: '01',
    title: '听系统声音',
    copy: '不占用麦克风。mimi 直接听电脑正在播放的系统声音，macOS 和 Windows 上的浏览器、播放器和桌面应用都能用。',
    accent: 'mint',
  },
  {
    index: '02',
    title: '字幕自然跟上',
    copy: '预翻译先稳稳出现，最终译文自然接替。已经说完的句子不乱跳，只让正在说的尾句继续更新。',
    accent: 'lilac',
  },
  {
    index: '03',
    title: '安静地待在一旁',
    copy: '字幕窗可以移动、缩放、锁定。锁定后，鼠标直接穿过，不挡住你正在看的内容。',
    accent: 'peach',
  },
]

const scenes = [
  { image: '/mimi/film-960.jpg', label: '海外影视', copy: '看懂没有字幕的夜晚' },
  { image: '/mimi/game-960.jpg', label: '剧情游戏', copy: '跟上每一句对白' },
  { image: '/mimi/live-960.jpg', label: '直播与播客', copy: '把远方的声音听清楚' },
  { image: '/mimi/romance-960.jpg', label: '外语短剧', copy: '不再错过关键情节' },
]

const setupSteps = [
  { index: '01', title: '下载并打开', copy: '从 GitHub Releases 获取适合 macOS 或 Windows 的最新版，按系统提示安装并打开。' },
  { index: '02', title: '填入凭证', copy: '选择阿里云百炼（默认）或 OpenAI Realtime，并填写对应 API Key；阿里共享 API 无需 Workspace ID。' },
  { index: '03', title: '开始听', copy: '播放视频后点击 Start Listening，把字幕窗放到舒服的位置，就可以继续看了。' },
]

function App() {
  const [menuOpen, setMenuOpen] = useState(false)

  const closeMenu = () => setMenuOpen(false)

  return (
    <div className="mimi-site">
      <header className="site-header">
        <a className="brand" href="#top" aria-label="mimi 首页">
          <CatMark />
          <span>mimi</span>
        </a>

        <button
          className="mobile-menu-button"
          type="button"
          aria-expanded={menuOpen}
          aria-controls="site-navigation"
          onClick={() => setMenuOpen((open) => !open)}
        >
          <span>{menuOpen ? '关闭' : '菜单'}</span>
          <i aria-hidden="true" />
        </button>

        <nav id="site-navigation" className={`site-navigation${menuOpen ? ' site-navigation--open' : ''}`} aria-label="主导航">
          <a href="#features" onClick={closeMenu}>能力</a>
          <a href="#scenes" onClick={closeMenu}>适合什么</a>
          <a href="#start" onClick={closeMenu}>开始使用</a>
          <a href="#privacy" onClick={closeMenu}>隐私</a>
          <a href={repositoryUrl} target="_blank" rel="noreferrer" onClick={closeMenu}>GitHub</a>
        </nav>

        <a className="header-download" href={downloadUrl} target="_blank" rel="noreferrer">
          下载 mimi <ArrowUpRight />
        </a>
      </header>

      <main id="top">
        <section className="hero-section">
          <div className="hero-glow hero-glow--mint" />
          <div className="hero-glow hero-glow--lilac" />
          <div className="hero-copy">
            <div className="eyebrow eyebrow--light"><span className="eyebrow-dot" /> macOS + Windows 实时翻译字幕</div>
            <h1>听懂<br /><em>正在发生的事。</em></h1>
            <p className="hero-lead">mimi 把电脑上播放的日语、英语或韩语，变成自然、清晰、不会挡路的实时字幕。</p>
            <div className="hero-actions">
              <a className="button button--mint" href={downloadUrl} target="_blank" rel="noreferrer">下载最新版 <ArrowUpRight /></a>
              <a className="hero-text-link" href="#features">看看它怎么工作 <span>↓</span></a>
            </div>
            <div className="hero-meta">
              <span><i /> macOS + Windows</span>
              <span><i /> English / 日本語 / 한국어</span>
            </div>
          </div>

          <div className="hero-media" aria-label="mimi 在日本车站场景上显示实时中文字幕" role="img">
            <div className="hero-media-frame">
              <img
                src="/mimi/product-hero-1440.jpg"
                alt="mimi 在日本车站场景上显示实时中文字幕"
                width={1440}
                height={810}
                decoding="async"
                fetchPriority="high"
              />
              <div className="hero-media-shine" />
            </div>
            <div className="listening-pill"><span className="listening-pulse" /><WaveIcon /><span>mimi 正在听</span></div>
            <div className="companion-card">
              <img src="/mimi/mimi-icon-512.jpg" alt="mimi 角色头像" width={512} height={512} decoding="async" />
              <div><span>mimi · みみ</span><strong>我在听哦</strong></div>
              <WaveIcon />
            </div>
            <div className="hero-caption"><span>字幕只是陪伴</span><strong>不抢你的注意力</strong></div>
          </div>
        </section>

        <section className="trust-strip" aria-label="mimi 产品特点">
          <div><span>01</span><strong>系统声音</strong><small>不使用麦克风</small></div>
          <div><span>02</span><strong>无需账号</strong><small>打开就能开始</small></div>
          <div><span>03</span><strong>安全保存</strong><small>系统安全凭据存储</small></div>
          <div><span>04</span><strong>双平台支持</strong><small>macOS + Windows</small></div>
        </section>

        <section className="statement-section" aria-labelledby="statement-title">
          <div className="section-label">01 / mimi 是什么</div>
          <div className="statement-grid">
            <h2 id="statement-title">一层轻轻的字幕，<br /><em>让世界多懂一点。</em></h2>
            <div className="statement-copy">
              <p>它不会接管你的屏幕，也不要求你改变观看习惯。打开视频、游戏或直播，mimi 会安静地把听不懂的对白放在你看得到的地方。</p>
              <p className="statement-note"><span>耳</span> みみ · mimi</p>
            </div>
          </div>
        </section>

        <section className="features-section" id="features" aria-labelledby="features-title">
          <div className="section-heading">
            <div>
              <div className="section-label">02 / 它会做什么</div>
              <h2 id="features-title">像字幕一样自然，<br /><em>像猫一样安静。</em></h2>
            </div>
            <p>把复杂的技术藏起来，只留下阅读时真正需要的东西。</p>
          </div>

          <div className="feature-grid">
            {features.map((feature) => (
              <article className={`feature-card feature-card--${feature.accent}`} key={feature.index}>
                <div className="feature-card-top"><span>{feature.index}</span><WaveIcon /></div>
                <div>
                  <h3>{feature.title}</h3>
                  <p>{feature.copy}</p>
                </div>
                <div className="feature-card-line" />
              </article>
            ))}
          </div>
        </section>

        <section className="overlay-section" aria-labelledby="overlay-title">
          <div className="overlay-visual">
            <div className="overlay-window">
              <img src="/mimi/overlay-current.png" alt="mimi 实时字幕窗界面" width={1280} height={272} loading="lazy" decoding="async" />
            </div>
            <div className="overlay-orbit overlay-orbit--one" />
            <div className="overlay-orbit overlay-orbit--two" />
          </div>
          <div className="overlay-copy">
            <div className="section-label section-label--light">03 / 字幕窗</div>
            <h2 id="overlay-title">让对白<br /><em>自己排好队。</em></h2>
            <p>当前一句保持清楚，刚刚说过的内容带着时间慢慢淡去。持续讲话时，mimi 会把长段落拆成刚好读得完的小句。</p>
            <ul className="check-list">
              <li><span>✦</span> 拖动、缩放，放到你喜欢的位置</li>
              <li><span>✦</span> 锁定后鼠标可以直接穿过字幕窗</li>
              <li><span>✦</span> 字号 14–20，窄窗口也能自然换行</li>
            </ul>
          </div>
        </section>

        <section className="scenes-section" id="scenes" aria-labelledby="scenes-title">
          <div className="section-heading section-heading--scenes">
            <div>
              <div className="section-label">04 / 你可以在哪里用</div>
              <h2 id="scenes-title">不管你在看什么，<br /><em>都别让语言挡住它。</em></h2>
            </div>
            <a className="inline-link" href={downloadUrl} target="_blank" rel="noreferrer">开始使用 <ArrowUpRight /></a>
          </div>
          <div className="scene-grid">
            {scenes.map((scene) => (
              <article className="scene-card" key={scene.label}>
                <img src={scene.image} alt={`${scene.label} 使用场景`} width={960} height={540} loading="lazy" decoding="async" />
                <div className="scene-shade" />
                <div className="scene-content"><span>{scene.label}</span><strong>{scene.copy}</strong></div>
              </article>
            ))}
          </div>
        </section>

        <section className="start-section" id="start" aria-labelledby="start-title">
          <div className="start-heading">
            <div className="section-label">05 / 三步开始</div>
            <h2 id="start-title">把语言门槛，<br /><em>留在三分钟以前。</em></h2>
            <p>mimi 是开源项目。选择阿里云百炼（默认）或 OpenAI Realtime，保存对应 API Key；阿里共享 API 无需 Workspace ID。</p>
          </div>
          <div className="setup-grid">
            {setupSteps.map((step) => (
              <article className="setup-step" key={step.index}>
                <span>{step.index}</span>
                <div><h3>{step.title}</h3><p>{step.copy}</p></div>
              </article>
            ))}
          </div>
          <aside className="start-card">
            <div className="start-card-character">
              <img src="/mimi/mimi-icon-512.jpg" alt="mimi 角色" width={512} height={512} loading="lazy" decoding="async" />
              <span className="start-card-bubble">准备好就叫我 ♫</span>
            </div>
            <div className="start-card-copy">
              <span className="start-card-kicker">READY WHEN YOU ARE</span>
              <h3>下载，填好凭证，<br />然后继续看你的。</h3>
              <div className="start-card-links">
                <a className="button button--ink" href={downloadUrl} target="_blank" rel="noreferrer">下载最新版 <ArrowUpRight /></a>
                <a href={apiKeyUrl} target="_blank" rel="noreferrer">阿里云 API Key</a>
                <a href={openAiApiKeyUrl} target="_blank" rel="noreferrer">OpenAI API Key</a>
              </div>
            </div>
          </aside>
        </section>

        <section className="privacy-section" id="privacy" aria-labelledby="privacy-title">
          <div className="privacy-copy">
            <div className="section-label">06 / 不多打扰</div>
            <h2 id="privacy-title">你的声音，<br /><em>只经过，不留下。</em></h2>
            <p>mimi 不使用麦克风，不需要注册账号，也不会保存音频和字幕记录。API Key 保存在系统安全凭据存储中。</p>
            <a className="inline-link" href={repositoryUrl} target="_blank" rel="noreferrer">在 GitHub 查看更多 <ArrowUpRight /></a>
          </div>
          <div className="settings-card">
            <div className="settings-card-top"><span>mimi Settings</span><span>macOS + Windows</span></div>
            <img src="/mimi/settings-560.jpg" alt="mimi 设置界面，API Key 保存在系统安全凭据存储中" width={560} height={492} loading="lazy" decoding="async" />
            <span className="settings-caption">secure OS credential storage</span>
          </div>
        </section>

        <section className="download-section" aria-labelledby="download-title">
          <div className="download-character">
            <img className="download-character-main" src="/mimi/mimi-icon-512.jpg" alt="mimi 角色形象" width={512} height={512} loading="lazy" decoding="async" />
            <img className="download-cat-sticker" src="/mimi/mimi-cat-256.png" alt="mimi 黑猫伙伴" width={256} height={256} loading="lazy" decoding="async" />
          </div>
          <div className="download-copy">
            <div className="section-label section-label--light">07 / 现在就听</div>
            <h2 id="download-title">给耳朵<br /><em>多一个选择。</em></h2>
            <p>开源、轻巧，安静地待在你的电脑上。</p>
            <div className="download-actions">
              <a className="button button--mint" href={downloadUrl} target="_blank" rel="noreferrer">下载 mimi <ArrowUpRight /></a>
              <a className="button button--ghost" href={repositoryUrl} target="_blank" rel="noreferrer">查看源码</a>
            </div>
          </div>
        </section>
      </main>

      <footer className="site-footer">
        <a className="brand brand--footer" href="#top" aria-label="mimi 首页">
          <CatMark dark />
          <span>mimi</span>
        </a>
        <p>Live translated subtitles on macOS and Windows.</p>
        <div className="footer-links">
          <a href={repositoryUrl} target="_blank" rel="noreferrer">GitHub</a>
          <a href={`${repositoryUrl}/issues`} target="_blank" rel="noreferrer">反馈问题</a>
          <span>MIT · © 2026 yuxino</span>
        </div>
      </footer>
    </div>
  )
}

export default App
