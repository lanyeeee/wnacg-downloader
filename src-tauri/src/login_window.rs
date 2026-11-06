use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::{bail, Context};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tokio::time::sleep;

use crate::{
    browser_session::{evaluate, read_cookie},
    config::ProxyMode,
    extensions::AppHandleExt,
    wnacg_client::LoginOutcome,
};

const WINDOW_LABEL: &str = "login_captcha";

#[derive(Default)]
pub struct LoginState(AtomicU64);

pub fn begin(app: &AppHandle) -> u64 {
    app.state::<LoginState>().0.fetch_add(1, Ordering::SeqCst) + 1
}

pub fn is_current(app: &AppHandle, attempt: u64) -> bool {
    app.state::<LoginState>().0.load(Ordering::SeqCst) == attempt
}

pub async fn open(
    app: &AppHandle,
    username: &str,
    password: &str,
    attempt: u64,
) -> anyhow::Result<Option<String>> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window.set_focus()?;
        bail!("验证码窗口已打开，请先完成验证");
    }

    let config = app.get_config().read().clone();
    let api_domain = config.get_api_domain();
    let origin: tauri::Url = format!("https://{api_domain}/").parse()?;
    if origin.host_str().is_none() || origin.path() != "/" || !origin.username().is_empty() {
        bail!("网站域名配置无效，请在配置中填写域名");
    }
    let login_url = origin.join("users-login.html")?;
    let mut builder =
        WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::External(login_url.clone()))
            .title("请完成滑块验证")
            .inner_size(400.0, 360.0)
            .resizable(false)
            .visible(false)
            .initialization_script(include_str!("captcha_window.js"))
            .center()
            .on_navigation({
                let origin = origin.clone();
                move |url| url.origin() == origin.origin()
            });

    if config.proxy_mode == ProxyMode::Custom {
        builder = builder
            .proxy_url(format!("http://{}:{}", config.proxy_host, config.proxy_port).parse()?);
    } else if config.proxy_mode == ProxyMode::NoProxy {
        builder = builder.additional_browser_args("--no-proxy-server");
    }

    if let Some(main) = app.get_webview_window("main") {
        builder = builder.parent(&main)?;
    }
    let window = builder.build().context("无法打开验证码窗口")?;
    let closed = Arc::new(AtomicBool::new(false));
    window.on_window_event({
        let closed = closed.clone();
        move |event| {
            if matches!(
                event,
                WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed
            ) {
                closed.store(true, Ordering::SeqCst);
            }
        }
    });

    let result = wait_for_captcha(app, &window, &closed, &login_url, attempt).await;
    let result = match result {
        Ok(Some((pass, browser_cookie))) => {
            // Credentials stay in Rust; only the challenge is rendered in the webview.
            if closed.load(Ordering::SeqCst) || !is_current(app, attempt) {
                Ok(None)
            } else if app.get_config().read().get_api_domain() != api_domain {
                Err(anyhow::anyhow!("网站域名已改变，请重新登录"))
            } else {
                let _ = window.set_title("验证成功，正在登录…");
                let outcome = app
                    .get_wnacg_client()
                    .login_with_captcha(&api_domain, username, password, &pass, &browser_cookie)
                    .await;
                if closed.load(Ordering::SeqCst) || !is_current(app, attempt) {
                    Ok(None)
                } else {
                    match outcome {
                        Ok(LoginOutcome::Authenticated(cookie)) => {
                            save_cookie(app, &api_domain, &cookie).map(|()| Some(cookie))
                        }
                        Ok(LoginOutcome::CaptchaRequired) => {
                            Err(anyhow::anyhow!("验证码已失效，请重新登录"))
                        }
                        Err(err) => Err(err),
                    }
                }
            }
        }
        Ok(None) => Ok(None),
        Err(err) => Err(err),
    };
    let _ = window.close();
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_focus();
    }
    result
}

async fn wait_for_captcha(
    app: &AppHandle,
    window: &WebviewWindow,
    closed: &AtomicBool,
    login_url: &tauri::Url,
    attempt: u64,
) -> anyhow::Result<Option<(String, String)>> {
    let started = Instant::now();
    let expected_url = serde_json::to_string(login_url.as_str())?;
    let script = format!("location.href === {expected_url} ? window.__wnacgCaptcha : null");
    let mut shown = false;
    loop {
        if closed.load(Ordering::SeqCst) || !is_current(app, attempt) {
            return Ok(None);
        }
        let current_domain = app.get_config().read().get_api_domain();
        if format!("https://{current_domain}/users-login.html") != login_url.as_str() {
            bail!("网站域名已改变，请重新登录");
        }
        let state = evaluate(window, &script).await;
        if closed.load(Ordering::SeqCst) || !is_current(app, attempt) {
            return Ok(None);
        }
        let state = state?;
        match state["status"].as_str() {
            Some("ready" | "passed") => {
                if !shown {
                    window.show()?;
                    window.set_focus()?;
                    shown = true;
                }
                if state["status"] == "passed" {
                    let pass = state["pass"]
                        .as_str()
                        .filter(|pass| !pass.is_empty() && pass.len() <= 4096)
                        .context("验证码返回结果不完整，请重试")?;
                    let cookie = read_cookie(window, login_url.as_str()).await?;
                    return Ok(Some((pass.to_owned(), cookie)));
                }
            }
            Some("cancelled") => return Ok(None),
            Some("error") => bail!("验证码加载失败，请检查网络后重试"),
            _ if !shown && started.elapsed() > Duration::from_secs(30) => {
                bail!("验证码加载超时，请检查网络后重试");
            }
            _ => {}
        }
        sleep(Duration::from_millis(200)).await;
    }
}

pub fn save_cookie(app: &AppHandle, api_domain: &str, cookie: &str) -> anyhow::Result<()> {
    let config_state = app.get_config();
    let mut config = config_state.write();
    if config.get_api_domain() != api_domain {
        bail!("网站域名已改变，请重新登录");
    }
    let previous = std::mem::replace(&mut config.cookie, cookie.to_owned());
    if let Err(err) = config.save(app) {
        config.cookie = previous;
        return Err(err.context("保存登录状态失败"));
    }
    Ok(())
}

pub fn close(app: &AppHandle) -> anyhow::Result<()> {
    begin(app);
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window.close()?;
    }
    Ok(())
}
