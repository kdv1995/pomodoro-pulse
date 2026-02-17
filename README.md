# Pomodoro Pulse

Local-first macOS Pomodoro tracker with analytics, menu bar controls, and DMG packaging.

## Features

- Focus timer with 25/5 defaults and long break every 4 cycles
- Pause, resume, and skip controls from app window and menu bar
- Optional iPhone remote control on your local Wi‑Fi (simple web page)
- SQLite persistence (no auth, no cloud)
- Projects + tags for focus sessions
- Analytics dashboard:
  - total focus time
  - completed pomodoros
  - streak days
  - interruptions
  - daily trend chart
  - session history
- Local export to CSV and JSON
- macOS notifications and optional sound alerts

## Tech Stack

- Tauri 2 (Rust backend)
- React + TypeScript + Vite
- Recharts + TanStack Query
- SQLite via `rusqlite` (bundled SQLite)

## Development

```bash
npm install
npm run tauri dev
```

## Build DMG

```bash
npm run tauri build
```

DMG output:

`src-tauri/target/release/bundle/dmg/*.dmg`

## Releases (Recommended)

For versioned downloads (and older versions), publish a GitHub Release with the DMG attached.

## Data Storage

Database location:

`~/Library/Application Support/com.user.pomodoro-pulse/pomodoro.db`

## Notes

- This is single-user local software with no authentication.
- Updates are manual: download and install a new DMG release.

## Support / Donate

This is an open source project. If you find it useful and want to support development, donations are optional and appreciated:

```text
https://donatello.to/codebezmezh
```

## Links

Landing page:

```text
https://landing-pomodoro.vercel.app/
```

## Community / Vibe Coding

If you want to join and develop this open source project with me through vibe coding, you can connect via my socials below.

### 💡 Для кого цей канал

- якщо хочеш зайти в Web3
- якщо використовуєш або хочеш використовувати AI в розробці
- якщо будуєш продукти, а не просто "вчиш синтаксис"
- якщо хочеш мислити як інженер, а не як туторiал-вотчер

### 🔗 Лiнки

- LinkedIn: https://www.linkedin.com/in/danyil-ku...
- Telegram (код, iдеї та життя без меж): https://t.me/codebezmezh
- TikTok (короткi формати й хардкорнi iнсайти): https://www.tiktok.com/@codebezmezh?_...
- Discord (спiльнота): https://discord.gg/uQ6QwQsa
- Twitch (лайв-кодинг i стрiми): https://www.twitch.tv/codebezmezh
- Email (стрiми, колаби, iдеї): tribeofdanel@gmail.com

### 📌 Пiдписуйся, якщо тобi цiкаво

- як створити Web3 токен або NFT-маркетплейс
- як використовувати AI для розробникiв
- як стати Fullstack-iнженером без меж

Пiдписка + лайк = бiльше шипiнгу 🚀

## iPhone Remote Control (LAN)

You can optionally control the timer from your iPhone using a local web page served by the desktop app.

1. Open the app -> Settings -> enable "iPhone Remote Control (LAN)" -> Save.
2. Find your Mac's Wi‑Fi IP: `ipconfig getifaddr en0`
3. On iPhone Safari open: `http://YOUR_MAC_IP:PORT/?token=TOKEN`

Your Mac and iPhone must be on the same Wi‑Fi, and the app must be running.
