# Security review — 2026-10-30

Full-repo review, prompted by a corporate AV quarantining `astur.exe` as
"unwanted software" (PUA). Two parts: real vulnerabilities, then the traits
that most likely trip AV/EDR heuristics.

## Vulnerabilities

| # | Severity | File | Lines | Vulnerability | Confidence |
|---|----------|------|-------|---------------|------------|
| 1 | MEDIUM | crates/astur/src/main.rs, crates/astur-config/src/lib.rs | launch / run_custom / pipe DACL | Elevated Astur runs commands from a user-writable config | 7/10 |
| 2 | MEDIUM | .github/workflows/release.yml | 20, 31 | Unpinned release toolchain (supply chain) | 7/10 |

### 1. Elevated Astur runs commands from a user-writable config

The README suggests running as admin to manage elevated windows. The elevated
process still reads `%USERPROFILE%\.astur\astur.conf` / `navbar.conf` (or
`ASTUR_CONFIG`), which any medium-integrity process of the same user can write,
and reloads them without a restart. Keys that run commands: `custom_command`
(`cmd /S /C` on a timer), `terminal`, `browser`, `scratchpad_command`,
`extra_hotkeys` / bar `launch` / `command` actions, `launcher_entries` `cmd:`
targets. The named pipe DACL grants by user SID with no integrity label, so with
`ipc_enabled` + `ipc_allow_launch` a medium-integrity client can drive an
elevated Astur's `launch`.

Exploit: unelevated malware appends `custom_command = <payload>` to
`navbar.conf`; Astur runs it elevated, no UAC prompt. Secondary: the elevated
process writes/renames/deletes `astur.log`, `rescue.lst`, `state.conf` in that
user-writable folder (junction/symlink file-write or file-delete primitive).

Fix: when `process_elevated()`, either ignore command-running keys, load config
from an admin-only location, or require the config files' ACL to deny
medium-integrity writes. Add a mandatory label `S:(ML;;NW;;;HI)` to the pipe's
security descriptor when elevated.

### 2. Unpinned release toolchain

`release.yml` publishes with `contents: write`, using
`dtolnay/rust-toolchain@stable` (mutable ref) and an unpinned
`choco install innosetup`. Either upstream can alter the shipped binaries;
releases have no signature or checksum to expose it.

Fix: pin the action to a commit SHA, pin Inno Setup's version (or download and
verify its SHA-256), publish SHA-256 sums with each release.

## AV / PUA heuristic triggers (most likely first)

1. **Unsigned, no VERSIONINFO, no manifest.** The resource scripts carried only
   an icon. Unknown publisher + new hash each release + item 2 = classic ML PUA
   verdict. *Addressed:* `crates/astur/resources.rs` now embeds a manifest
   (`asInvoker`, Windows 10/11 `supportedOS`) and VERSIONINFO into both exes.
   *Still to do:* code-sign the exes and installer (Azure Trusted Signing or an
   OV/EV cert, `SignTool` in Inno Setup); for a corporate AV, submit to the
   Microsoft WDSI portal as a false positive or ask IT to allow-list by
   hash/signer.
2. **Keylogger-shaped imports.** `windows_subsystem` GUI, `WH_KEYBOARD_LL` +
   `WH_MOUSE_LL`, key swallowing, `ToUnicode`, `GetAsyncKeyState`. Behaviour is
   benign (`ToUnicode` only while the launcher is open, modifiers only polled),
   but static scanners see the combination. Optional: feed launcher text via
   `WM_CHAR` on a focused edit instead of `ToUnicode` on hook data.
3. **Self-reinstalling hooks.** Hook watchdog re-runs `SetWindowsHookExW` when
   the hooks go quiet. Optional: cap re-arms per session, log each one.
4. **Always-on clipboard listener.** `AddClipboardFormatListener` ran even with
   `clipboard_history = false` (the default). *Addressed:* the listener is now
   registered only while `clipboard_history` is on and follows hot-reload
   (`clipboard_listener_sync`); turning history off also clears it from memory.
5. **Screen capture of other apps.** `PrintWindow(PW_RENDERFULLCONTENT)`,
   `BitBlt`, DWM thumbnails. Never written to disk. Inherent to the animations.
6. **Hidden `cmd.exe` children.** Every launch went through
   `cmd /C start` with `CREATE_NO_WINDOW`. *Addressed:* plain
   `program args` / paths / URIs now go straight to `ShellExecuteExW`; shell
   syntax (`& | < > ^`) or a ShellExecute failure still falls back to cmd. The
   custom bar widget still uses `cmd /S /C` (opt-in, user-written command).
7. **Synthetic input.** `SendInput` (synthetic Alt, Alt+Tab, Ctrl+V paste), the
   `0xE8` mask key (same trick AutoHotkey uses), `AttachThreadInput`, system
   foreground-lock-timeout change. Inherent; optional: `0xFF` instead of
   `0xE8`, tag injected input via `dwExtraInfo`.
8. **Hiding / off-screen parking of foreign windows.** Inherent.
9. **Shutdown privilege imports.** `AdjustTokenPrivileges(SeShutdownPrivilege)`,
   `ExitWindowsEx`, `SetSuspendState`. Only used on a power action; optional:
   `InitiateShutdownW` / `shutdown.exe`, or resolve lazily.
10. **Server code compiled in.** Loopback `TcpListener` (YASB) and a named-pipe
    server, both off by default. Optional: Cargo features.
11. **Persistent wallpaper change** (`SPI_SETDESKWALLPAPER` +
    `SPIF_UPDATEINIFILE`), only with per-workspace wallpapers. Optional:
    `IDesktopWallpaper::SetWallpaper`.
12. Minor: reads other processes' token integrity, `TerminateProcess` on self,
    `AttachConsole`, opt-in Startup-folder shortcut in the installer. No packer;
    release profile is normal.

## Verified safe

- No keystroke content logged or persisted. Launcher query never logged and
  cleared on close; `launcher-mru.conf` holds launched paths only; clipboard
  history is memory-only and honours password-manager exclusion formats.
- No `CreateRemoteThread`, `WriteProcessMemory`, `VirtualAlloc`, injection,
  anti-debug or `NtQuery*`.
- No `Run` key, scheduled task, self-elevation or UAC prompt.
- No outbound network code or downloads.
- YASB WebSocket: off by default, `127.0.0.1` only, rejects browser `Origin`,
  workspace query/subscribe/focus only.
- IPC pipe: off by default, owner-only DACL, `PIPE_REJECT_REMOTE_CLIENTS`,
  launching needs a second opt-in (`ipc_allow_launch`), name sanitised.
- Windows Search query strips user quotes and escapes the scope; web-search
  query is URL-encoded.
- Elevated Astur's UIPI filter allows only `TaskbarCreated` from below.
- Settings GUI: registry read-only, atomic config writes, newlines rejected.
