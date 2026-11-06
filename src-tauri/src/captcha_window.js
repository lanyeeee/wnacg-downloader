;(() => {
  if (window !== window.top) return
  window.__wnacgCaptcha = { status: 'loading' }
  const start = () => {
    document.title = '请完成滑块验证'
    document.body.replaceChildren()
    document.body.style.cssText = 'margin:0;background:#f5f6f8;font:14px sans-serif'
    const status = document.createElement('p')
    status.textContent = '正在加载验证码…'
    status.style.cssText = 'text-align:center;margin-top:24px;color:#666'
    document.body.appendChild(status)

    let attempts = 0
    const timer = setInterval(() => {
      if (window.SliderCaptcha && typeof window.SliderCaptcha.open === 'function') {
        clearInterval(timer)
        status.textContent = '请完成滑块验证'
        window.__wnacgCaptcha = { status: 'ready' }
        window.SliderCaptcha.open(
          (pass) => {
            window.__wnacgCaptcha = { status: 'passed', pass }
            status.textContent = '验证成功，正在登录…'
          },
          () => {
            window.__wnacgCaptcha = { status: 'cancelled' }
          },
        )
      } else if (++attempts >= 100) {
        clearInterval(timer)
        window.__wnacgCaptcha = { status: 'error' }
        status.textContent = '验证码加载失败，请关闭窗口后重试'
      }
    }, 100)
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', start, { once: true })
  } else {
    start()
  }
})()
