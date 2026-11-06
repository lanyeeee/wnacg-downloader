// Run on Windows with: cargo run --release --example browser_session_smoke
// Uses an isolated WebView2 profile and synthetic cookies from a loopback server.
#[path = "../src/browser_session.rs"]
mod browser_session;

use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        atomic::{AtomicI32, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{ensure, Context};
use tauri::{webview::PageLoadEvent, WebviewUrl, WebviewWindowBuilder};

fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
            let mut request = [0; 4096];
            let _ = stream.read(&mut request);
            let body = "<!doctype html><title>Session smoke test</title>\
                <script>window.SliderCaptcha={open:pass=>pass('synthetic-proof')};</script>\
                <form><input name='login_name'><input name='login_pass'></form>";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\
                 Set-Cookie: test_session=synthetic-session; Path=/; HttpOnly; SameSite=Lax\r\n\
                 Set-Cookie: other_path=excluded; Path=/private; HttpOnly\r\n\
                 Connection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    let profile = std::env::current_dir()?
        .join("target")
        .join(format!("browser-session-smoke-{}", uuid::Uuid::new_v4()));
    let exit_code = Arc::new(AtomicI32::new(1));
    let exit_result = exit_code.clone();
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();

    tauri::Builder::default()
        .setup(move |app| {
            let loaded = Arc::new(tokio::sync::Notify::new());
            let signal = loaded.clone();
            let window = WebviewWindowBuilder::new(
                app,
                "session-smoke",
                WebviewUrl::External(format!("http://127.0.0.1:{port}/users.html").parse()?),
            )
            .visible(false)
            .data_directory(profile)
            .initialization_script(include_str!("../src/captcha_window.js"))
            .on_page_load(move |_, payload| {
                if matches!(payload.event(), PageLoadEvent::Finished)
                    && payload.url().scheme() == "http"
                {
                    signal.notify_one();
                }
            })
            .build()?;
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let result: anyhow::Result<()> = async {
                    tokio::time::timeout(Duration::from_secs(20), loaded.notified())
                        .await.context("fixture page failed to load")?;
                    let proof = tokio::time::timeout(Duration::from_secs(10), async {
                        loop {
                            let state = browser_session::evaluate(&window, "window.__wnacgCaptcha").await?;
                            if state["status"] == "passed" {
                                break Ok::<_, anyhow::Error>(state);
                            }
                            if state["status"] == "error" {
                                anyhow::bail!("fixture captcha failed to initialize");
                            }
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }).await.context("fixture captcha state timed out")??;
                    ensure!(proof["status"] == "passed" && proof["pass"] == "synthetic-proof", "captcha proof did not return to native code");
                    let form_removed = browser_session::evaluate(&window, "document.querySelector('[name=login_pass]') === null").await?;
                    ensure!(form_removed == true, "captcha popup still shows a login form");
                    let cookie = browser_session::read_cookie(
                        &window, &format!("http://127.0.0.1:{port}/users.html")
                    ).await?;
                    ensure!(cookie == "test_session=synthetic-session", "HttpOnly/path filtering failed");
                    let unrelated = browser_session::read_cookie(
                        &window, &format!("http://localhost:{port}/users.html")
                    ).await?;
                    ensure!(unrelated.is_empty(), "cookies leaked across hosts");
                    // Repeat to catch blocked callbacks or prematurely released COM handlers.
                    let again = browser_session::read_cookie(
                        &window, &format!("http://127.0.0.1:{port}/users.html")
                    ).await?;
                    ensure!(again == cookie, "repeated cookie read failed");
                    Ok(())
                }.await;
                match result {
                    Ok(()) => {
                        println!("PASS: captcha proof round-trip, captcha-only popup, HttpOnly session, URL path scope, host isolation, repeated async reads");
                        exit_result.store(0, Ordering::SeqCst);
                    }
                    Err(err) => {
                        eprintln!("FAIL: {err:#}");
                        std::process::exit(1);
                    }
                }
                handle.exit(exit_result.load(Ordering::SeqCst));
            });
            Ok(())
        })
        .run(context)?;
    std::process::exit(exit_code.load(Ordering::SeqCst));
}
