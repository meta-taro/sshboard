<!--
  English version. **Paired with `.github/release-note-alpha.ja.md`.**
  **Do not edit one without the other** — this file is rewritten every release,
  which is exactly where the two will drift apart.
-->

## Which file do I want?

| Your OS | Take this |
|---|---|
| **Windows** | **`sshboard_<version>_x64-setup.exe`** (installer) |
| Windows (prefer MSI) | `sshboard_<version>_x64_en-US.msi` |
| **macOS** (Apple Silicon) | **`sshboard.app.tar.gz`** |

**The `.sig` files are signatures for the auto-updater.** You don't need to download them.

> **`sshboard.app.tar.gz` is for macOS.** A macOS `.app` is a directory with contents inside,
> so **on Windows it just looks like a folder.** It will not run there.

## This is an alpha

**It is now in use against a production server.** Issues #8–#25 came out of that.
**There is still no record of a job being carried through from connect to finish.**

**0.1.12 opened the path to operations that need root** (D48 / Issue #19).

You can set `become` per connection:

```toml
[[connections]]
become = "ask"        # a human types it at the time. **Never stored**
# become = "sudoers"  # sudoers.d already grants it
# omit it and nothing is elevated (default — unchanged from every prior version)
```

`ask` prompts for the password on the approval screen and **feeds it to `sudo -S` over stdin.**

- **Not stored.** Not on disk, not in the OS credential store
- **Never on the command line** (it does not show up in `ps`)
- **Never on screen and never in the log.** What you see stops at `$ sudo -S -p '' …`
- **One approval, one run.** Approvals expire after 5 minutes

**Until then, the AI side had no route to read anything root-owned at all.**
`operations.toml` (0.1.11) decides **which commands may run**, but it never decided
**how privilege is obtained.**

**The same release fixed `max_per_hour`, which had never once taken effect.**
The per-hour ceiling added in 0.1.11 **counted into the same container as the approval tokens**,
so it **never stopped anything.** Advertising a limit that isn't there was the worst shape of all.

**Added `about_sshboard` for AI agents** (D49).
It answers what this tool is, what is and isn't allowed, and **what changed in each version.**

**0.1.11 made state-changing operations runnable under human approval** (D45).
All the AI can pass is **an id a human wrote into `operations.toml`.**
The first attempt is always refused, and **the screen shows exactly what would run.**

> This section **is rewritten every release.**
> "It's an alpha" doesn't change, but **how far it has actually been verified changes every time.**
> Freezing this text and leaving it would ship claims that no longer match reality (Issue #6).
> **Every version's changes are in `CHANGELOG.md`.**

### ⚠️ Not code-signed. You will get a warning when you open it

| How you got it | What you'll see | What to do |
|---|---|---|
| Windows, **downloaded in a browser and double-clicked** | SmartScreen blocks it | "More info" → "Run anyway" |
| Windows, **`gh run download` and friends** | **Nothing blocks it** (measured — Issue #3) | Just run it |
| macOS | Gatekeeper blocks it (**unverified**) | Right-click → "Open" |

The split on Windows comes down to **whether Mark of the Web is attached.**
SmartScreen's app-reputation check keys off that, so on routes that don't attach it
(`gh`, CI, scripts) no warning appears at all.

**This is a tool that handles keys.** We are shipping it knowing that the first thing it teaches
you is "dismiss the warning." **Signing goes in once someone is actually hurt by its absence** (D12).
**If you hit a warning, or couldn't tell what to do, please file an issue.** That's what decides
whether we buy a certificate.

Windows files extracted from a zip carry Mark of the Web. Right-click → Properties →
**tick "Unblock"** to quiet it down.

### What's attached

| File | What it is |
|---|---|
| `sshboard_<version>_x64-setup.exe` | Windows installer (NSIS). **This is the straightforward one** |
| `sshboard_<version>_x64_en-US.msi` | Windows installer (MSI) |
| `sshboard.app.tar.gz` | macOS. Extract it and move it to `/Applications` |

Windows needs **WebView2**. Windows 11 and recent 10 have it. If it's missing the installer
fetches it, so **the first install needs an internet connection.**

Where connection settings live:

- Windows: `%APPDATA%\sshboard\sshboard\config\connections.toml`
- macOS: `~/Library/Application Support/dev.sshboard.sshboard/connections.toml`

### Building from source

**Point it at your own test server once before you point it at a production one.**

```sh
pnpm install
sh tools/test-server/up.sh
pnpm --filter desktop tauri dev
```

### Things worth knowing up front

- **The MCP port is fixed at `22022`,** and the token is reused, so `claude mcp add` is a one-time step.
  On a collision it **does not quietly move to another port** — it says so on screen
  (`SSHBOARD_MCP_PORT` moves it)
- **The AI can only write beneath directories a human listed for that connection.**
  **The default is empty — it cannot write a single byte**
- **`run_readonly`'s allow-list is also empty by default.** Until a human writes `readonly.toml`,
  not one command will run
- **The terminal is shared between the human and the AI.** While the AI holds it your input is
  locked out, and **[Stop] always works**
- **It is in use on a real Windows machine.** Issues #8–#25 came from there.
  **ssh-agent (named pipe / Pageant) is unverified** — it is currently used with passwords and key files

### Reporting problems

**Whoever used it, right after using it, in their own words** — please write the issue yourself.
The moment it gets summarised, the part that actually hurt falls out.
