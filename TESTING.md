# Testing

This document provides a regression testing checklist for the COSMIC Launcher. The checklist provides a starting point for Quality Assurance reviews.

## Checklist

- [ ] Launcher does not flicker or jump when opening (first launch & subsequent launches)
- [ ] Cut text from the launcher, then close it; pasting into an app works
- [ ] All windows on all workspaces appear on launch
- [ ] Choosing an app on another workspace moves workspaces and focus to that app
- [ ] Alt+Tab with "All workspaces" lists windows from every workspace
- [ ] Alt+Tab with "Current workspace on all displays" lists only windows on the workspace shown on each display
- [ ] Alt+Tab with "Current workspace on this display only" lists only the workspace of the display it opens on (the one under the pointer), even with keyboard focus on another display
- [ ] Changing the window switcher scope in Settings applies on the next Alt+Tab without restarting
- [ ] Alt+` lists only the windows of the focused app (e.g. only Firefox windows, not terminals); releasing Alt activates the selection
- [ ] Shift+Alt+` moves backward through the focused app's windows
- [ ] Alt+` follows each of the three window switcher scopes in Settings
- [ ] Alt+` with only one window of the focused app (in scope) shows nothing
- [ ] Alt+` on an empty workspace (no focused window) shows nothing
- [ ] ` and dead-key accents (e.g. on US alt-intl) still type normally, also with AltGr
- [ ] Alt+Tab is unchanged and still lists every app's windows
- [ ] Launching an application works
- [ ] Typing text and then removing it will re-show the open windows
- [ ] Search works for applications and windows
- [ ] Open windows are sorted above applications (e.g. "firefox")
- [ ] Search works for COSMIC settings panels
- [ ] t: executes a command in a terminal
- [ ] : executes a command in sh
- [ ] = calculates an equation
- [ ] Search results are as expected:
    - `cal` returns LibreOffice Calc first
    - `pops` returns Popsicle first
    - `shop` returns the COSMIC Store first
