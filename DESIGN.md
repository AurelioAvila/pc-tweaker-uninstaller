# PC Tweaker Uninstaller — desktop design

Preserve the official application icon and existing selectable themes. This is an inventory and removal workspace: installed software, its size and removal evidence come first.

- Inter for UI; 28px workspace title, 20px section headings, 14px rows, 12px metadata. Readable muted text, no decorative slogans.
- Existing theme tokens drive all surfaces and accent states. One accent for primary actions; green/amber/red only communicate actual status.
- Spacious workspace heading, compact summary strip, search always visible. Advanced filters are disclosed on demand. Columns must not overlap at the 940px minimum window width.
- Native installed-app icons, framed at 44px; crisp duotone SVG fallback and functional icons. Subtle hover lift and short transitions, disabled under reduced motion.
- Profile contains clearly labeled expandable account, plan, language and theme sections. Escape/outside click closes it; keyboard focus stays usable.
- Show actual operation progress; never invent reclaimed space, unused apps, scan problems, or percentages. Unknown sizes remain unknown.
- Update notification offers the signed updater, a later action, actual download progress and retry errors. Manual checks available in profile; never update during a removal flow.
- Removal confirmation, backend validation, Pro entitlements, restore points and recovery limitations remain authoritative.

Rollback: this branch started at c173e5e (origin/master, 0.11.3). The original desktop checkout remains untouched. Review the diff before using git restore on the changed files.
