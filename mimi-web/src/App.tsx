import { useState } from 'react'

const downloadUrl = 'https://github.com/yuxino/mimi/releases/latest'
const repositoryUrl = 'https://github.com/yuxino/mimi'

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
      <img src="/mimi/mimi-icon.png" alt="" />
    </span>
  )
}

const features = [
  {
    index: '01',
    title: '听系统声音',
    copy: '不占用麦克风。mimi 直接听 Mac 正在播放的声音，浏览器、播放器和桌面应用都能用。',
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
  { image: '/mimi/film-real.jpg', label: '海外影视', copy: '看懂没有字幕的夜晚' },
  { image: '/mimi/game-real.jpg', label: '剧情游戏', copy: '跟上每一句对白' },
  { image: '/mimi/live-real.jpg', label: '直播与播客', copy: '把远方的声音听清楚' },
  { image: '/mimi/romance-real.jpg', label: '成熟向内容', copy: '不再错过关键情节' },
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
            <div className="eyebrow eyebrow--light"><span className="eyebrow-dot" /> macOS 原生实时翻译字幕</div>
            <h1>听懂<br /><em>正在发生的事。</em></h1>
            <p className="hero-lead">mimi 把 Mac 上播放的日语、英语或韩语，变成自然、清晰、不会挡路的实时字幕。</p>
            <div className="hero-actions">
              <a className="button button--mint" href={downloadUrl} target="_blank" rel="noreferrer">下载最新版 <ArrowUpRight /></a>
              <a className="hero-text-link" href="#features">看看它怎么工作 <span>↓</span></a>
            </div>
            <div className="hero-meta">
              <span><i /> macOS 14+</span>
              <span><i /> English / 日本語 / 한국어</span>
            </div>
          </div>

          <div className="hero-media" aria-label="mimi 在日本车站场景上显示实时中文字幕" role="img">
            <div className="hero-media-frame">
              <img src="/mimi/product-hero.png" alt="mimi 在日本车站场景上显示实时中文字幕" />
              <div className="hero-media-shine" />
            </div>
            <div className="listening-pill"><span className="listening-pulse" /><WaveIcon /><span>mimi 正在听</span></div>
            <div className="cat-float"><img src="/mimi/mimi-cat.png" alt="mimi 黑猫形象" /></div>
            <div className="hero-caption"><span>字幕只是陪伴</span><strong>不抢你的注意力</strong></div>
          </div>
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
              <img src="/mimi/overlay-current.png" alt="mimi 实时字幕窗界面" />
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
                <img src={scene.image} alt={`${scene.label} 使用场景`} />
                <div className="scene-shade" />
                <div className="scene-content"><span>{scene.label}</span><strong>{scene.copy}</strong></div>
              </article>
            ))}
          </div>
        </section>

        <section className="privacy-section" id="privacy" aria-labelledby="privacy-title">
          <div className="privacy-copy">
            <div className="section-label">05 / 不多打扰</div>
            <h2 id="privacy-title">你的声音，<br /><em>只经过，不留下。</em></h2>
            <p>mimi 不使用麦克风，不需要注册账号，也不会保存音频和字幕记录。API Key 保存在 Mac 的钥匙串中。</p>
            <a className="inline-link" href={repositoryUrl} target="_blank" rel="noreferrer">在 GitHub 查看更多 <ArrowUpRight /></a>
          </div>
          <div className="settings-card">
            <div className="settings-card-top"><span>mimi Settings</span><span className="settings-lights"><i /><i /><i /></span></div>
            <img src="/mimi/settings.png" alt="mimi 设置界面，API Key 保存在 Keychain" />
            <span className="settings-caption">credentials stay on this Mac</span>
          </div>
        </section>

        <section className="download-section" aria-labelledby="download-title">
          <div className="download-cat"><img src="/mimi/mimi-cat.png" alt="mimi 黑猫形象" /></div>
          <div className="download-copy">
            <div className="section-label section-label--light">06 / 现在就听</div>
            <h2 id="download-title">给耳朵<br /><em>多一个选择。</em></h2>
            <p>开源、原生、轻轻地待在你的 Mac 上。</p>
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
        <p>Live translated subtitles on Mac.</p>
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
