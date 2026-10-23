//! One-shot Chrome Native Messaging host. The browser launches this process;
//! there is no HTTP server, listening port, or persistent process.
use crate::cli::Browser;
use crate::state::{BrowserVisit, QueuedUrl, Store};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

pub const HOST_NAME: &str = "xyz.rindle.history";
pub const EXTENSION_ID: &str = "jlpbjckajdeinkbdbegabmfieclibihf";
const MAX_INCOMING: usize = 1024 * 1024;
const MAX_OUTGOING: usize = 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    version: u32,
    browser: Browser,
    #[serde(default)]
    visits: Vec<BrowserVisit>,
    #[serde(default)]
    ack: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Response {
    ok: bool,
    received: usize,
    acknowledged: usize,
    import: Vec<QueuedUrl>,
}

pub fn exchange(
    store: &mut Store,
    expected: Option<Browser>,
    request: Request,
) -> Result<Response> {
    if request.version != 1 || expected.is_some_and(|browser| browser != request.browser) {
        bail!("Unsupported Native Messaging protocol or browser mismatch")
    }
    let browser = request.browser;
    if !store.has_browser(browser)? {
        bail!("Run 'rindle setup --browser {}' first", browser.as_str())
    }
    let received = store.ingest(browser, &request.visits)?;
    let acknowledged = store.acknowledge(browser, &request.ack)?;
    let to_import = store.next_for_browser(browser, 100)?;
    store.touch(browser)?;
    Ok(Response {
        ok: true,
        received,
        acknowledged,
        import: to_import,
    })
}

fn read_message(reader: &mut impl Read) -> Result<Vec<u8>> {
    let mut header = [0_u8; 4];
    reader
        .read_exact(&mut header)
        .context("Missing Native Messaging length")?;
    let size = u32::from_ne_bytes(header) as usize;
    if size == 0 || size > MAX_INCOMING {
        bail!("Invalid Native Messaging length {size}")
    }
    let mut payload = vec![0_u8; size];
    reader
        .read_exact(&mut payload)
        .context("Truncated Native Messaging request")?;
    Ok(payload)
}

fn write_message(writer: &mut impl Write, data: &[u8]) -> Result<()> {
    if data.len() > MAX_OUTGOING {
        bail!("Native Messaging reply exceeded protocol limit")
    }
    writer.write_all(&(data.len() as u32).to_ne_bytes())?;
    writer.write_all(data)?;
    writer.flush()?;
    Ok(())
}

pub fn serve(browser: Option<Browser>, browser_arguments: &[String]) -> Result<()> {
    // Chrome supplies the sender extension origin as the host's first argument.
    // Do not trust a caller merely because it knows our host's filesystem path.
    let expected = format!("chrome-extension://{EXTENSION_ID}/");
    if !browser_arguments
        .iter()
        .any(|argument| argument == &expected)
    {
        bail!("Native Messaging sender extension is not authorized")
    }
    let bytes = read_message(&mut io::stdin().lock())?;
    let request: Request =
        serde_json::from_slice(&bytes).context("Invalid Native Messaging JSON")?;
    let mut store = Store::open()?;
    let response = exchange(&mut store, browser, request)?;
    write_message(&mut io::stdout().lock(), &serde_json::to_vec(&response)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn frames_are_little_endian_native_and_bounded() -> Result<()> {
        let mut buf = Vec::new();
        write_message(&mut buf, br#"{"version":1}"#)?;
        assert_eq!(&buf[..4], &(13_u32).to_ne_bytes());
        assert_eq!(read_message(&mut Cursor::new(buf))?, br#"{"version":1}"#);
        assert!(read_message(&mut Cursor::new(0_u32.to_ne_bytes())).is_err());
        assert!(read_message(&mut Cursor::new((MAX_INCOMING as u32 + 1).to_ne_bytes())).is_err());
        Ok(())
    }

    #[test]
    fn rejects_cross_browser_requests_and_returns_queue() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let mut store = Store::open_at(&dir.path().join("state.sqlite"))?;
        store.enable(Browser::Dia)?;
        store.enqueue_remote(&["https://example.org/a".to_owned()])?;
        let message = r#"{"version":1,"browser":"dia","visits":[{"url":"https://example.com/","title":"Test","lastVisitTime":1792770000000}]}"#;
        let request: Request = serde_json::from_str(message)?;
        let response = exchange(&mut store, Some(Browser::Dia), request)?;
        assert_eq!(response.received, 1);
        assert_eq!(response.import.len(), 1);
        let wrong: Request = serde_json::from_str(message)?;
        assert!(exchange(&mut store, Some(Browser::Edge), wrong).is_err());
        let shared: Request = serde_json::from_str(message)?;
        assert!(exchange(&mut store, None, shared).is_ok());
        Ok(())
    }
}
