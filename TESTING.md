# Testing

This document provides a regression testing checklist for the COSMIC Launcher. The checklist provides a starting point for Quality Assurance reviews.

## Checklist

- [ ] Launcher does not flicker or jump when opening (first launch & subsequent launches)
- [ ] Cut text from the launcher, then close it; pasting into an app works
- [ ] All windows on all workspaces appear on launch
- [ ] Choosing an app on another workspace moves workspaces and focus to that app
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
