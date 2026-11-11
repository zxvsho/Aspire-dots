# aspire-bg reference (doc-1: architecture + flows)

## Flow
- Startup: no full walk; sample 20 in worker; paint empty grid then update.
- Search: debounce 200ms; worker builds index on first query; filters first 20; GTK applies.
- Wallhaven: Stack view; search worker HTTP + thumbs; drainer updates grid; More appends; pick downloads original and applies.
- Options popover: subfolders, hidden, sort; Wallhaven sort/purity.

