---
title: Other browsers
description: Connect Google Chrome or Microsoft Edge alongside Dia.
order: 6
category: Everyday use
summary: Bring Chrome or Edge into the same Safari/iCloud history bridge.
---

Rindle is designed around **Safari on your iPhone** and **Dia on your Mac**, but it also supports Google Chrome and Microsoft Edge on the Mac.

The setup is similar: tell Rindle which browser to connect, then load the same unpacked extension in that browser.

## Add Chrome

```sh
~/.local/bin/rindle setup --browser chrome
```

Open `chrome://extensions`, turn on Developer mode, and choose **Load unpacked**. Select `~/Library/Application Support/Rindle/extension`. In the Rindle popup, select **Google Chrome**.

## Add Edge

```sh
~/.local/bin/rindle setup --browser edge
```

Open `edge://extensions`, turn on Developer mode, and load the same extension folder. In the popup, choose **Microsoft Edge**.

## Use all three

You can configure all supported Chromium browsers with:

```sh
~/.local/bin/rindle setup --all
```

History queues are kept separately for each browser. Changes pass through Safari history and your existing iCloud account. Rindle doesn't need you to be signed into a Chrome or Edge account.

**A note on compatibility:** Dia and iPhone Safari are the real-device-tested primary experience. Chrome and Edge share the same Chromium history and Native Messaging APIs, but behavior can differ between browser versions.
