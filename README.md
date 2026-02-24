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

## Contributing

PRs and issues are welcome. For bigger changes, please open an issue first. Contribution rules are in `CONTRIBUTING.md` (including a note to avoid adding contributor-name lists to the README).

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

## CLI Remote Control (HTTP)

You can control the timer from terminal using the standalone `pp` binary (Rust, no npm runtime required).

1. In app Settings, enable "iPhone Remote Control (LAN)".
2. Copy the remote token from Settings.
3. Install/update the app using the regular installer.
4. On first app launch, `pp` is auto-installed to:
   - Windows: `%USERPROFILE%\.local\bin\pp.exe`
   - macOS/Linux: `~/.local/bin/pp`
5. The app also auto-registers that directory in your user `PATH`.
6. Open a new terminal session.
7. Save token and port once:
   - `pp token YOUR_TOKEN`
   - `pp port 48484`
8. Use one of:
   - `pp status`
   - `pp start`
   - `pp skip`
   - `pp stop`

Manual build for local development:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --bin pp --release
```

Direct run without PATH setup:

```bash
# Windows (PowerShell)
.\src-tauri\target\release\pp.exe help

# macOS/Linux
./src-tauri/target/release/pp help
```

Config is stored in:

- Windows: `%USERPROFILE%\.pomodoro-pulse-pp.json`
- macOS/Linux: `~/.pomodoro-pulse-pp.json`
