export function isMac(): boolean {
  return typeof navigator !== "undefined" && /Mac/.test(navigator.platform);
}

export function isWindows(): boolean {
  return typeof navigator !== "undefined" && /Win/.test(navigator.platform);
}

export function isLinux(): boolean {
  return typeof navigator !== "undefined" && /Linux/.test(navigator.platform);
}

/** Human-readable OS name for tooltips / hints. */
export function osName(): string {
  if (isMac()) return "macOS";
  if (isWindows()) return "Windows";
  return "Linux";
}

/**
 * Returns "System Settings → Sound → Input" on macOS,
 * "Sound Settings → Input" on Windows, or a generic fallback.
 */
export function audioInputSettingsPath(): string {
  if (isMac()) return "System Settings → Sound → Input";
  if (isWindows()) return "Sound Settings → Input";
  return "your system audio settings";
}

/** Human-readable name for the system credential store.
 * Returns values that fit grammatically after "in the ".
 */
export function keychainName(): string {
  if (isMac()) return "macOS Keychain";
  if (isWindows()) return "Windows Credential Manager";
  return "system keyring";
}

/**
 * Human-readable name for the platform file manager.
 * "Reveal in Finder" on macOS, "Show in File Explorer" on Windows,
 * "Show in file manager" on Linux.
 */
export function revealNoun(): string {
  if (isMac()) return "Finder";
  if (isWindows()) return "File Explorer";
  return "file manager";
}

/**
 * Human-readable device name used in "stays on your …" / "runs on your …" copy.
 */
export function thisDevice(): string {
  if (isMac()) return "this Mac";
  if (isWindows()) return "this PC";
  return "this device";
}

/**
 * Possessive form: "your Mac" / "your PC" / "your device".
 */
export function yourDevice(): string {
  if (isMac()) return "your Mac";
  if (isWindows()) return "your PC";
  return "your device";
}
