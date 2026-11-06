use tauri::WebviewWindow;

// Tauri 2.3 has no public cookie API. Read WebView2's URL-scoped store,
// including HttpOnly cookies, through an asynchronous callback on its UI thread.
#[cfg(windows)]
pub(crate) async fn read_cookie(window: &WebviewWindow, url: &str) -> anyhow::Result<String> {
    use std::time::Duration;

    use anyhow::{anyhow, Context};
    use webview2_com::{
        take_pwstr, GetCookiesCompletedHandler, Microsoft::Web::WebView2::Win32::ICoreWebView2_2,
    };
    use windows::core::{Interface, HSTRING, PCWSTR, PWSTR};

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let url = url.to_owned();
    window.with_webview(move |platform| unsafe {
        let result = (|| -> windows::core::Result<()> {
            let webview = platform
                .controller()
                .CoreWebView2()?
                .cast::<ICoreWebView2_2>()?;
            let callback_tx = tx.clone();
            let uri = HSTRING::from(url);
            webview.CookieManager()?.GetCookies(
                PCWSTR::from_raw(uri.as_ptr()),
                &GetCookiesCompletedHandler::create(Box::new(move |status, cookies| {
                    let result = (|| -> windows::core::Result<String> {
                        status?;
                        let mut pairs = Vec::new();
                        if let Some(cookies) = cookies {
                            let mut count = 0;
                            cookies.Count(&mut count)?;
                            for index in 0..count {
                                let cookie = cookies.GetValueAtIndex(index)?;
                                let mut name = PWSTR::null();
                                cookie.Name(&mut name)?;
                                let name = take_pwstr(name);
                                let mut value = PWSTR::null();
                                cookie.Value(&mut value)?;
                                let value = take_pwstr(value);
                                pairs.push(format!("{name}={value}"));
                            }
                        }
                        pairs.sort();
                        Ok(pairs.join("; "))
                    })();
                    let _ = callback_tx.send(result);
                    Ok(())
                })),
            )
        })();
        if let Err(err) = result {
            let _ = tx.send(Err(err));
        }
    })?;
    tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .context("读取网站登录状态超时，请重试")?
        .ok_or_else(|| anyhow!("登录窗口已关闭"))?
        .map_err(Into::into)
}

#[cfg(not(windows))]
pub(crate) async fn read_cookie(_window: &WebviewWindow, _url: &str) -> anyhow::Result<String> {
    anyhow::bail!("此版本的网页登录暂仅支持 Windows");
}

#[cfg(windows)]
pub(crate) async fn evaluate(
    window: &WebviewWindow,
    script: &str,
) -> anyhow::Result<serde_json::Value> {
    use anyhow::{anyhow, Context};
    use std::time::Duration;
    use webview2_com::ExecuteScriptCompletedHandler;
    use windows::core::HSTRING;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let script = script.to_owned();
    window.with_webview(move |platform| unsafe {
        let result = (|| -> windows::core::Result<()> {
            let webview = platform.controller().CoreWebView2()?;
            let callback_tx = tx.clone();
            webview.ExecuteScript(
                &HSTRING::from(script),
                &ExecuteScriptCompletedHandler::create(Box::new(move |status, result| {
                    let _ = callback_tx.send(status.map(|()| result));
                    Ok(())
                })),
            )
        })();
        if let Err(err) = result {
            let _ = tx.send(Err(err));
        }
    })?;
    let json = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .context("读取验证码状态超时")?
        .ok_or_else(|| anyhow!("验证码窗口已关闭"))??;
    serde_json::from_str(&json).context("验证码窗口返回了无效状态")
}

#[cfg(not(windows))]
pub(crate) async fn evaluate(
    _window: &WebviewWindow,
    _script: &str,
) -> anyhow::Result<serde_json::Value> {
    anyhow::bail!("此版本的验证码弹窗暂仅支持 Windows");
}
