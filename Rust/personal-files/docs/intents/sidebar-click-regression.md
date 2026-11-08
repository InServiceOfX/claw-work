# Sidebar click regression — 2026-10-16

Exact report:

> Double clicking directly on the left a directory say Downloads doesn't immediately refresh and display in the current tab the Downloads directory and its contents

This restores approved behavior R2 (sidebar click navigates) rather than
introducing a new product meaning. Read-only intake plan:
`double-clicking-directly-on-the-left-a-directory-d60bfbc7`. No additional
provider-backed intent apply or semantic validation was attempted.

Cause: the sidebar connected navigation to ListBoxRow's action signal, while
mouse activation is emitted by ListBox's row-activated signal. The original
GUI test emitted the row action directly and missed this dispatch difference.

Evidence: changed the test to emit the native list activation signal against
untouched code; it failed, remaining in the previous folder. After connecting
ListBox::row-activated and explicitly enabling single-click activation, it
passed. A temporary Downloads fixture also checks the address/file listing,
current-tab-only navigation, repeated-click refresh and unchanged visit count.
