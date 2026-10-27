# UI and Wording Guidelines

## Compact-from-the-Start Design Rules

Quota Station keeps screens scannable at a glance so the owner sees what a feature does and its current state without reading paragraphs of prose:

1. **One row = name + at most one short line.** Each row or card header states the feature name and at most one short summary line.
2. **Longer explanations go behind `Chi tiết` (`Details`).** Technical mechanics, file paths, command arguments, and attribution notes belong in an expandable `Chi tiết` / `Details` disclosure rather than inline on the row.
3. **Settings blocks rest as one status summary row.** Complex settings blocks rest as a compact status summary row that expands in place when configured or inspected.
4. **One plain status line per feature.** Every feature shows a single plain status line stating when it was last updated or checked, whether it is off, or what failed and why.
5. **One term per idea across Vietnamese and English.** Use plain, everyday wording in both languages, never internal mechanics (`đọc log`, `đọc số`, `switcher`, `thẻ macOS`), and keep product names unchanged (`Quota Station`, `Claude Code`, `Codex`, `Antigravity`, `Antigravity.app`).

## Approved Vietnamese / English Glossary

| Concept | Vietnamese (`vi`) | English (`en`) | Avoid |
| --- | --- | --- | --- |
| **Checking status, updates, or connectivity** | `kiểm tra`, `kiểm tra lại`, `đang kiểm tra…`, `kiểm tra lúc HH:MM`, `lần kiểm tra sau`, `tự động kiểm tra` | `check`, `check again`, `checking…`, `checked at HH:MM`, `next check`, `check automatically` | Abbreviated `kiểm`, `kiểm lại`, `đang kiểm…`, `kiểm lúc HH:MM`, `lần kiểm sau`, `tự kiểm` |
| **Latest data timestamp** | `cập nhật lúc HH:MM`, `cập nhật X trước`, `đang cập nhật…`, `chưa cập nhật` | `updated at HH:MM`, `updated X ago`, `updating…`, `not updated yet` | Internal mechanics: `đọc log lúc`, `đọc lúc`, `số X trước`, `đang đọc số…` / `logs read at`, `read at`, `numbers X old` |
| **Reloading a page or view** | `Làm mới` | `Refresh` | `Đọc lại` / `Read again` |
| **Quota and usage** | `hạn mức`, `dữ liệu hạn mức`, `phần trăm đã dùng` | `quota`, `quota data`, `percent used` | `số`, `không đọc được số`, `chưa có số` / `numbers`, `cannot read the numbers`, `share used` |
| **Automatic account switching** | `tự động chuyển` (or `bộ tự chuyển` for the background job) | `auto-switch` | Raw English `switcher` in Vietnamese copy; `Asking the switcher…` |
| **Pausing an account from rotation** | `Tạm dừng tự chuyển…`, `Đưa lại vào tự chuyển`, `tạm dừng đến …` | `Pause auto-switch…`, `Resume auto-switch`, `paused until …` | Colloquial `Cho nghỉ khỏi tự chuyển…`, `Cho vào lại tự chuyển`, `đang nghỉ` / `Leave out of auto-switch…`, `resting` |
| **Applying a configuration set** | `Áp dụng`, `Áp dụng bộ`, `Áp dụng…`, `chưa áp dụng`, `vừa áp dụng` | `Apply`, `Apply set`, `Apply…`, `not applied yet`, `applied just now` | Clipped `Áp`, `Áp bộ`, `Áp…`, `chưa áp`, `vừa áp` |
| **System notifications** | `thông báo macOS`, `thông báo` | `macOS notification`, `notification` | Internal term `thẻ macOS`, `thẻ` / `macOS card`, `card` |
| **Cancel, discard, and dismiss actions** | `Huỷ` (Cancel), `Bỏ thay đổi` (Discard changes), `Bỏ qua` (Dismiss / Ignore) | `Cancel`, `Discard changes`, `Dismiss` / `Ignore` | Colloquial `Thôi`; `Huỷ thay đổi` when paired with `Huỷ` |
| **Expanding details** | `Chi tiết` | `Details` | Inline multi-sentence explanations on primary rows |
