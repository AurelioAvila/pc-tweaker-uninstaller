PC Tweaker Uninstaller 0.11.0 closes a hole in leftover cleanup: a program whose recorded install location pointed at its containing folder rather than its own could have had that whole folder offered as a leftover and moved to the Recycle Bin. The only guard was a path-component count, and a count cannot tell a real install directory from a Windows well-known folder. Every well-known root, the user profile and its document folders, and any bare drive root are now refused outright.

- The window no longer freezes at startup or during a Store removal. Six commands that ran on the main thread, including the two that fire together on launch, now run off it.
- The expandable row opens with Enter or Space and shows a focus ring. The confidence reasons, install folder, registry entry and exact uninstall command were unreachable without a mouse.
- An uninstall running longer than twenty seconds explains that the program's own uninstaller has probably opened a window behind this one, in all five languages.
- A local diagnostic log records what the app did and any crash, beside the removal ledger. "Open log folder" in the ledger dialog reaches it. Nothing is uploaded and there is no crash-reporting service.

Windows application and installer signatures are verified before distribution. Code signing identifies the publisher; Windows may still show reputation warnings.
